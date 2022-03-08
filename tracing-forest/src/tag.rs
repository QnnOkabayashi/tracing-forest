//! Trait for tagging events with custom messages and levels.
//!
//! # Why use tags
//!
//! Using tags in your application can improve readability by distinguishing
//! between different kinds of trace data such as requests, internal state,
//! or special operations. An error during a network request could mean a
//! timeout occurred, while an error in the internal state could mean
//! corruption. Both are errors, but one should be treated more seriously than
//! the other, and therefore should be easily distinguishable.
//!
//! # Custom macros for applications
//!
//! The first step to using custom tags is to call [`tracing_forest::declare_tags!`]
//! at the root level of your crate for macro hygiene purposes. Then define an
//! `enum` type with variants for each possible log, and finally [deriving] the
//! [`Tag`] trait. Ensure that the visibility is `pub(crate)` so any generated
//! macros have access to it.
//!
//! [`tracing_forest::declare_tags!`]: crate::declare_tags!
//! ```
//! // lib.rs
//! tracing_forest::declare_tags! {
//!     use tracing_forest::Tag;
//!    
//!     #[derive(Tag)]
//!     pub(crate) enum MyTag {
//!         #[tag(lvl = "trace", msg = "simple")]
//!         Simple,
//!         #[tag(
//!             lvl = "info",
//!             msg = "all.features",
//!             icon = '🔐',
//!             macro = "all_features"
//!         )]
//!         AllFeatures,
//!     }
//! }
//! ```
//! In `enum` types, each variant must be a unit type, and have the `#[tag(..)]`
//! attribute. Similarly, `struct` types must be unit types and have the
//! `#[tag(..)]` attribute above their declaration.
//! The attribute has four arguments that it takes:
//! * `lvl`: The log level at which the log occurs at, like `"trace"` or
//! `"warn"`. This is used to determine the default icon and the log level if
//! a macro is derived.
//! * `msg`: A minimalistic message displayed with the log.
//! * `icon`: An optional character displayed during pretty formatting. Defaults
//! do the icon associated with the level.
//! * `macro`: An optional identifier used to declare a macro that can write
//! logs with the specified tag kind. If not provided, then no macro is
//! generated.
//!
//! If you generate macros, then they can be used throughout your application
//! ```
//! # tracing_forest::declare_tags! {
//! #   use tracing_forest::Tag;
//! #
//! #   #[derive(Tag)]
//! #   pub(crate) enum MyTag {
//! #       #[tag(lvl = "trace", msg = "simple")]
//! #       Simple,
//! #       #[tag(
//! #           lvl = "info",
//! #           msg = "all.features",
//! #           icon = '🔐',
//! #           macro = "all_features"
//! #       )]
//! #       AllFeatures,
//! #   }
//! # }
//! #[tracing_forest::main(tag = "MyTag")]
//! fn main() {
//!     use tracing_forest::Tag;
//!     tracing::trace!(__event_tag = crate::tracing_forest_tag::MyTag::Simple.as_field(), "a simple log");
//!     all_features!("all the features wow");
//! }
//! ```
//! ```log
//! TRACE    📍 [simple]: a simple log
//! INFO     🔐 [all.features]: all the features wow
//! ```
//!
//! Tagging works by passing in a field-value pair to [`tracing`]s log macros
//! with the field name `__event_tag`, meaning that this is a reserved name that
//! must not be used for other values, and may panic otherwise.
//!
//! ## Note:
//!
//! Although the [`Tag`] trait is unsafe to implement, it is guaranteed that
//! `Tag::as_field` will retain the same name and input parameter `&self`.
//!
//! [deriving]: tracing_forest_macros::Tag
//!
//! # Example
//!
//! ```
//! tracing_forest::declare_tags! {
//!     use tracing_forest::Tag;
//!
//!     #[derive(Tag)]
//!     pub(crate) enum KanidmTag {
//!         #[tag(lvl = "info", msg = "admin.info", macro = "admin_info")]
//!         AdminInfo,
//!         #[tag(lvl = "warn", msg = "admin.warn", macro = "admin_warn")]
//!         AdminWarn,
//!         #[tag(lvl = "error", msg = "admin.error", macro = "admin_error")]
//!         AdminError,
//!         #[tag(lvl = "trace", msg = "request.trace", macro = "request_trace")]
//!         RequestTrace,
//!         #[tag(lvl = "info", msg = "request.info", macro = "request_info")]
//!         RequestInfo,
//!         #[tag(lvl = "warn", msg = "request.warn", macro = "request_warn")]
//!         RequestWarn,
//!         #[tag(lvl = "error", msg = "request.error", macro = "request_error")]
//!         RequestError,
//!         #[tag(lvl = "trace", msg = "security.access", icon = '🔓', macro = "security_access")]
//!         SecurityAccess,
//!         #[tag(lvl = "info", msg = "security.info", icon = '🔒', macro = "security_info")]
//!         SecurityInfo,
//!         #[tag(lvl = "error", msg = "security.critical", icon = '🔐', macro = "security_critical")]
//!         SecurityCritical,
//!         #[tag(lvl = "trace", msg = "filter.trace", macro = "filter_trace")]
//!         FilterTrace,
//!         #[tag(lvl = "info", msg = "filter.info", macro = "filter_info")]
//!         FilterInfo,
//!         #[tag(lvl = "warn", msg = "filter.warn", macro = "filter_warn")]
//!         FilterWarn,
//!         #[tag(lvl = "error", msg = "filter.error", macro = "filter_error")]
//!         FilterError,
//!     }
//! }
//! ```
use crate::cfg_serde;
use std::fmt;
pub use tracing::{Event, Level};

/// The type that all tags resolve to once collected.
#[derive(Debug, Clone)]
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

    pub const fn new(prefix: Option<&'static str>, level: Level) -> Self {
        match level {
            Level::TRACE => Tag::new_custom_level(prefix, "trace", '📍'),
            Level::DEBUG => Tag::new_custom_level(prefix, "debug", '🐛'),
            Level::INFO => Tag::new_custom_level(prefix, "info", '💬'),
            Level::WARN => Tag::new_custom_level(prefix, "warn", '🚧'),
            Level::ERROR => Tag::new_custom_level(prefix, "error", '🚨'),
        }
    }

    pub const fn new_level(level: Level) -> Self {
        Tag::new(None, level)
    }

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

/// Parse a tag from a [`tracing::Event`]
pub trait GetTag: 'static {
    fn get_tag(&self, event: &Event) -> Option<Tag>;
}

#[derive(Debug)]
pub struct NoTag;

impl GetTag for NoTag {
    fn get_tag(&self, _event: &Event) -> Option<Tag> {
        None
    }
}

impl<F> GetTag for F
where
    F: 'static + Fn(&Event) -> Option<Tag>,
{
    #[inline]
    fn get_tag(&self, event: &Event) -> Option<Tag> {
        (self)(event)
    }
}
