//! Trait for tagging events with custom messages and levels.
//!
//! # Custom macros for applications
//!
//! The first step to using custom tags is to define an `enum` type with
//! variants for each possible error, and then [deriving] the [`Tag`] trait.
//!
//! ```
//! # use tracing_forest::Tag;
//! #[derive(Tag)]
//! pub enum MyTag {
//!     #[tag(custom('🔐'): "security.critical")]
//!     SecurityCritical,
//! }
//! ```
//! Note that tagging works by passing in a field-value pair to [`tracing`]s log
//! macros with the field name `__event_tag`, meaning that this is a reserved
//! name that must not be used for other values, and may panic otherwise.
//!
//! The most convenient way to log events with that tag is to write a custom
//! macro for each variant of your [`Tag`]. Use the following example as a
//! template:
//! ```
//! macro_rules! security_critical {
//!           /* ^ your macro name */
//!     ($tokens:tt) => {
//!         ::tracing::error!(
//!                 /* ^ the associated log level */
//!             __event_tag = ::tracing_forest::Tag::as_field(&$crate::MyTag::SecurityCritical),
//!                                                         /* ^ path to your type */
//!             $tokens
//!         )
//!     };
//! }
//! ```
//!
//! Then, use the custom macro throughout your application.
//! ```
//! # use tracing_forest::Tag;
//! # #[derive(Tag)]
//! # pub enum MyTag {
//! #     #[tag(custom('🔐'): "security.critical")]
//! #     SecurityCritical,
//! # }
//! # macro_rules! security_critical {
//! #           /* ^ your macro name */
//! #     ($tokens:tt) => {
//! #         ::tracing::error!(
//! #                 /* ^ the associated log level */
//! #             __event_tag = ::tracing_forest::Tag::as_field(&$crate::MyTag::SecurityCritical),
//! #                                                         /* ^ path to your type */
//! #             $tokens
//! #         )
//! #     };
//! # }
//! #[tracing_forest::main(tag = "MyTag")]
//! fn main() {
//!     security_critical!("the db has been breached");
//! }
//! ```
//! ```log
//! ERROR    🔐 [security.critical]: the db has been breached
//! ```
//!
//! ## Note:
//!
//! Although the [`Tag`] trait is unsafe to implement, it is guaranteed that
//! `Tag::as_field` will retain the same name and input parameter `&self`.
//!
//! [deriving]: tracing_forest_macros::Tag
use crate::cfg_json;
use crate::fail;

/// A type that can tag events with custom messages.
///
/// This trait is unsafe to implement as the method signatures are subject to
/// change. Instead, [derive][tracing_forest_macros::Tag] it to ensure a correct
/// implementation.
///
/// See [module level documentation][self] for how to use [`Tag`]s.
// There's nothing unsafe about this function other than if the implementor
// makes a mistake, tags may not map to the correct TagData and there's nothing
// to catch that. Using the derive macro guaranteeds a correct implementation.
pub unsafe trait Tag: 'static {
    #[doc(hidden)]
    fn as_field(&self) -> u64;

    #[doc(hidden)]
    fn from_field(value: u64) -> TagData;
}

pub(crate) type TagParser = fn(u64) -> TagData;

#[doc(hidden)]
pub fn unrecognized_tag_id(id: u64) -> ! {
    fail::unrecognized_tag_id(id)
}

/// The type that all tags resolve to once collected.
#[derive(Debug)]
pub struct TagData {
    /// Minimalistic message denoting the tag category.
    pub message: &'static str,
    /// Icon associated with the category.
    pub icon: char,
}

cfg_json! {
    use serde::{Serialize, Serializer};

    impl Serialize for TagData {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(self.message)
        }
    }
}

pub(crate) enum NoTag {}

unsafe impl Tag for NoTag {
    fn as_field(&self) -> u64 {
        match *self {}
    }

    fn from_field(value: u64) -> TagData {
        fail::tag_unset(value)
    }
}
