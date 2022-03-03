//! Internal details for the [`TreeLayer`] type.
//!
//! Implementation details in this module aren't important, unless you're
//! implementing your own [`Processor`] or [`Formatter`].
//!
//! # Quick reference
//!
//! Trace data is stored in the layer as a tree, where spans represent internal
//! nodes and events represent leafs.
//!
//! * [`Tree`]: A node in the trace tree.
//! * [`TreeAttrs`]: Common data used by spans and events, like a [`Uuid`] if
//! the `uuid` feature is enabled, a timestamp if the `timestamp` feature is
//! enabled, and a [`Level`].
//! * [`TreeKind`]: Contains either a [`TreeSpan`] or a [`TreeEvent`].
//! * [`TreeSpan`]: Data unique to span traces, including durations and other
//! [`Tree`] nodes.
//! * [`TreeEvent`]: Data unique to event traces, like tags.
//!
//! [`Formatter`]: crate::formatter::Formatter

use crate::builder::MakeStdout;
use crate::formatter::{format_immediate, Pretty};
use crate::processor::{Printer, Processor};
use crate::tag::{NoTag, Tag, TagData, TagParser};
use crate::{cfg_chrono, cfg_json, cfg_uuid, fail};
#[cfg(feature = "smallvec")]
use smallvec::SmallVec;
use std::time::{Duration, Instant};
use std::{borrow::Cow, fmt};
use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::{Event, Id, Level, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};
cfg_json! {
    use crate::ser;
    use serde::Serialize;
}
cfg_chrono! {
    use chrono::{DateTime, Utc};
}
cfg_uuid! {
    use uuid::Uuid;
    const DEFAULT_EVENT_UUID: Uuid = Uuid::nil();
}

#[cfg(feature = "smallvec")]
pub(crate) type Fields = SmallVec<[KeyValue; 3]>;
#[cfg(not(feature = "smallvec"))]
pub(crate) type Fields = Vec<KeyValue>;

#[doc(hidden)]
#[derive(Debug)]
pub struct KeyValue {
    pub key: &'static str,
    pub value: String,
}

pub(crate) const TAG_KEY: &str = "__event_tag";

/// A [`Layer`] that tracks and maintains contextual coherence.
///
/// See the [top-level documentation][crate] for details on how to use.
#[derive(Debug)]
pub struct TreeLayer<P> {
    processor: P,
    tag_parser: TagParser,
}

impl<P: Processor> TreeLayer<P> {
    pub fn new(processor: P) -> Self {
        TreeLayer {
            processor,
            tag_parser: NoTag::from_field,
        }
    }

    /// Set the accepted [`Tag`] type of the `TreeLayer`.
    pub fn tag<T: Tag>(mut self) -> Self {
        self.tag_parser = T::from_field;
        self
    }

    // Placeholder API for when we deprecate the `Tag` trait
    // and just use a function.
    pub fn set_tag(mut self, tag_parser: TagParser) -> Self {
        self.tag_parser = tag_parser;
        self
    }

    fn parse_event(&self, event: &Event) -> (TreeAttrs, TreeEvent, bool) {
        struct EventVisitor {
            immediate: bool,
            tag: Option<TagData>,
            message: Cow<'static, str>,
            fields: Fields,
            tag_parser: TagParser,
        }

        impl EventVisitor {
            fn new(tag_parser: TagParser) -> Self {
                EventVisitor {
                    immediate: false,
                    tag: None,
                    message: Cow::from("<no message>"),
                    fields: Fields::new(),
                    tag_parser,
                }
            }
        }

        impl Visit for EventVisitor {
            fn record_bool(&mut self, field: &Field, value: bool) {
                match field.name() {
                    "immediate" => self.immediate = value,
                    _ => self.record_debug(field, &value),
                }
            }

            fn record_u64(&mut self, field: &Field, value: u64) {
                match field.name() {
                    TAG_KEY => {
                        if self.tag.is_some() {
                            fail::multiple_tags_on_event();
                        }
                        self.tag = Some((self.tag_parser)(value));
                    }
                    _ => self.record_debug(field, &value),
                }
            }

            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                let value = format!("{:?}", value);
                match field.name() {
                    // Only the first "message" is the message
                    "message" if matches!(self.message, Cow::Borrowed(_)) => {
                        self.message = Cow::from(value)
                    }
                    key => self.fields.push(KeyValue { key, value }),
                }
            }
        }

