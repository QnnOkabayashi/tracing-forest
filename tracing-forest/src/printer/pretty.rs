use crate::printer::Formatter;
use crate::tree::{Event, Shared, Span, Tree};
use std::fmt::{self, Write};

/// Format logs for pretty printing.
///
/// # Interpreting span times
///
/// Spans have the following format:
/// ```txt
/// <NAME> [ <DURATION> | <BODY> / <ROOT> ]
/// ```
/// * `DURATION` represents the total time the span was entered for. If the span
/// was used to instrument a `Future` that sleeps, then that time won't be counted
/// since the `Future` won't be polled during that time, and so the span won't enter.
/// * `BODY` represents the percent time the span is entered relative to the root
/// span, *excluding* time that any child spans are entered.
/// * `ROOT` represents the percent time the span is entered relative to the root
/// span, *including* time that any child spans are entered.
///
/// As a mental model, look at `ROOT` to quickly narrow down which branches are
/// costly, and look at `BASE` to pinpoint exactly which spans are expensive.
///
/// Spans without any child spans would have the same `BASE` and `ROOT`, so the
/// redundency is omitted.
///
/// # Examples
///
/// An arbitrarily complex example:
/// ```log
/// INFO     try_from_entry_ro [ 7.47ms | 6.52% / 100.00% ]
/// INFO     ┝━ server::internal_search [ 6.98ms | 31.88% / 93.47% ]
/// INFO     │  ┝━ __ [filter.info]: Some filter info...
/// INFO     │  ┝━ server::search [ 4.59ms | 0.81% / 61.41% ]
/// INFO     │  │  ┝━ be::search [ 4.51ms | 0.40% / 60.31% ]
/// INFO     │  │  │  ┕━ be::search -> filter2idl [ 4.48ms | 22.40% / 59.91% ]
/// INFO     │  │  │     ┝━ be::idl_arc_sqlite::get_idl [ 571µs | 7.64% ]
/// INFO     │  │  │     │  ┕━ ＿ [filter.info]: Some filter info...
/// INFO     │  │  │     ┕━ be::idl_arc_sqlite::get_idl [ 2.23ms | 29.85% ]
/// ERROR    │  │  │        ┝━ 🚨 [admin.error]: Oh no, an admin error occurred :(
/// DEBUG    │  │  │        ┝━ 🐛 [debug]: An untagged debug log
/// INFO     │  │  │        ┕━ ＿ [admin.info]: there's been a big mistake | alive: false | status: "very sad"
/// INFO     │  │  ┕━ be::idl_arc_sqlite::get_identry [ 21.4µs | 0.28% ]
/// INFO     │  │     ┝━ 🔐 [security.critical]: A security critical log
/// INFO     │  │     ┕━ 🔓 [security.access]: A security access log
/// INFO     │  ┕━ server::search<filter_resolve> [ 13.4µs | 0.17% ]
/// WARN     │     ┕━ 🚧 [filter.warn]: Some filter warning
/// TRACE    ┕━ 📍 [trace]: Finished!
/// ```
#[derive(Debug, Default)]
pub struct Pretty {
    _priv: (),
}

impl Formatter for Pretty {
    type Error = fmt::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, fmt::Error> {
        let mut writer = String::with_capacity(256);

        self.format_tree(tree, None, &mut Vec::with_capacity(16), &mut writer)?;

        Ok(writer)
    }
}

impl Pretty {
    fn format_tree(
        &self,
        tree: &Tree,
        duration_root: Option<f64>,
        indent: &mut Vec<Indent>,
        writer: &mut String,
    ) -> fmt::Result {
        match tree {
            Tree::Event(event) => {
                self.format_shared(&event.shared, writer)?;
                self.format_indent(indent, writer)?;
                self.format_event(event, writer)
            }
            Tree::Span(span) => {
                self.format_shared(&span.shared, writer)?;
                self.format_indent(indent, writer)?;
                self.format_span(span, duration_root, indent, writer)
            }
        }
    }

    fn format_shared(&self, shared: &Shared, writer: &mut String) -> fmt::Result {
        #[cfg(feature = "uuid")]
        write!(writer, "{} ", shared.uuid)?;

        #[cfg(feature = "chrono")]
        write!(writer, "{:<32} ", shared.timestamp.to_rfc3339())?;

        write!(writer, "{:<8} ", shared.level)
    }

    fn format_indent(&self, indent: &mut Vec<Indent>, writer: &mut String) -> fmt::Result {
        for indent in indent.iter() {
            writer.write_str(indent.repr())?;
        }
        Ok(())
    }

    fn format_event(&self, event: &Event, writer: &mut String) -> fmt::Result {
        let tag = event.tag();
        let message = event.message().unwrap_or("");

        write!(writer, "{} [{}]: {}", tag.icon(), tag, message)?;

        for field in event.fields().iter() {
            write!(writer, " | {}: {}", field.key(), field.value())?;
        }

        writeln!(writer)
    }

    fn format_span(
        &self,
        span: &Span,
        duration_root: Option<f64>,
        indent: &mut Vec<Indent>,
        writer: &mut String,
    ) -> fmt::Result {
        let total_duration = span.total_duration().as_nanos() as f64;
        let inner_duration = span.inner_duration().as_nanos() as f64;
        let root_duration = duration_root.unwrap_or(total_duration);
        let percent_total_of_root_duration = 100.0 * total_duration / root_duration;

        write!(
            writer,
            "{} [ {} | ",
            span.name(),
            DurationDisplay(total_duration)
        )?;

        if inner_duration > 0.0 {
            let base_duration = span.base_duration().as_nanos() as f64;
            let percent_base_of_root_duration = 100.0 * base_duration / root_duration;
            write!(writer, "{:.2}% / ", percent_base_of_root_duration)?;
        }

        writeln!(writer, "{:.2}% ]", percent_total_of_root_duration)?;

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
                self.format_tree(tree, Some(root_duration), indent, writer)?;
            }

            if let Some(edge) = indent.last_mut() {
                *edge = Indent::Turn;
            }
            self.format_tree(last, Some(root_duration), indent, writer)?;

            indent.pop();
        }

        Ok(())
    }
}

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
