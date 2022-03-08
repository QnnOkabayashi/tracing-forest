use crate::processor::Processor;
use crate::tag::{GetTag, NoTag, Tag};
use crate::tree::{self, FieldSet, Tree};
use crate::{cfg_uuid, fail};
#[cfg(feature = "chrono")]
use chrono::Utc;
use std::fmt;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::{LookupSpan, SpanRef};

cfg_uuid! {
    use uuid::Uuid;
    mod id;
    pub use id::id;
}

pub(crate) struct OpenedSpan {
    span: tree::Span,
    start: Instant,
}

impl OpenedSpan {
    fn open<S>(attrs: &Attributes, ctx: &Context<S>) -> Self
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        #[cfg(feature = "uuid")]
        let uuid = {
            let mut maybe_uuid = None;

            attrs.record(&mut |field: &Field, value: &dyn fmt::Debug| {
                if field.name() == "uuid" && maybe_uuid.is_none() {
                    #[cfg(feature = "smallvec")]
                    let mut buf = smallvec::SmallVec::<[u8; 64]>::new();
                    #[cfg(not(feature = "smallvec"))]
                    let mut buf = Vec::with_capacity(45);

                    if let Ok(()) = write!(buf, "{:?}", value) {
                        if let Ok(parsed) = id::try_parse(&buf[..]) {
                            maybe_uuid = Some(parsed);
                            return;
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

        let span = tree::Span {
            shared: tree::Shared {
                #[cfg(feature = "chrono")]
                timestamp: Utc::now(),
                #[cfg(feature = "uuid")]
                uuid,
                level: *attrs.metadata().level(),
            },
            name: attrs.metadata().name(),
            children: Vec::new(),
            total_duration: Duration::ZERO,
            inner_duration: Duration::ZERO,
        };

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

        self.span.children.push(Tree::Event(event))
    }

    fn record_span(&mut self, span: tree::Span) {
        self.span.inner_duration += span.total_duration();
        self.span.children.push(Tree::Span(span))
    }

    cfg_uuid! {
        pub(crate) fn uuid(&self) -> Uuid {
            self.span.uuid()
        }
    }
}

#[derive(Clone, Debug)]
pub struct ForestLayer<P, T> {
    processor: P,
    tag: T,
}

impl<P: Processor> ForestLayer<P, NoTag> {
    pub fn new(processor: P) -> Self {
        ForestLayer::new_with_tag(processor, NoTag)
    }
}

impl<P: Processor, T: GetTag> ForestLayer<P, T> {
    pub fn new_with_tag(processor: P, tag: T) -> Self {
        ForestLayer { processor, tag }
    }
}

impl<P, T, S> Layer<S> for ForestLayer<P, T>
where
    P: Processor,
    T: GetTag,
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes, id: &Id, ctx: Context<S>) {
        let span = ctx.span(id).unwrap_or_else(fail::span_not_in_ctx);
        let opened = OpenedSpan::open(attrs, &ctx);

        let mut extensions = span.extensions_mut();
        extensions.insert(opened);
    }

    fn on_event(&self, event: &Event, ctx: Context<S>) {
        struct Visitor {
            message: Option<String>,
            fields: FieldSet,
            urgent: bool,
        }

        impl Visit for Visitor {
            fn record_bool(&mut self, field: &Field, value: bool) {
                match field.name() {
                    "urgent" => self.urgent |= value,
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
            urgent: false,
        };

        event.record(&mut visitor);

        let level = *event.metadata().level();
        let tag = self.tag.get_tag(event).unwrap_or_else(|| Tag::from(level));

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

        if visitor.urgent {
            write_urgent(&event, current_span.as_ref()).expect("writing urgent failed");
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

        let span = span_ref
            .extensions_mut()
            .remove::<OpenedSpan>()
            .unwrap_or_else(fail::opened_span_not_in_exts)
            .close();

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

fn write_urgent<S>(event: &tree::Event, current: Option<&SpanRef<S>>) -> io::Result<()>
where
    S: for<'a> LookupSpan<'a>,
{
    // uuid timestamp LEVEL root > inner > leaf > my message here | key: val
    #[cfg(feature = "smallvec")]
    let mut writer = smallvec::SmallVec::<[u8; 256]>::new();
    #[cfg(not(feature = "smallvec"))]
    let mut writer = Vec::with_capacity(256);

    let tag = Tag::from(event.level());

    write!(writer, "{icon} URGENT {icon} ", icon = tag.icon())?;

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
