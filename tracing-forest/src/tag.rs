//! Provide supplemental information to events.
//!
//! # Use cases for tags
//!
//! Using tags in trace data can improve readability by distinguishing
//! between different kinds of trace data such as requests, internal state,
//! or special operations. An error during a network request could mean a
//! timeout occurred, while an error in the internal state could mean
//! corruption. Both are errors, but one should be treated more seriously than
//! the other, and therefore the two should be easily distinguishable.
//!
//! # Using tags
//!
//! This module provides the [`Tag`] type, which holds information required for
//! formatting events, and the [`TagParser`] trait, which allows Tracing events to
//! be parsed to return `Option<Tag>`.
//!
//! ## Examples
//! Declaring and using a custom `TagParser`.
//! ```
//! use tracing::{info, error, Event, Level};
//! use tracing_forest::tag::Tag;
//!
//! // `TagParser` is implemented for all `Fn(&tracing::Event) -> Option<Tag>`,
//! // so a top-level `fn` can be used.
//! fn simple_tag(event: &Event) -> Option<Tag> {
//!     // `target` is similar to a field, except has its own syntax and is a
//!     // `&'static str`. It's intended to mark where the event occurs, making
//!     // it ideal for storing tags.
//!     let target = event.metadata().target();
//!     let level = *event.metadata().level();
//!
//!     match target {
//!         "security" if level == Level::ERROR => {
//!             Some(Tag::new_custom_level(Some(target), "critical", '🔐'))
//!         }
//!         "admin" | "request" => Some(Tag::new(Some(target), level)),
//!         _ => None,
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     tracing_forest::worker_task()
//!         .set_tag(simple_tag)
//!         .build()
//!         .on(async {
//!             // Since `my_tag` reads from the `target`, we use the target.
//!             // If it parsed the event differently, we would reflect that here.
//!             info!(target: "admin", "some info for the admin");
//!             error!(target: "request", "the request timed out");
//!             error!(target: "security", "the db has been breached");
//!             info!("no tags here");
//!         })
//!         .await;
//! }
//! ```
//! ```log
//! INFO     💬 [admin.info]: some info for the admin
//! ERROR    🚨 [request.error]: the request timed out
//! ERROR    🔐 [security.critical]: the db has been breached
//! INFO     💬 [info]: no tags here
//! ```
//!
use crate::cfg_serde;
use std::fmt;
use tracing::{Event, Level};

/// A type containing categorical information about where an event occurred.
///
/// See the [module-level documentation](crate::tag) for more details.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Tag {
    /// Optional prefix for the tag message
    prefix: Option<&'static str>,

    /// Level specifying the important of the log.
    ///
    /// This value isn't necessarily "trace", "debug", "info", "warn", or "error",
    /// and can be customized.
    level: &'static str,

    /// An icon, typically emoji, that represents the tag.
    icon: char,
}

impl Tag {
    /// Returns a `Tag` constructed from an optional prefix and a custom level
    /// and icon.
    pub const fn new_custom_level(
        prefix: Option<&'static str>,
        level: &'static str,
        icon: char,
    ) -> Self {
        Tag {
            prefix,
            level,
            icon,
        }
    }

    /// Returns a `Tag` constructed from an optional prefix and a level.
    pub const fn new(prefix: Option<&'static str>, level: Level) -> Self {
        match level {
            Level::TRACE => Tag::new_custom_level(prefix, "trace", '📍'),
            Level::DEBUG => Tag::new_custom_level(prefix, "debug", '🐛'),
            Level::INFO => Tag::new_custom_level(prefix, "info", '💬'),
            Level::WARN => Tag::new_custom_level(prefix, "warn", '🚧'),
            Level::ERROR => Tag::new_custom_level(prefix, "error", '🚨'),
        }
    }

    /// Returns a `Tag` constructed from a `Level` and no prefix.
    pub const fn new_level(level: Level) -> Self {
        Tag::new(None, level)
    }

    /// Returns the `Tag`'s icon for printing.
    pub const fn icon(&self) -> char {
        self.icon
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(prefix) = self.prefix {
            write!(f, "{}.{}", prefix, self.level)
        } else {
            f.write_str(self.level)
        }
    }
}

impl From<Level> for Tag {
    fn from(level: Level) -> Self {
        Tag::new_level(level)
    }
}

impl PartialEq<str> for Tag {
    fn eq(&self, other: &str) -> bool {
        match self.prefix {
            Some(prefix) => other
                .strip_prefix(prefix)
                .and_then(|s| s.strip_prefix('.'))
                .map_or(false, |s| s == self.level),
            None => other == self.level,
        }
    }
}

cfg_serde! {
    use serde::{Serialize, Serializer};

    impl Serialize for Tag {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            // This could probably go in a smart string
            serializer.serialize_str(&format!("{}", self))
        }
    }
}

/// A type that can parse [`Tag`]s from Tracing events.
///
/// This trait is blanket-implemented for all `Fn(&tracing::Event) -> Option<Tag>`,
/// so top-level `fn`s can be used.
///
/// See the [module-level documentation](crate::tag) for more details.
pub trait TagParser: 'static {
    /// Parse a tag from a [`tracing::Event`]
    fn try_parse(&self, event: &Event) -> Option<Tag>;
}

/// A `TagParser` that always returns `None`.
#[derive(Clone, Debug)]
pub struct NoTag;

impl TagParser for NoTag {
    #[inline]
    fn try_parse(&self, _event: &Event) -> Option<Tag> {
        None
    }
}

impl<F> TagParser for F
where
    F: 'static + Fn(&Event) -> Option<Tag>,
{
    #[inline]
    fn try_parse(&self, event: &Event) -> Option<Tag> {
        self(event)
    }
}
