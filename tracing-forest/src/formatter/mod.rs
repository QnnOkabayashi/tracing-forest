//! Trait for formatting logs.
//!
//! See [`Formatter`] for more details.

use tracing_subscriber::registry::{LookupSpan, SpanRef};

use crate::fail;
use crate::layer::{KeyValue, Tree, TreeAttrs, TreeEvent, TreeSpanOpened};
use crate::tag::TagData;
use std::fmt::{self, Write};
use std::io;

pub mod pretty;

#[cfg(feature = "json")]
pub mod json;

/// A type that formats [`Tree`]s into a buffer.
///
/// [`Formatter`] types are typically used by [`Processor`]s in order to break
/// down processing responsibilities into smaller, composable units.
///
/// If you're implementing a custom formatter, see the [layer module]
/// documentation for internal representation details.
///
/// [`Processor`]: crate::processor::Processor
/// [layer module]: crate::layer
pub trait Formatter {
    /// Format a [`Tree`] into a buffer for writing.
    fn fmt(&self, tree: Tree, writer: &mut Vec<u8>) -> io::Result<()>;
}

// Format and write emergency logs to stderr
pub(crate) fn format_immediate<S>(
    attrs: &TreeAttrs,
    event: &TreeEvent,
    span: Option<SpanRef<S>>,
) -> fmt::Result
where
    S: for<'a> LookupSpan<'a>,
{
    // uuid timestamp LEVEL root > inner > leaf > my message here | key: val
    let tag = TagData::from(attrs.level);
    let mut writer = format!("{icon} IMMEDIATE {icon} ", icon = tag.icon);

    #[cfg(feature = "uuid")]
    if let Some(span) = &span {
        let uuid = span
            .extensions()
            .get::<TreeSpanOpened>()
            .unwrap_or_else(fail::tree_span_opened_not_in_extensions)
            .uuid();
        write!(writer, "{} ", uuid)?;
    }

    #[cfg(feature = "chrono")]
    write!(writer, "{} ", attrs.timestamp.to_rfc3339())?;
    write!(writer, "{:<8} ", attrs.level)?;

    if let Some(span) = &span {
        for ancestor in span.scope().from_root() {
            write!(writer, "{} > ", ancestor.name())?;
        }
    }

    write!(writer, "{}", event.message)?;

    for KeyValue { key, value } in event.fields.iter() {
        write!(writer, " | {}: {}", key, value)?;
    }

    eprintln!("{}", writer);

    Ok(())
}
