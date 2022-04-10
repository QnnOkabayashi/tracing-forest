use crate::fail;
use crate::printer::{PrettyPrinter, Printer};
use crate::processor::{Processor, Sink};
use crate::tag::{NoTag, Tag, TagParser};
use crate::tree::{self, FieldSet, Tree};
#[cfg(feature = "chrono")]
use chrono::Utc;
use std::fmt;
use std::io::{self, Write};
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::{LookupSpan, Registry, SpanRef};
use tracing_subscriber::util::SubscriberInitExt;
#[cfg(feature = "uuid")]
use uuid::Uuid;
#[cfg(feature = "uuid")]
pub(crate) mod id;

pub(crate) struct OpenedSpan {
    span: tree::Span,
    start: Instant,
}

impl OpenedSpan {
    fn new<S>(attrs: &Attributes, ctx: &Context<S>) -> Self
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        #[cfg(feature = "uuid")]
        let uuid = {
            let mut maybe_uuid = None;

            attrs.record(&mut |field: &Field, value: &dyn fmt::Debug| {
                if field.name() == "uuid" && maybe_uuid.is_none() {
                    const SIZE: usize = 64;
                    let mut buf = [0u8; SIZE];
                    let mut remaining = &mut buf[..];

                    if let Ok(()) = write!(remaining, "{:?}", value) {
                        let len = SIZE - remaining.len();
                        if let Ok(parsed) = id::try_parse(&buf[..len]) {
                            maybe_uuid = Some(parsed);
                        }
                    }
                }

                // record other field-values pairs here...
            });

            match maybe_uuid {
                Some(uuid) => uuid,
                None => match ctx.lookup_current() {
                    Some(parent) => parent
                        .extensions()
                        .get::<OpenedSpan>()
                        .unwrap_or_else(fail::opened_span_not_in_exts)
                        .span
                        .uuid(),
                    None => Uuid::new_v4(),
                },
            }
        };

        let shared = tree::Shared {
            #[cfg(feature = "chrono")]
            timestamp: Utc::now(),
            #[cfg(feature = "uuid")]
            uuid,
            level: *attrs.metadata().level(),
        };

        let span = tree::Span::new(shared, attrs.metadata().name());

        OpenedSpan {
            span,
            start: Instant::now(),
        }
    }

    fn enter(&mut self) {
        self.start = Instant::now();
    }

    fn exit(&mut self) {
        self.span.total_duration += self.start.elapsed();
    }

    fn close(self) -> tree::Span {
        self.span
    }

    fn record_event(&mut self, event: tree::Event) {
        #[cfg(feature = "uuid")]
        let event = {
            let mut event = event;
            event.shared.uuid = self.span.uuid();
            event
        };

        self.span.children.push(Tree::Event(event));
    }

    fn record_span(&mut self, span: tree::Span) {
        self.span.inner_duration += span.total_duration();
        self.span.children.push(Tree::Span(span));
    }

    #[cfg(feature = "uuid")]
    pub(crate) fn uuid(&self) -> Uuid {
        self.span.uuid()
    }
}

/// A [`Layer`] that collects and processes trace data while preserving
/// contextual coherence.
#[derive(Clone, Debug)]
pub struct ForestLayer<P, T> {
    processor: P,
    tag: T,
}

impl<P: Processor, T: TagParser> ForestLayer<P, T> {
    /// Create a new `ForestLayer` from a [`Processor`] and a [`TagParser`].
    pub fn new(processor: P, tag: T) -> Self {
        ForestLayer { processor, tag }
    }
}

impl<P: Processor> From<P> for ForestLayer<P, NoTag> {
    fn from(processor: P) -> Self {
        ForestLayer::new(processor, NoTag)
    }
}

impl ForestLayer<Sink, NoTag> {
    /// Create a new `ForestLayer` that does nothing with collected trace data.
    pub fn sink() -> Self {
        ForestLayer::from(Sink)
    }
}

impl Default for ForestLayer<PrettyPrinter, NoTag> {
    fn default() -> Self {
        ForestLayer {
            processor: Printer::default(),
            tag: NoTag,
        }
    }
}

