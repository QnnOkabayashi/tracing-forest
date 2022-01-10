//! A [`Formatter`] that formats logs for pretty printing.
//!
//! See [`Pretty`] for more details.

use crate::formatter::Formatter;
use crate::layer::{KeyValue, Tree, TreeAttrs, TreeEvent, TreeKind, TreeSpan};
use crate::private::{DEBUG_ICON, ERROR_ICON, INFO_ICON, TRACE_ICON, WARN_ICON};
use crate::tag::TagData;
use std::fmt;
use std::io::{self, Write};
use tracing::Level;

/// Format logs for pretty printing.
/// 
/// # Examples
/// 
/// ```log
/// INFO     try_from_entry_ro [ 7.47ms | 6.523% / 100.000% ]
/// INFO     ┝━ server::internal_search [ 6.98ms | 31.887% / 93.477% ]
/// INFO     │  ┝━ 💬 [filter.info]: Some filter info...
/// INFO     │  ┝━ server::search [ 4.59ms | 0.813% / 61.410% ]
/// INFO     │  │  ┝━ be::search [ 4.51ms | 0.400% / 60.311% ]
/// INFO     │  │  │  ┕━ be::search -> filter2idl [ 4.48ms | 22.408% / 59.911% ]
/// INFO     │  │  │     ┝━ be::idl_arc_sqlite::get_idl [ 571µs | 7.645% ]
/// INFO     │  │  │     │  ┕━ 💬 [filter.info]: Some filter info...
/// INFO     │  │  │     ┕━ be::idl_arc_sqlite::get_idl [ 2.23ms | 29.858% ]
/// ERROR    │  │  │        ┝━ 🚨 [admin.error]: Oopsies, and admin error occurred :/
/// DEBUG    │  │  │        ┝━ 🐛 [debug]: An untagged debug log
/// INFO     │  │  │        ┕━ 💬 [admin.info]: there's been a big mistake | alive: false | status: "very sad"
/// INFO     │  │  ┕━ be::idl_arc_sqlite::get_identry [ 21.4µs | 0.286% ]
/// INFO     │  │     ┝━ 🔐 [security.critical]: A security critical log
/// INFO     │  │     ┕━ 🔓 [security.access]: A security access log
/// INFO     │  ┕━ server::search<filter_resolve> [ 13.4µs | 0.179% ]
/// WARN     │     ┕━ 🚧 [filter.warn]: Some filter warning lol
/// TRACE    ┕━ 📍 [trace]: We finished!
/// ```
pub struct Pretty {
    #[doc(hidden)]
    _priv: (),
}

impl Pretty {
    /// Constructs a new [`Pretty`] formatter.
    pub const fn new() -> Self {
        Pretty { _priv: () }
    }
}

impl Formatter for Pretty {
    fn fmt(&self, tree: Tree, writer: &mut Vec<u8>) -> io::Result<()> {
        let mut indent = Vec::with_capacity(0);

        format_tree(&tree, None, &mut indent, writer)
    }
}

#[derive(Copy, Clone)]
enum Edge {
    Null,
    Line,
    Fork,
    Turn,
}

impl Edge {
    fn repr(&self) -> &'static str {
        match self {
            Self::Null => "   ",
            Self::Line => "│  ",
            Self::Fork => "┝━ ",
            Self::Turn => "┕━ ",
        }
    }
}

fn format_attrs(attrs: &TreeAttrs, writer: &mut Vec<u8>) -> io::Result<()> {
    #[cfg(feature = "uuid")]
    write!(writer, "{} ", attrs.uuid)?;

    #[cfg(feature = "chrono")]
    write!(writer, "{:<32} ", attrs.timestamp.to_rfc3339())?;

    write!(writer, "{:<8} ", attrs.level)
}

fn format_indent(indent: &mut Vec<Edge>, writer: &mut Vec<u8>) -> io::Result<()> {
    indent
        .iter()
        .try_for_each(|edge| writer.write_all(edge.repr().as_bytes()))
}

fn format_event(event: &TreeEvent, level: Level, writer: &mut Vec<u8>) -> io::Result<()> {
    let (message, icon) = match event.tag {
        Some(TagData { message, icon }) => (message, icon),
        None => match level {
            Level::TRACE => ("trace", TRACE_ICON),
            Level::DEBUG => ("debug", DEBUG_ICON),
            Level::INFO => ("info", INFO_ICON),
            Level::WARN => ("warn", WARN_ICON),
            Level::ERROR => ("error", ERROR_ICON),
        },
    };

    write!(writer, "{} [{}]: {}", icon, message, event.message)?;

    for KeyValue { key, value } in event.fields.iter() {
        write!(writer, " | {}: {}", key, value)?;
    }

    writeln!(writer)
}

fn format_span(
    span: &TreeSpan,
    duration_root: Option<f64>,
    indent: &mut Vec<Edge>,
    writer: &mut Vec<u8>,
) -> io::Result<()> {
    let duration_total = span.duration_total.as_nanos() as f64;
    let duration_nested = span.duration_nested.as_nanos() as u64;
    let duration_root = duration_root.unwrap_or(duration_total);
    let load_total = 100.0 * duration_total / duration_root;

    write!(
        writer,
        "{} [ {} | ",
        span.name,
        DurationDisplay(duration_total)
    )?;

    if duration_nested > 0 {
        let load_direct = 100.0 * (duration_total - duration_nested as f64) / duration_root;
        write!(writer, "{:.3}% / ", load_direct)?;
    }

    writeln!(writer, "{:.3}% ]", load_total)?;

    if let Some((last, remaining)) = span.children.split_last() {
        match indent.last_mut() {
            Some(edge @ Edge::Turn) => *edge = Edge::Null,
            Some(edge @ Edge::Fork) => *edge = Edge::Line,
            _ => {}
        }

        indent.push(Edge::Fork);

        for tree in remaining {
            if let Some(edge) = indent.last_mut() {
                *edge = Edge::Turn;
            }
            format_tree(tree, Some(duration_root), indent, writer)?;
        }

        if let Some(edge) = indent.last_mut() {
            *edge = Edge::Turn;
        }
        format_tree(last, Some(duration_root), indent, writer)?;

        indent.pop();
    }

    Ok(())
}

fn format_tree(
    tree: &Tree,
    duration_root: Option<f64>,
    indent: &mut Vec<Edge>,
    writer: &mut Vec<u8>,
) -> io::Result<()> {
    format_attrs(&tree.attrs, writer)?;

    format_indent(indent, writer)?;

    match &tree.kind {
        TreeKind::Event(event) => format_event(event, tree.attrs.level, writer),
        TreeKind::Span(span) => format_span(span, duration_root, indent, writer),
    }
}

struct DurationDisplay(f64);

// Taken from chrono
impl fmt::Display for DurationDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut t = self.0;
        for unit in ["ns", "µs", "ms", "s"] {
            if t < 10.0 {
                return write!(f, "{:.2}{}", t, unit);
            } else if t < 100.0 {
                return write!(f, "{:.1}{}", t, unit);
            } else if t < 1000.0 {
                return write!(f, "{:.0}{}", t, unit);
            }
            t /= 1000.0;
        }
        write!(f, "{:.0}s", t * 1000.0)
    }
}
