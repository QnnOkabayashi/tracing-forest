use crate::printer::{shared::DurationDisplay, Formatter};
use crate::tree::{Event, Shared, Span, Tree};
use crate::Tag;
use std::fmt::{self, Write};

#[cfg(feature = "smallvec")]
type IndentVec = smallvec::SmallVec<[Indent; 32]>;
#[cfg(not(feature = "smallvec"))]
type IndentVec = Vec<Indent>;

/// TBD
#[derive(Debug)]
pub struct Simple;

impl Formatter for Simple {
    type Error = fmt::Error;

    fn fmt(&self, tree: &Tree) -> Result<String, fmt::Error> {
        let mut writer = String::with_capacity(256);

        Self::format_tree(tree, None, &mut IndentVec::with_capacity(8), &mut writer)?;

        Ok(writer)
    }
}

impl Simple {
    fn format_tree(
        tree: &Tree,
        duration_root: Option<f64>,
        indent: &mut IndentVec,
        writer: &mut String,
    ) -> fmt::Result {
        match tree {
            Tree::Event(event) => {
                Self::format_shared(&event.shared, writer)?;
                Self::format_indent(indent, writer)?;
                Self::format_event(event, writer)
            }
            Tree::Span(span) => {
                // This needs to be called before all other actions are taken. If we don't
                // do this now, we end up rendering every shared item of defered spans, but
                // they are incomplete and we end up with a lot of junk on screen.
                if !span.should_render() {
                    return Ok(());
                }

                Self::format_shared(&span.shared, writer)?;
                Self::format_indent(indent, writer)?;
                Self::format_span(span, duration_root, indent, writer)
            }
        }
    }

    fn format_shared(shared: &Shared, writer: &mut String) -> fmt::Result {
        #[cfg(feature = "uuid")]
        write!(writer, "{} ", shared.uuid)?;

        #[cfg(feature = "chrono")]
        write!(writer, "{:<36} ", shared.timestamp.to_rfc3339())?;

        write!(writer, "{:<8} ", shared.level)
    }

    fn format_indent(indent: &[Indent], writer: &mut String) -> fmt::Result {
        for indent in indent {
            writer.write_str(indent.repr())?;
        }
        Ok(())
    }

    fn format_event(event: &Event, writer: &mut String) -> fmt::Result {
        let tag = event.tag().unwrap_or_else(|| Tag::from(event.level()));

        write!(writer, "[{}]: ", tag)?;

        if let Some(message) = event.message() {
            writer.write_str(message)?;
        }

        for field in event.fields() {
            write!(writer, " | {}: {}", field.key(), field.value())?;
        }

        writeln!(writer)
    }

    fn format_span(
        span: &Span,
        duration_root: Option<f64>,
        indent: &mut IndentVec,
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
            write!(writer, "{percent_base_of_root_duration:.2}% / ")?;
        }

        write!(writer, "{percent_total_of_root_duration:.2}% ]")?;

        for (n, field) in span.shared.fields.iter().enumerate() {
            write!(
                writer,
                "{} {}: {}",
                if n == 0 { "" } else { " |" },
                field.key(),
                field.value()
            )?;
        }
        writeln!(writer)?;

        // We need to limit our nodes to those that *will* render, else we
        // end up printing ghost indents.
        let nodes: Vec<_> = span
            .nodes()
            .iter()
            .filter(|node| node.should_render())
            .collect();

        if let Some((last, remaining)) = nodes.split_last() {
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
                Self::format_tree(tree, Some(root_duration), indent, writer)?;
            }

            if let Some(edge) = indent.last_mut() {
                *edge = Indent::Turn;
            }
            Self::format_tree(last, Some(root_duration), indent, writer)?;

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
            Self::Line => "|  ",
            Self::Fork => "|- ",
            Self::Turn => "|- ",
        }
    }
}
