use crate::printer::StringifyTree;
use crate::tree::{Event, Shared, Span, Tree};
use std::fmt::{self, Write};

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
#[derive(Debug)]
pub struct Pretty;

impl StringifyTree for Pretty {
    type Error = fmt::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, fmt::Error> {
        let mut writer = String::with_capacity(256);

        format_tree(tree, None, &mut Vec::with_capacity(16), &mut writer)?;

        Ok(writer)
    }
}

fn format_tree(
    tree: &Tree,
    duration_root: Option<f64>,
    indent: &mut Vec<Indent>,
    writer: &mut String,
) -> fmt::Result {
    match tree {
        Tree::Event(event) => {
            format_shared(&event.shared, writer)?;
            format_indent(indent, writer)?;
            format_event(event, writer)
        }
        Tree::Span(span) => {
            format_shared(&span.shared, writer)?;
            format_indent(indent, writer)?;
            format_span(span, duration_root, indent, writer)
        }
    }
}

fn format_shared(shared: &Shared, writer: &mut String) -> fmt::Result {
    #[cfg(feature = "uuid")]
    write!(writer, "{} ", shared.uuid)?;

    #[cfg(feature = "chrono")]
    write!(writer, "{:<32} ", shared.timestamp.to_rfc3339())?;

    write!(writer, "{:<8} ", shared.level)
}

fn format_indent(indent: &mut Vec<Indent>, writer: &mut String) -> fmt::Result {
    for indent in indent.iter() {
        writer.write_str(indent.repr())?;
    }
    Ok(())
}

fn format_event(event: &Event, writer: &mut String) -> fmt::Result {
    let tag = event.tag();
    let message = event.message().unwrap_or("");

    write!(writer, "{} [{}]: {}", tag.icon(), tag, message)?;

    for field in event.fields().iter() {
        write!(writer, " | {}: {}", field.key(), field.value())?;
    }

    writeln!(writer)
}

fn format_span(
    span: &Span,
    duration_root: Option<f64>,
    indent: &mut Vec<Indent>,
    writer: &mut String,
) -> fmt::Result {
    let duration_total = span.total_duration().as_nanos() as f64;
    let duration_nested = span.inner_duration().as_nanos() as u64;
    let root_duration = duration_root.unwrap_or(duration_total);
    let load_total = 100.0 * duration_total / root_duration;

    write!(
        writer,
        "{} [ {} | ",
        span.name(),
        DurationDisplay(duration_total)
    )?;

    if duration_nested > 0 {
        let load_direct = 100.0 * (duration_total - duration_nested as f64) / root_duration;
        write!(writer, "{:.3}% / ", load_direct)?;
    }

    writeln!(writer, "{:.3}% ]", load_total)?;

    if let Some((last, remaining)) = span.children().split_last() {
        match indent.last_mut() {
            Some(edge @ Indent::Turn) => *edge = Indent::Null,
            Some(edge @ Indent::Fork) => *edge = Indent::Line,
            _ => {}
        }

        indent.push(Indent::Fork);

        for tree in remaining {
            if let Some(edge) = indent.last_mut() {
                *edge = Indent::Fork;
            }
            format_tree(tree, Some(root_duration), indent, writer)?;
        }

        if let Some(edge) = indent.last_mut() {
            *edge = Indent::Turn;
        }
        format_tree(last, Some(root_duration), indent, writer)?;

        indent.pop();
    }

    Ok(())
}

#[derive(Copy, Clone)]
enum Indent {
    Null,
    Line,
    Fork,
    Turn,
}

impl Indent {
    fn repr(&self) -> &'static str {
        match self {
            Self::Null => "   ",
            Self::Line => "│  ",
            Self::Fork => "┝━ ",
            Self::Turn => "┕━ ",
        }
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
