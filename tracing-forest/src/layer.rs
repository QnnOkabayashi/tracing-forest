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

use crate::formatter::format_immediate;
use crate::processor::Processor;
use crate::tag::{NoTag, Tag, TagData, TagParser};
use crate::{cfg_chrono, cfg_json, cfg_uuid, fail};
#[cfg(feature = "smallvec")]
use smallvec::SmallVec;
use std::time::{Duration, Instant};
use std::{borrow::Cow, fmt};
use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::{Event, Id, Level, Subscriber};
use tracing_subscriber::layer::Layered;
use tracing_subscriber::Registry;
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
pub struct TreeLayer<P> {
    processor: P,
    tag_parser: TagParser,
}

impl<P: Processor> TreeLayer<P> {
    /// Create a new `TreeLayer` from a [`Processor`].
    pub fn new(processor: P) -> Self {
        TreeLayer {
            processor,
            tag_parser: NoTag::from_field,
        }
    }

    /// Compose the `TreeLayer` onto a [`Registry`].
    pub fn into_subscriber(self) -> Layered<Self, Registry> {
        self.with_subscriber(Registry::default())
    }

    /// Set the accepted [`Tag`] type of the `TreeLayer`.
    pub fn tag<T: Tag>(mut self) -> Self {
        self.tag_parser = T::from_field;
        self
    }
}

/// A node of a log tree.
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Tree {
    /// Shared fields associated with both spans and events.
    #[cfg_attr(feature = "json", serde(flatten))]
    pub attrs: TreeAttrs,
    /// Fields specific to either a span or an event.
    pub kind: TreeKind,
}

impl Tree {
    /// Create a new `Tree`.
    fn new(attrs: TreeAttrs, kind: impl Into<TreeKind>) -> Self {
        Tree {
            attrs,
            kind: kind.into(),
        }
    }
}

/// The shared attributes of both spans and events within a [`Tree`].
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct TreeAttrs {
    /// The ID that this trace data is associated with.
    #[cfg(feature = "uuid")]
    pub uuid: Uuid,
    /// When the trace data was collected.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "json", serde(serialize_with = "ser::timestamp"))]
    pub timestamp: DateTime<Utc>,
    /// Level the trace data was collected with.
    #[cfg_attr(feature = "json", serde(serialize_with = "ser::level"))]
    pub level: Level,
}

/// The kind of log, either a [`TreeEvent`] or a [`TreeSpan`].
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub enum TreeKind {
    Event(TreeEvent),
    Span(TreeSpan),
}

/// Information unique to logged events.
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct TreeEvent {
    /// An optional tag that the event was collected with.
    pub tag: Option<TagData>,
    /// The message associated with the event.
    pub message: Cow<'static, str>,
    /// Key-value data.
    #[cfg_attr(feature = "json", serde(serialize_with = "ser::fields"))]
    pub fields: Fields,
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

impl TreeKind {
    /// Converts into a [`TreeEvent`], if the kind is `Event`.
    pub fn into_event(self) -> Option<TreeEvent> {
        match self {
            TreeKind::Event(event) => Some(event),
            TreeKind::Span(_) => None,
        }
    }

    /// Converts into a [`TreeSpan`], if the kind is `Span`.
    pub fn into_span(self) -> Option<TreeSpan> {
        match self {
            TreeKind::Event(_) => None,
            TreeKind::Span(span) => Some(span),
        }
    }
}

/// Information unique to logged spans.
#[derive(Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct TreeSpan {
    /// The name of the span.
    pub name: &'static str,
    #[cfg_attr(
        feature = "json",
        serde(rename = "nanos_total", serialize_with = "ser::nanos")
    )]
    /// The duration that the span was entered for.
    pub duration_total: Duration,
    #[cfg_attr(
        feature = "json",
        serde(rename = "nanos_nested", serialize_with = "ser::nanos")
    )]
    /// The duration that child spans of this span were entered for.
    pub duration_nested: Duration,
    /// Spans and events that occurred inside of this span.
    pub children: Vec<Tree>,
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
                duration_nested: Duration::ZERO,
                duration_total: Duration::ZERO,
            },
            start: Instant::now(),
        }
    }

    fn enter(&mut self) {
        self.start = Instant::now();
    }

    fn exit(&mut self) {
        self.span.duration_total += self.start.elapsed();
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
        self.span.duration_nested += span.duration_total;
        self.span.children.push(Tree::new(attrs, span));
    }

    cfg_uuid! {
        pub fn uuid(&self) -> Uuid {
            self.attrs.uuid
        }
    }
}

impl<P: Processor> TreeLayer<P> {
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
            Some(parent) => {
                parent
                    .extensions_mut()
                    .get_mut::<TreeSpanOpened>()
                    .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
                    .log_event(tree_attrs, tree_event);
            }
            None => {
                self.processor.process(Tree::new(tree_attrs, tree_event));
            }
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
            None => self.processor.process(Tree::new(tree_attrs, tree_span)),
        }
    }
}
