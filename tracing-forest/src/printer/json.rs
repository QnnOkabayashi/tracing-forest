use crate::printer::Formatter;
use crate::tree::ser;
use crate::tree::{Event, Shared, Span, Tree};
use serde::Serialize;
use std::fmt;
use std::io::{Cursor, Write};
use std::time::Duration;

/// TBD
#[derive(Debug)]
pub struct Json;

impl Formatter for Json {
    type Error = fmt::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, fmt::Error> {
        let mut writer = Cursor::new(Vec::with_capacity(256));

        Self::format_tree(tree, &mut writer)?;

        String::from_utf8(writer.into_inner()).map_err(|_err| fmt::Error)
    }
}

#[derive(Serialize)]
struct StartSpan<'a> {
    #[serde(flatten)]
    pub(crate) shared: &'a Shared,

    /// The name of the span.
    pub(crate) name: &'static str,

    #[serde(rename = "nanos_total", serialize_with = "ser::nanos")]
    pub(crate) total_duration: Duration,

    #[serde(rename = "nanos_inner", serialize_with = "ser::nanos")]
    pub(crate) inner_duration: Duration,
}

#[derive(Serialize)]
struct EndSpan<'a> {
    #[serde(flatten)]
    pub(crate) shared: &'a Shared,

    /// The name of the span.
    pub(crate) name: &'static str,
}

impl Json {
    fn format_tree(tree: &Tree, writer: &mut Cursor<Vec<u8>>) -> fmt::Result {
        match tree {
            Tree::Event(event) => Self::format_event(event, writer),
            Tree::Span(span) => {
                // This needs to be called before all other actions are taken. If we don't
                // do this now, we end up rendering every shared item of defered spans, but
                // they are incomplete and we end up with a lot of junk on screen.
                if !span.should_render() {
                    return Ok(());
                }

                Self::format_span(span, writer)
            }
        }
    }

    fn format_event(event: &Event, writer: &mut Cursor<Vec<u8>>) -> fmt::Result {
        serde_json::to_writer(&mut *writer, event)
            .map_err(|err| {
                eprintln!("serde_json_error = {:?}", err);
                fmt::Error
            })
            .and_then(|_| writer.write(b"\n").map(|_| ()).map_err(|_| fmt::Error))
    }

    fn format_span(span: &Span, writer: &mut Cursor<Vec<u8>>) -> fmt::Result {
        let start_span = StartSpan {
            shared: &span.shared,
            name: span.name,
            total_duration: span.total_duration,
            inner_duration: span.inner_duration,
        };

        serde_json::to_writer(&mut *writer, &start_span)
            .map_err(|err| {
                eprintln!("serde_json_error = {:?}", err);
                fmt::Error
            })
            .and_then(|_| writer.write(b"\n").map(|_| ()).map_err(|_| fmt::Error))?;

        let nodes: Vec<_> = span
            .nodes()
            .iter()
            .filter(|node| node.should_render())
            .collect();

        for node in nodes {
            Self::format_tree(node, writer)?;
        }

        let end_span = EndSpan {
            shared: &span.shared,
            name: span.name,
        };

        serde_json::to_writer(&mut *writer, &end_span)
            .map_err(|err| {
                eprintln!("serde_json_error = {:?}", err);
                fmt::Error
            })
            .and_then(|_| writer.write(b"\n").map(|_| ()).map_err(|_| fmt::Error))
    }
}
