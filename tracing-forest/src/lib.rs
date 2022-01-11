//! Preserve contextual coherence between trace data from concurrent tasks.
//!
//! # Overview
//!
//! [`tracing`] is a framework for instrumenting programs to collect structured
//! and async-aware diagnostics via the [`Subscriber`] trait. The
//! [`tracing-subscriber`] crate provides tools for composing [`Subscriber`]s
//! from smaller units. This crate extends [`tracing-subscriber`] by providing
//! [`TreeLayer`], a [`Layer`] that preserves contextual coherence of trace
//! data from concurrent tasks when logging.
//!
//! This crate is intended for programs running many nontrivial and disjoint
//! tasks concurrently, like server backends. Unlike other [`Subscriber`]s which
//! simply keep track of the context of an event, `tracing-forest` preserves
//! the contextual coherence when writing logs, allowing readers to easily trace
//! a sequence of events.
//!
//! `tracing-forest` provides many tools for authoring applications, but can
//! also be extended to author other libraries.
//!
//! [`tracing-subscriber`]: tracing_subscriber
//! [`Layer`]: tracing_subscriber::layer::Layer
//! [`Subscriber`]: tracing::subscriber::Subscriber
//! [processes]: crate::processor::Processor
//!
//! # Getting started
//!
//! The easiest way to get started is to enable all features. Do this by
//! enabling the `full` feature flag:
//! ```toml
//! tracing-forest = { version = "1", features = ["full"] }
//! ```
//!
//! Then, add the [`#[tracing_forest::main]`][attr_main] attribute to your main function:
//! ```
//! #[tracing_forest::main]
//! fn main() {
//!     // do stuff here...
//!     tracing::trace!("Hello, world!");
//! }
//! ```
//!
//! # Contextual Coherence in action
//!
//! This example contains two counters, one for evens and another for odds.
//! Running it will emit trace data at the root level, implying that all the
//! events are _independent_, meaning each trace will be processed and written
//! as it's collected. In this case, the logs will count up chronologically.
//! ```
//! # use std::time::Duration;
//! # use tokio::time::sleep;
//! # #[tracing_forest::test]
//! # #[tokio::test]
//! # async fn test_contextual_coherence() {
//! let evens = async {
//!     for i in 0..3 {
//!         tracing::info!("{}", i * 2);
//!         // pause for `odds`
//!         sleep(Duration::from_millis(100)).await;
//!     }
//! };
//!
//! let odds = async {
//!     // pause for `evens`
//!     sleep(Duration::from_millis(50)).await;
//!     for i in 0..3 {
//!         tracing::info!("{}", i * 2 + 1);
//!         // pause for `evens`
//!         sleep(Duration::from_millis(100)).await;
//!     }
//! };
//!
//! let _ = tokio::join!(evens, odds);
//! # }
//! ```
//! ```log
//! INFO     💬 [info]: 0
//! INFO     💬 [info]: 1
//! INFO     💬 [info]: 2
//! INFO     💬 [info]: 3
//! INFO     💬 [info]: 4
//! INFO     💬 [info]: 5
//! ```
//! [Instrumenting] the counters tells the [`TreeLayer`] to preserve the
//! contextual coherency of trace data from each task. Traces from the `even`
//! counter will be grouped, and traces from the `odd` counter will be grouped.
//! ```
//! # use std::time::Duration;
//! # use tokio::time::sleep;
//! # use tracing::Instrument;
//! # #[tracing_forest::main]
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn concurrent_counting() {
//! let evens = async {
//!     // ...
//! #   for i in 0..3 {
//! #       tracing::info!("{}", i * 2);
//! #       sleep(Duration::from_millis(100)).await;
//! #   }
//! }.instrument(tracing::trace_span!("counting_evens"));
//!
//! let odds = async {
//!     // ...
//! #   sleep(Duration::from_millis(50)).await;
//! #   for i in 0..3 {
//! #       tracing::info!("{}", i * 2 + 1);
//! #       sleep(Duration::from_millis(100)).await;
//! #   }
//! }.instrument(tracing::trace_span!("counting_odds"));
//!     
//! let _ = tokio::join!(evens, odds);
//! # }
//! ```
//! ```log
//! TRACE    counting_evens [ 409µs | 100.000% ]
//! INFO     ┕━ 💬 [info]: 0
//! INFO     ┕━ 💬 [info]: 2
//! INFO     ┕━ 💬 [info]: 4
//! TRACE    counting_odds [ 320µs | 100.000% ]
//! INFO     ┕━ 💬 [info]: 1
//! INFO     ┕━ 💬 [info]: 3
//! INFO     ┕━ 💬 [info]: 5
//! ```
//!
//! [join]: tokio::join
//! [instrumenting]: tracing::instrument::Instrument::instrument
//! [`Span`]: tracing::Span
//!
//! # Distinguishing event kinds with tags
//!
//! Beyond log levels, this crate provides the [`Tag`] trait, which allows
//! events to carry additional categorical data.
//!
//! Untagged logs aren't very informative at a glance.
//! ```log
//! INFO     💬 [info]: some info for the admin
//! ERROR    🚨 [error]: the request timed out
//! ERROR    🚨 [error]: the db has been breached
//! ```
//!
//! But with custom tags, they can be!
//! ```log
//! INFO     💬 [admin.info]: some info for the admin
//! ERROR    🚨 [request.error]: the request timed out
//! ERROR    🔐 [security.critical]: the db has been breached
//! ```
//!
//! See the [module level documentation][crate::tag] for details.
//!
//! # Attaching [`Uuid`]s to spans
//!
//! By enabling the `uuid` feature flag, spans created in the context of a
//! [`TreeLayer`] subscriber are assigned a [`Uuid`]. At the root level, the
//! ID is randomly generated, whereas child spans adopt the ID of their
//! parent.
//!
//! To retreive the [`Uuid`] of the current span, use the [`id`] function.
//! To set the [`Uuid`] of a new span, use [`uuid_span!`], or the shorthand
//! versions, [`uuid_trace_span!`], [`uuid_debug_span!`], [`uuid_info_span!`],
//! [`uuid_warn_span!`], or [`uuid_error_span!`].
//!
//! ## Example
//!
//! ```
//! # use tracing_forest::uuid_trace_span;
//! # use ::uuid::Uuid;
//! # #[tracing_forest::test]
//! # fn test_stack_of_spans() {
//! let first_id = Uuid::new_v4();
//! let second_id = Uuid::new_v4();
//!
//! // Explicitly pass `first_id` into a new span
//! uuid_trace_span!(first_id, "first").in_scope(|| {
//!     
//!     // Check that the ID we passed in is the current ID
//!     assert_eq!(first_id, tracing_forest::id::<Registry>());
//!     
//!     // Open another span, explicitly passing in a new ID
//!     uuid_trace_span!(second_id, "second").in_scope(|| {
//!             
//!         // Check that the second ID was set
//!         assert_eq!(second_id, tracing_forest::id::<Registry>());
//!     });
//!         
//!     // Now that `second` has closed, check that `first_id` is back
//!     assert_eq!(first_id, tracing_forest::id::<Registry>());
//! });
//! # }
//! ```
//!
//! # Feature flags
//!
//! `tracing-forest` uses feature flags to reduce dependencies in your code.
//!
//! * `full`: Enables all features listed below.
//! * `uuid`: Enables spans to carry operation IDs.
//! * `chrono`: Enables timestamps on trace data.
//! * `smallvec`: Enables some performance optimizations.
//! * `sync`: Enables the [`AsyncProcessor`] type.
//! * `json`: Enables JSON formatting for logs.
//! * `derive`: Enables [`#[derive(Tag)]`][derive] for making custom [tag] types.
//! * `attributes`: Enables the [`#[tracing_forest::test]`][attr_test] and
//! [`#[tracing_forest::main]`][attr_main] attributes.
//!
//! [`Uuid`]: ::uuid::Uuid
//! [`AsyncProcessor`]: crate::processor::sync::AsyncProcessor
//! [derive]: tracing_forest_macros::Tag
//! [attr_test]: tracing_forest_macros::test
//! [attr_main]: tracing_forest_macros::main

pub mod formatter;
pub mod layer;
pub mod processor;
pub mod tag;
#[doc(hidden)]
#[macro_use]
mod cfg;
#[doc(hidden)]
#[cfg(feature = "json")]
mod ser;
#[cfg(feature = "uuid")]
mod uuid;
#[macro_use]
mod macros;
pub(crate) mod fail;

// Items that are required for macros but not intended for public API
#[doc(hidden)]
pub mod private {
    pub use crate::tag::{unrecognized_tag_id, TagData};
    #[cfg(feature = "uuid")]
    pub use crate::uuid::into_u64_pair;
    pub use tracing::subscriber::set_default;
    pub use tracing_subscriber::{fmt::TestWriter, Layer, Registry};
    pub const TRACE_ICON: char = '📍';
    pub const DEBUG_ICON: char = '🐛';
    pub const INFO_ICON: char = '💬';
    pub const WARN_ICON: char = '🚧';
    pub const ERROR_ICON: char = '🚨';
}

// TODO:
// * [x] Organize processors
//      - change `proc` back to `processor`
//      - break into modules?
// * [x] Remove P generic from TreeLayer, it's not needed
// * [x] Rebrand to `tracing-forest`
// * [x] Pretty formatter (basically refactor the original)
// * [ ] Convert JSON formatting to generic serialization
// * [x] Write proc macros for `tracing_forest::test` and `tracing_forest::main`
// * [ ] Write docs
// *   [ ] crate-wide docs
// *   [ ] proc macros

pub use crate::layer::TreeLayer;
pub use crate::processor::blocking::blocking;
#[cfg(feature = "sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "sync")))]
pub use crate::processor::sync::async_spawn;
pub use crate::processor::Processor;
pub use crate::tag::Tag;
#[cfg(feature = "uuid")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
pub use crate::uuid::id;

/// Derive macro generating an implementation of the [`Tag`] trait.
///
/// See [`tag` module][crate::tag] for details on how to use tags in logs.
///
/// # Note:
///
/// This is targetted towards application authors. Each application should
/// derive on exactly one `enum` type with variants associated with any type of
/// event that could occur. See below for examples.
///
/// # The `#[tag(..)]` Attribute
///
/// This macro uses the `#[tag(..)]` attribute to determine what the displayed
/// tag should look like. `struct` types must use this attribute above their
/// definition, and `enum` types must use this attribute on every variant.
///
/// There are two components that make up a tag: an icon character, and a
/// message. The icon is typically used only for pretty printing, while the
/// message provides a minimalist description of what kind of event happened.
///
/// Tag variants are often associated with log levels, which have predefined
/// icons. You can opt into these defaults by using the shorthand syntax, which
/// accepts `trace`, `debug`, `info`, `warn`, and `error`. The message follows
/// in the string literal.
/// ```
/// # use tracing_forest::Tag;
/// #[derive(Tag)]
/// enum MyTag {
///     // `trace` keyword defaults to 📍
///     #[tag(trace: "request.trace")]
///     RequestTrace,
/// }
/// ```
/// To use a custom icon, use the `custom` keyword:
/// ```
/// # use tracing_forest::Tag;
/// #[derive(Tag)]
/// enum MyTag {
///     #[tag(custom('🤯'): "security.breach")]
///     SecurityBreach,
/// }
/// ```
///
/// # Examples
///
/// ```
/// # use tracing_forest::Tag;
/// #[derive(Tag)]
/// pub enum KanidmTag {
///     #[tag(debug: "admin.debug")]
///     AdminDebug,
///     #[tag(info: "admin.info")]
///     AdminInfo,
///     #[tag(warn: "admin.warn")]
///     AdminWarn,
///     #[tag(trace: "request.trace")]
///     RequestTrace,
///     #[tag(error: "request.error")]
///     RequestError,
///     #[tag(custom('🔓'): "security.access")]
///     SecurityAccess,
///     #[tag(custom('🔐'): "security.critical")]
///     SecurityCritical,
/// }
/// ```
///
#[cfg(feature = "derive")]
pub use tracing_forest_macros::Tag;

/// Marks test to run in the context of a [`TreeLayer`] subscriber,
/// suitable to test environment.
///
/// # Examples
///
/// ### Default behavior
///
/// By default, logs are pretty-printed to stdout.
///
/// ```
/// #[tracing_forest::test]
/// fn test_subscriber() {
///     tracing::info!("Hello, world!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello, world!
/// ```
///
/// ### Tags and formatting
///
/// Custom tags and formatting can be configured with the `tag` and `fmt`
/// arguments respectively. Currently, only `"pretty"` and `"json"` are
/// supported formats, and tag types must implement the [`Tag`] trait.
///
/// ```
/// # use tracing_forest::Tag;
/// # #[derive(Tag)]
/// # enum MyTag {}
/// #[tracing_forest::test(tag = "MyTag", fmt = "json")]
/// fn test_subscriber_config() {
///     tracing::info!("Hello in JSON");
/// }
/// ```
/// ```json
/// {
///   "level": "INFO",
///   "kind": {
///     "Event": {
///       "tag": null,
///       "message": "Hello in JSON",
///       "fields": {}
///     }
///   }
/// }
/// ```
///
/// ### Using with Tokio runtime
///
/// The attribute can also be proceeded by the [`#[tokio::test]`][tokio::test]
/// attribute to run the test in the context of an async runtime.
/// ```
/// #[tracing_forest::test]
/// #[tokio::test]
/// async fn test_tokio() {
///     tracing::info!("Hello from Tokio!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello from Tokio!
/// ```
#[cfg(feature = "attributes")]
pub use tracing_forest_macros::test;

/// Marks function to run in the context of a [`TreeLayer`] subscriber.
///
/// # Examples
///
/// ### Default behavior
///
/// By default, logs are pretty-printed to stdout.
///
/// ```
/// #[tracing_forest::main]
/// fn main() {
///     tracing::info!("Hello, world!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello, world!
/// ```
///
/// ### Tags and formatting
///
/// Custom tags and formatting can be configured with the `tag` and `fmt`
/// arguments respectively. Currently, only `"pretty"` and `"json"` are
/// supported formats, and tag types must implement the [`Tag`] trait.
///
/// ```
/// # use tracing_forest::Tag;
/// # #[derive(Tag)]
/// # enum MyTag {}
/// #[tracing_forest::main(tag = "MyTag", fmt = "json")]
/// fn main() {
///     tracing::info!("Hello in JSON");
/// }
/// ```
/// ```json
/// {
///   "level": "INFO",
///   "kind": {
///     "Event": {
///       "tag": null,
///       "message": "Hello in JSON",
///       "fields": {}
///     }
///   }
/// }
/// ```
///
/// ### Using with Tokio runtime
///
/// The attribute can also be proceeded by the [`#[tokio::main]`][tokio::main]
/// attribute to run the function in the context of an async runtime.
/// ```
/// #[tracing_forest::main]
/// #[tokio::main(flavor = "current_thread")]
/// async fn main() {
///     tracing::info!("Hello from Tokio!");
/// }
/// ```
/// ```log
/// INFO     💬 [info]: Hello from Tokio!
/// ```
#[cfg(feature = "attributes")]
pub use tracing_forest_macros::main;
