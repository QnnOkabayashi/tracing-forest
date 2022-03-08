//! [![github-img]][github-url] [![crates-img]][crates-url] [![docs-img]][docs-url]
//!
//! [github-url]: https://github.com/QnnOkabayashi/tracing-forest
//! [crates-url]: https://crates.io/crates/tracing-forest
//! [docs-url]: crate
//! [github-img]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-img]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-img]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K
//!
//! Preserve contextual coherence among trace data from concurrent tasks.
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
//! a sequence of events from the same task.
//!
//! `tracing-forest` provides many tools for authoring applications, but can
//! also be extended to author other libraries.
//!
//! [`tracing-subscriber`]: tracing_subscriber
//! [`Layer`]: tracing_subscriber::layer::Layer
//! [`Subscriber`]: tracing::subscriber::Subscriber
//!
//! # Getting started
//!
//! The easiest way to get started is to enable all features. Do this by
//! adding the following to your `Cargo.toml` file:
//! ```toml
//! tracing-forest = { version = "0.1", features = ["full"] }
//! ```
//! Then, add the [`#[tracing_forest::main]`][attr_main] attribute to your main function:
//! ```
//! # #[allow(clippy::needless_doctest_main)]
//! #[tracing_forest::main]
//! #[tokio::main]
//! async fn main() {
//!     // do stuff here...
//!     tracing::trace!("Hello, world!");
//! }
//! ```
//! For more configuration options, see the
//! [`builder` module documentation][mod@builder].
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
//! # #[tokio::test]
//! # async fn test_contextual_coherence() {
//! # tracing_forest::new().on_registry().on(async {
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
//! # }).await;
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
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn concurrent_counting() {
//! # tracing_forest::new().on_registry().on(async {
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
//! # }).await;
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
//! Although the numbers were logged chronologically, they appear grouped in the
//! spans they originated from.
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
//! INFO     💬 [admin_info]: some info for the admin
//! ERROR    🚨 [request_error]: the request timed out
//! ERROR    🔐 [security_critical]: the db has been breached
//! ```
//!
//! See the [`tag` module documentation][crate::tag] for details.
//!
//! # Attaching `Uuid`s to spans
//!
//! By enabling the `uuid` feature flag, spans created in the context of a
//! [`TreeLayer`] subscriber are assigned a `Uuid`. At the root level, the uuid
//! is randomly generated, whereas child spans adopt the uuid of their parent.
//!
//! ### Retreiving the current `Uuid`
//!
//! To retreive the uuid of the current span, use the [`id`] function.
//!
//! ### Initializing a span with a specific `Uuid`
//!
//! To set the `Uuid` of a new span, use [`uuid_span!`], or the shorthand
//! versions, [`uuid_trace_span!`], [`uuid_debug_span!`], [`uuid_info_span!`],
//! [`uuid_warn_span!`], or [`uuid_error_span!`].
//!
//! ## Examples
//!
//! Passing in custom `Uuid`s to nested spans:
//! ```
//! # use tracing_forest::uuid_trace_span;
//! # use ::uuid::Uuid;
//! # #[tracing_forest::test]
//! # fn test_stack_of_spans() {
//! let first_id = Uuid::new_v4();
//! let second_id = Uuid::new_v4();
//!
//! tracing::info!("first_id: {}", first_id);
//! tracing::info!("second_id: {}", second_id);
//!
//! // Explicitly pass `first_id` into a new span
//! uuid_trace_span!(first_id, "first").in_scope(|| {
//!
//!     // Check that the ID we passed in is the current ID
//!     assert_eq!(first_id, tracing_forest::id());
//!
//!     // Open another span, explicitly passing in a new ID
//!     uuid_trace_span!(second_id, "second").in_scope(|| {
//!
//!         // Check that the second ID was set
//!         assert_eq!(second_id, tracing_forest::id());
//!     });
//!
//!     // `first_id` should still be the current ID
//!     assert_eq!(first_id, tracing_forest::id());
//! });
//! # }
//! ```
//! ```log
//! 00000000-0000-0000-0000-000000000000 INFO     💬 [info]: first_id: 9f197cc3-b340-4df6-be53-4ab742a3c586
//! 00000000-0000-0000-0000-000000000000 INFO     💬 [info]: second_id: d552ecfa-a568-4b68-9e68-a4f1f7918579
//! 9f197cc3-b340-4df6-be53-4ab742a3c586 TRACE    first [ 76.6µs | 80.206% / 100.000% ]
//! d552ecfa-a568-4b68-9e68-a4f1f7918579 TRACE    ┕━ second [ 15.2µs | 19.794% ]
//! ```
//!
//! Instrumenting a future with a span using a custom `Uuid`:
//! ```
//! # use tracing::{info, Instrument};
//! # use ::uuid::Uuid;
//! # #[tracing_forest::test]
//! # #[tokio::test]
//! # async fn test_instrument_with_uuid() {
//! let id = Uuid::new_v4();
//! info!("id: {}", id);
//!
//! async {
//!     assert_eq!(id, tracing_forest::id());
//! }.instrument(uuid_trace_span!(id, "in_async")).await;
//! # }
//! ```
//! ```log
//! 00000000-0000-0000-0000-000000000000 INFO     💬 [info]: id: 5aacc2d4-f625-401b-9bb8-dc5c355fd31b
//! 5aacc2d4-f625-401b-9bb8-dc5c355fd31b TRACE    in_async [ 18.6µs | 100.000% ]
//! ```
//!
//! # Immediate logs
//!
//! This crate also provides functionality to immediately format and print logs
//! to stderr in the case of logs with high urgency. This can be done by setting
//! the `immediate` field to `true` in the trace data.
//!
//! ## Example
//!
//! ```
//! # use tracing::{info, trace_span};
//! # #[tracing_forest::test]
//! # fn test_immediate() {
//! trace_span!("my_span").in_scope(|| {
//!     info!("first");
//!     info!("second");
//!     info!(immediate = true, "third, but immediately");
//! })
//! # }
//! ```
//! ```log
//! 💬 IMMEDIATE 💬 INFO     my_span > third, but immediately
//! TRACE    my_span [ 163µs | 100.000% ]
//! INFO     ┕━ 💬 [info]: first
//! INFO     ┕━ 💬 [info]: second
//! INFO     ┕━ 💬 [info]: third, but immediately
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

pub mod printer;
pub mod processor;
pub mod tag;
pub mod tree;
#[macro_use]
mod cfg;
mod fail;
mod layer;

pub use layer::ForestLayer;
pub use printer::{printer, StringifyTree};
pub use processor::Processor;

cfg_sync! {
    pub mod builder;
    pub use builder::{capture, new};
}

cfg_uuid! {
    pub use layer::id;
}

cfg_attributes! {
    #[doc(inline)]
    pub use tracing_forest_macros::test;
    #[doc(inline)]
    pub use tracing_forest_macros::main;
}

cfg_derive! {
    #[doc(inline)]
    pub use tracing_forest_macros::Tag;
}

mod sealed {
    pub trait Sealed {}
}