        let mut visitor = EventVisitor::new(self.tag_parser);

        event.record(&mut visitor);

        let tree_event = TreeEvent {
            tag: visitor.tag,
            message: visitor.message,
            fields: visitor.fields,
        };

        let tree_attrs = TreeAttrs {
            #[cfg(feature = "uuid")]
            uuid: DEFAULT_EVENT_UUID,
            #[cfg(feature = "chrono")]
            timestamp: Utc::now(),
            level: *event.metadata().level(),
        };

        (tree_attrs, tree_event, visitor.immediate)
    }
}

impl Default for TreeLayer<Printer<Pretty, MakeStdout>> {
    fn default() -> Self {
        Self {
            processor: Printer::default(),
            tag_parser: NoTag::from_field,
        }
    }
}

/// A node of a log tree.
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Tree {
    /// Shared fields associated with both spans and events.
    #[cfg_attr(feature = "json", serde(flatten))]
    pub(crate) attrs: TreeAttrs,
    /// Fields specific to either a span or an event.
    pub(crate) kind: TreeKind,
}

impl Tree {
    fn new(attrs: TreeAttrs, kind: impl Into<TreeKind>) -> Self {
        Tree {
            attrs,
            kind: kind.into(),
        }
    }

    cfg_uuid! {
        /// Get the [`Uuid`].
        pub fn uuid(&self) -> Uuid {
            self.attrs.uuid()
        }
    }

    cfg_chrono! {
        /// Get the [`DateTime`].
        pub fn timestamp(&self) -> DateTime<Utc> {
            self.attrs.timestamp()
        }
    }

    /// Returns the [`Level`].
    pub fn level(&self) -> Level {
        self.attrs.level()
    }

    /// Returns a reference to the [`TreeEvent`], if the tree is an event.
    pub fn event(&self) -> Result<&TreeEvent, KindError> {
        match &self.kind {
            TreeKind::Event(event) => Ok(event),
            TreeKind::Span(_) => Err(KindError { is_event: false }),
        }
    }

    /// Returns a reference to the [`TreeSpan`], if the tree is a span.
    pub fn span(&self) -> Result<&TreeSpan, KindError> {
        match &self.kind {
            TreeKind::Event(_) => Err(KindError { is_event: true }),
            TreeKind::Span(span) => Ok(span),
        }
    }

    /// Returns the [`TreeEvent`], if the tree is an event.
    pub fn into_event(self) -> Result<TreeEvent, KindError> {
        match self.kind {
            TreeKind::Event(event) => Ok(event),
            TreeKind::Span(_) => Err(KindError { is_event: false }),
        }
    }

    /// Returns the [`TreeSpan`], if the tree is a span.
    pub fn into_span(self) -> Result<TreeSpan, KindError> {
        match self.kind {
            TreeKind::Event(_) => Err(KindError { is_event: true }),
            TreeKind::Span(span) => Ok(span),
        }
    }
}

/// Error returned by [`Tree::event`] and [`Tree::span`].
#[derive(Debug)]
pub struct KindError {
    is_event: bool,
}

impl fmt::Display for KindError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_event {
            write!(f, "Found event")
        } else {
            write!(f, "Found span")
        }
    }
}

impl std::error::Error for KindError {}

#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub(crate) struct TreeAttrs {
    #[cfg(feature = "uuid")]
    uuid: Uuid,
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "json", serde(serialize_with = "ser::timestamp"))]
    timestamp: DateTime<Utc>,
    #[cfg_attr(feature = "json", serde(serialize_with = "ser::level"))]
    level: Level,
}

