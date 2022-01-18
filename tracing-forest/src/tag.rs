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
//! The first step to using custom tags is to define an `enum` type with
//! variants for each possible log, and then [deriving] the [`Tag`] trait.
//! ```
//! use tracing_forest::Tag;
//!
//! #[derive(Tag)]
//! pub enum MyTag {
//!     #[tag(lvl = "trace", msg = "simple")]
//!     Simple,
//!     #[tag(
//!         lvl = "info",
//!         msg = "all.features",
//!         icon = '🔐',
//!         macro = "all_features"
//!     )]
//!     AllFeatures,
//! }
//! ```
//! In `enum` types, each variant must be a unit type, and have the `#[tag(..)]`
//! attribute. `struct` types must have the `#[tag(..)]` attribute above their
//! declaration. The attribute has four arguments that it takes:
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
//! # use tracing_forest::Tag;
//! # #[derive(Tag)]
//! # pub enum MyTag {
//! #     #[tag(lvl = "trace", msg = "simple")]
//! #     Simple,
//! #     #[tag(
//! #         lvl = "info",
//! #         msg = "all.features",
//! #         icon = '🔐',
//! #         macro = "all_features"
//! #     )]
//! #     AllFeatures,
//! # }
//! #[tracing_forest::main(tag = "MyTag")]
//! fn main() {
//!     use tracing_forest::Tag;
//!     tracing::trace!(__event_tag = MyTag::Simple.as_field(), "a simple log");
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
//! # use tracing_forest::Tag;
//! #[derive(Tag)]
//! enum KanidmTag {
//!     #[tag(lvl = "info", msg = "admin.info", macro = "admin_info")]
//!     AdminInfo,
//!     #[tag(lvl = "warn", msg = "admin.warn", macro = "admin_warn")]
//!     AdminWarn,
//!     #[tag(lvl = "error", msg = "admin.error", macro = "admin_error")]
//!     AdminError,
//!     #[tag(lvl = "trace", msg = "request.trace", macro = "request_trace")]
//!     RequestTrace,
//!     #[tag(lvl = "info", msg = "request.info", macro = "request_info")]
//!     RequestInfo,
//!     #[tag(lvl = "warn", msg = "request.warn", macro = "request_warn")]
//!     RequestWarn,
//!     #[tag(lvl = "error", msg = "request.error", macro = "request_error")]
//!     RequestError,
//!     #[tag(lvl = "trace", msg = "security.access", icon = '🔓', macro = "security_access")]
//!     SecurityAccess,
//!     #[tag(lvl = "info", msg = "security.info", icon = '🔒', macro = "security_info")]
//!     SecurityInfo,
//!     #[tag(lvl = "error", msg = "security.critical", icon = '🔐', macro = "security_critical")]
//!     SecurityCritical,
//!     #[tag(lvl = "trace", msg = "filter.trace", macro = "filter_trace")]
//!     FilterTrace,
//!     #[tag(lvl = "info", msg = "filter.info", macro = "filter_info")]
//!     FilterInfo,
//!     #[tag(lvl = "warn", msg = "filter.warn", macro = "filter_warn")]
//!     FilterWarn,
//!     #[tag(lvl = "error", msg = "filter.error", macro = "filter_error")]
//!     FilterError,
//! }
//! ```
use crate::cfg_json;
use tracing::Level;

/// A type that can tag events with custom messages.
///
/// # Safety
///
/// This trait is unsafe to implement as the method signatures are subject to
/// change. Instead, [derive][tracing_forest_macros::Tag] it to ensure a correct
/// implementation.
///
/// See [module level documentation][self] for how to use [`Tag`]s.
pub unsafe trait Tag: 'static {
    #[doc(hidden)]
    fn as_field(&self) -> u64;

    #[doc(hidden)]
    fn from_field(value: u64) -> TagData;
}

pub(crate) type TagParser = fn(u64) -> TagData;

/// The type that all tags resolve to once collected.
#[derive(Debug, Clone, Copy)]
pub struct TagData {
    /// Minimalistic message denoting the tag kind.
    pub message: &'static str,
    /// Icon associated with the tag kind.
    pub icon: char,
}

impl From<Level> for TagData {
    fn from(level: Level) -> Self {
        match level {
            Level::TRACE => TagData {
                message: "trace",
                icon: crate::private::TRACE_ICON,
            },
            Level::DEBUG => TagData {
                message: "debug",
                icon: crate::private::DEBUG_ICON,
            },
            Level::INFO => TagData {
                message: "info",
                icon: crate::private::INFO_ICON,
            },
            Level::WARN => TagData {
                message: "warn",
                icon: crate::private::WARN_ICON,
            },
            Level::ERROR => TagData {
                message: "error",
                icon: crate::private::ERROR_ICON,
            },
        }
    }
}

cfg_json! {
    use serde::{Serialize, Serializer};

    impl Serialize for TagData {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(self.message)
        }
    }
}