impl<P, T, S> Layer<S> for ForestLayer<P, T>
where
    P: Processor,
    T: TagParser,
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        let span = ctx.span(id).unwrap_or_else(fail::span_not_in_ctx);
        let opened = OpenedSpan::new(attrs, &ctx);

        let mut extensions = span.extensions_mut();
        extensions.insert(opened);
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        struct Visitor {
            message: Option<String>,
            fields: FieldSet,
            immediate: bool,
        }

        impl Visit for Visitor {
            fn record_bool(&mut self, field: &Field, value: bool) {
                match field.name() {
                    "immediate" => self.immediate |= value,
                    _ => self.record_debug(field, &value),
                }
            }

            fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
                let value = format!("{:?}", value);
                match field.name() {
                    "message" if self.message.is_none() => self.message = Some(value),
                    key => self.fields.push(tree::Field::new(key, value)),
                }
            }
        }

        let mut visitor = Visitor {
            message: None,
            fields: FieldSet::default(),
            immediate: false,
        };

        event.record(&mut visitor);

        let level = *event.metadata().level();
        let tag = self
            .tag
            .try_parse(event)
            .unwrap_or_else(|| Tag::from(level));

        let current_span = ctx.event_span(event);

        let event = tree::Event {
            shared: tree::Shared {
                #[cfg(feature = "uuid")]
                uuid: Uuid::nil(),
                #[cfg(feature = "chrono")]
                timestamp: Utc::now(),
                level,
            },
            message: visitor.message,
            tag,
            fields: visitor.fields,
        };

        if visitor.immediate {
            write_immediate(&event, current_span.as_ref()).expect("writing urgent failed");
        }

        match current_span.as_ref() {
            Some(parent) => parent
                .extensions_mut()
                .get_mut::<OpenedSpan>()
                .unwrap_or_else(fail::opened_span_not_in_exts)
                .record_event(event),
            None => self
                .processor
                .process(Tree::Event(event))
                .unwrap_or_else(fail::processing_error),
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<S>) {
        ctx.span(id)
            .unwrap_or_else(fail::span_not_in_ctx)
            .extensions_mut()
            .get_mut::<OpenedSpan>()
            .unwrap_or_else(fail::opened_span_not_in_exts)
            .enter();
    }

    fn on_exit(&self, id: &Id, ctx: Context<S>) {
        ctx.span(id)
            .unwrap_or_else(fail::span_not_in_ctx)
            .extensions_mut()
            .get_mut::<OpenedSpan>()
            .unwrap_or_else(fail::opened_span_not_in_exts)
            .exit();
    }

    fn on_close(&self, id: Id, ctx: Context<S>) {
        let span_ref = ctx.span(&id).unwrap_or_else(fail::span_not_in_ctx);

        let mut span = span_ref
            .extensions_mut()
            .remove::<OpenedSpan>()
            .unwrap_or_else(fail::opened_span_not_in_exts)
            .close();

        // Ensure that the total duration is at least as much as the inner
        // duration. This is caused by when a child span is manually passed
        // a parent span and then enters without entering the parent span. Also
        // when a child span is created within a parent, and then stored and
        // entered again when the parent isn't opened.
        //
        // Issue: https://github.com/QnnOkabayashi/tracing-forest/issues/11
        if span.total_duration < span.inner_duration {
            span.total_duration = span.inner_duration;
        }

        match span_ref.parent() {
            Some(parent) => parent
                .extensions_mut()
                .get_mut::<OpenedSpan>()
                .unwrap_or_else(fail::opened_span_not_in_exts)
                .record_span(span),
            None => self
                .processor
                .process(Tree::Span(span))
                .unwrap_or_else(fail::processing_error),
        }
    }
}

fn write_immediate<S>(event: &tree::Event, current: Option<&SpanRef<S>>) -> io::Result<()>
where
    S: for<'a> LookupSpan<'a>,
{
    // uuid timestamp LEVEL root > inner > leaf > my message here | key: val
    #[cfg(feature = "smallvec")]
    let mut writer = smallvec::SmallVec::<[u8; 256]>::new();
    #[cfg(not(feature = "smallvec"))]
    let mut writer = Vec::with_capacity(256);

    #[cfg(feature = "uuid")]
    if let Some(span) = current {
        let uuid = span
            .extensions()
            .get::<OpenedSpan>()
            .unwrap_or_else(fail::opened_span_not_in_exts)
            .span
            .uuid();
        write!(writer, "{} ", uuid)?;
    }

    #[cfg(feature = "chrono")]
    write!(writer, "{} ", event.timestamp().to_rfc3339())?;

    write!(writer, "{:<8} ", event.level())?;

    let tag = Tag::from(event.level());

    write!(writer, "{icon} IMMEDIATE {icon} ", icon = tag.icon())?;

    if let Some(span) = current {
        for ancestor in span.scope().from_root() {
            write!(writer, "{} > ", ancestor.name())?;
        }
    }

    // we should just do pretty printing here.

    if let Some(message) = event.message() {
        write!(writer, "{}", message)?;
    }

    for field in event.fields().iter() {
        write!(writer, " | {}: {}", field.key(), field.value())?;
    }

    writeln!(writer)?;

    io::stderr().write_all(&writer)
}

/// Initializes a global subscriber with a [`ForestLayer`] using the default configuration.
///
/// This function is intended for quick initialization and processes log trees "inline",
/// meaning it doesn't take advantage of a worker task for formatting and writing.
/// To use a worker task, consider using the [`worker_task`] function. Alternatively,
/// configure a `Subscriber` manually using a `ForestLayer`.
///
/// [`worker_task`]: crate::builder::worker_task
///
/// # Examples
/// ```
/// use tracing::{info, info_span};
///
/// tracing_forest::init();
///
/// info!("Hello, world!");
/// info_span!("my_span").in_scope(|| {
///     info!("Relevant information");
/// });
/// ```
/// Produces the the output:
/// ```log
/// INFO     ｉ [info]: Hello, world!
/// INFO     my_span [ 26.0µs | 100.000% ]
/// INFO     ┕━ ｉ [info]: Relevant information
/// ```
pub fn init() {
    Registry::default().with(ForestLayer::default()).init();
}