impl TreeAttrs {
    cfg_uuid! {
        pub fn uuid(&self) -> Uuid {
            self.uuid
        }
    }

    cfg_chrono! {
        pub fn timestamp(&self) -> DateTime<Utc> {
            self.timestamp
        }
    }

    pub fn level(&self) -> Level {
        self.level
    }
}

/// The kind of log, either a [`TreeEvent`] or a [`TreeSpan`].
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub(crate) enum TreeKind {
    Event(TreeEvent),
    Span(TreeSpan),
}

/// Information unique to logged events.
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct TreeEvent {
    /// An optional tag that the event was collected with.
    tag: Option<TagData>,
    /// The message associated with the event.
    message: Cow<'static, str>,
    /// Key-value data.
    #[cfg_attr(feature = "json", serde(serialize_with = "ser::fields"))]
    fields: Fields,
}

impl TreeEvent {
    /// Get the event's message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the event's tag message, if there was a tag.
    pub fn tag(&self) -> Option<TagData> {
        self.tag
    }

    /// Gets a slice of the event's key-value pairs.
    pub fn fields(&self) -> &[KeyValue] {
        &self.fields
    }
}

/// Information unique to logged spans.
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct TreeSpan {
    name: &'static str,
    #[cfg_attr(
        feature = "json",
        serde(rename = "nanos_total", serialize_with = "ser::nanos")
    )]
    total_duration: Duration,
    #[cfg_attr(
        feature = "json",
        serde(rename = "nanos_nested", serialize_with = "ser::nanos")
    )]
    inner_duration: Duration,
    pub(crate) children: Vec<Tree>,
}

impl TreeSpan {
    /// Returns the name of the span.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns the total duration the span was open for.
    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    /// Returns the duration the span spent inside inner spans.
    pub fn inner_duration(&self) -> Duration {
        self.inner_duration
    }

    /// Returns the duration the span spent while not inside inner spans.
    pub fn base_duration(&self) -> Duration {
        self.total_duration - self.inner_duration
    }

    /// Returns the log trees within the span.
    pub fn children(&self) -> &[Tree] {
        &self.children
    }

    /// Returns `true` is the span contains no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl From<TreeEvent> for TreeKind {
    fn from(event: TreeEvent) -> Self {
        TreeKind::Event(event)
    }
}

impl From<TreeSpan> for TreeKind {
    fn from(span: TreeSpan) -> Self {
        TreeKind::Span(span)
    }
}

pub(crate) struct TreeSpanOpened {
    attrs: TreeAttrs,
    span: TreeSpan,
    start: Instant,
}

impl TreeSpanOpened {
    fn open<S>(attrs: &Attributes, ctx: &Context<S>) -> Self
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        #[cfg(not(feature = "uuid"))]
        let _ = ctx;

        struct SpanVisitor {
            #[cfg(feature = "uuid")]
            uuid_lsb: Option<u64>,
            #[cfg(feature = "uuid")]
            uuid_msb: Option<u64>,
        }

        impl SpanVisitor {
            fn new() -> Self {
                SpanVisitor {
                    #[cfg(feature = "uuid")]
                    uuid_lsb: None,
                    #[cfg(feature = "uuid")]
                    uuid_msb: None,
                }
            }

            cfg_uuid! {
                fn get_uuid(&self) -> Option<Uuid> {
                    match (self.uuid_msb, self.uuid_lsb) {
                        (Some(msb), Some(lsb)) => Some(crate::uuid::from_u64_pair(msb, lsb)),
                        (None, None) => None,
                        _ => {
                            // This is the case where only half of a uuid
                            // was passed in. Should we say anything?
                            // For now, no.
                            None
                        }
                    }
                }
            }
        }

        impl Visit for SpanVisitor {
            fn record_u64(&mut self, field: &Field, value: u64) {
                match field.name() {
                    #[cfg(feature = "uuid")]
                    "__uuid_lsb" => self.uuid_lsb = Some(value),
                    #[cfg(feature = "uuid")]
                    "__uuid_msb" => self.uuid_msb = Some(value),
                    _ => self.record_debug(field, &value),
                }
            }

            fn record_debug(&mut self, _field: &Field, _value: &dyn fmt::Debug) {
                // do nothing
            }
        }

        let mut visitor = SpanVisitor::new();

        attrs.record(&mut visitor);

        #[cfg(feature = "uuid")]
        let uuid = match visitor.get_uuid() {
            Some(uuid) => uuid,
            None => match ctx.lookup_current() {
                Some(parent) => parent
                    .extensions()
                    .get::<TreeSpanOpened>()
                    .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
                    .uuid(),
                None => Uuid::new_v4(),
            },
        };

        TreeSpanOpened {
            attrs: TreeAttrs {
                #[cfg(feature = "chrono")]
                timestamp: Utc::now(),
                #[cfg(feature = "uuid")]
                uuid,
                level: *attrs.metadata().level(),
            },
            span: TreeSpan {
                name: attrs.metadata().name(),
                children: Vec::new(),
                inner_duration: Duration::ZERO,
                total_duration: Duration::ZERO,
            },
            start: Instant::now(),
        }
    }

    fn enter(&mut self) {
        self.start = Instant::now();
    }

    fn exit(&mut self) {
        self.span.total_duration += self.start.elapsed();
    }

    fn close(self) -> (TreeAttrs, TreeSpan) {
        (self.attrs, self.span)
    }

    fn log_event(&mut self, attrs: TreeAttrs, event: TreeEvent) {
        #[cfg(feature = "uuid")]
        let attrs = TreeAttrs {
            uuid: self.uuid(),
            ..attrs
        };

        self.span.children.push(Tree::new(attrs, event));
    }

    fn log_span(&mut self, attrs: TreeAttrs, span: TreeSpan) {
        self.span.inner_duration += span.total_duration;
        self.span.children.push(Tree::new(attrs, span));
    }

    cfg_uuid! {
        pub fn uuid(&self) -> Uuid {
            self.attrs.uuid()
        }
    }
}

impl<P, S> Layer<S> for TreeLayer<P>
where
    P: Processor,
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        let span = ctx.span(id).unwrap_or_else(fail::span_not_in_context);

        let opened = TreeSpanOpened::open(attrs, &ctx);

        let mut extensions = span.extensions_mut();

        extensions.insert(opened);
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        let (tree_attrs, tree_event, immediate) = self.parse_event(event);

        if immediate {
            #[allow(clippy::expect_used)]
            format_immediate(&tree_attrs, &tree_event, ctx.event_span(event))
                .expect("formatting immediate event failed");
        }

        match ctx.event_span(event) {
            Some(parent) => parent
                .extensions_mut()
                .get_mut::<TreeSpanOpened>()
                .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
                .log_event(tree_attrs, tree_event),
            None => self
                .processor
                .process(Tree::new(tree_attrs, tree_event))
                .unwrap_or_else(fail::processing_error),
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<S>) {
        ctx.span(id)
            .unwrap_or_else(fail::span_not_in_context)
            .extensions_mut()
            .get_mut::<TreeSpanOpened>()
            .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
            .enter();
    }

    fn on_exit(&self, id: &Id, ctx: Context<S>) {
        ctx.span(id)
            .unwrap_or_else(fail::span_not_in_context)
            .extensions_mut()
            .get_mut::<TreeSpanOpened>()
            .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
            .exit();
    }

    fn on_close(&self, id: Id, ctx: Context<S>) {
        let span = ctx.span(&id).unwrap_or_else(fail::span_not_in_context);

        let (tree_attrs, tree_span) = span
            .extensions_mut()
            .remove::<TreeSpanOpened>()
            .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
            .close();

        match span.parent() {
            Some(parent) => parent
                .extensions_mut()
                .get_mut::<TreeSpanOpened>()
                .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
                .log_span(tree_attrs, tree_span),
            None => self
                .processor
                .process(Tree::new(tree_attrs, tree_span))
                .unwrap_or_else(fail::processing_error),
        }
    }
}
