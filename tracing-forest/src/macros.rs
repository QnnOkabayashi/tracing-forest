/// Creates a new [`Span`] with a specified [`Uuid`].
///
/// This macro provides a useful subset of the functionality of `tracing`s
/// [`span!`] macro, except it accepts a [`Uuid`] as the first argument to set
/// as the span's ID.
///
/// # Examples
///
/// Creating a new span with a specified [`Uuid`]:
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::uuid_span;
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// let span = uuid_span!(id, Level::TRACE, "my span");
/// let _enter = span.enter();
/// // do work inside the span...
/// ```
///
/// [`Span`]: tracing::Span
/// [`Uuid`]: uuid::Uuid
/// [`span!`]: tracing::span!
#[macro_export]
macro_rules! uuid_span {
    ($uuid:expr, $lvl:expr, $name:expr, $( $fields:tt )*) => {{
        let (__uuid_msb, __uuid_lsb) = ::tracing_forest::private::into_u64_pair(&$uuid);
        ::tracing::span!($lvl, $name, __uuid_msb, __uuid_lsb, $( $fields )*)
    }};
    ($uuid:expr, $lvl:expr, $name:expr) => {
        ::tracing_forest::uuid_span!($uuid, $lvl, $name,)
    };
}

/// Creates a new [`Span`] at the trace level with a specified [`Uuid`].
///
/// This macro provides a useful subset of the functionality of `tracing`s
/// [`trace_span!`] macro, except is accepts a [`Uuid`] as the first argument
/// to set the span's ID.
///
/// # Examples
///
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::{uuid_span, uuid_trace_span};
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// uuid_trace_span!(id, "my_span");
/// // is equivalent to:
/// uuid_span!(id, Level::TRACE, "my_span");
/// ```
/// Creating a new trace span with a specified [`Uuid`]:
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::uuid_trace_span;
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// let span = uuid_trace_span!(id, "my_span");
/// let _enter = span.enter();
/// // do work inside the span...
/// ```
///
/// [`Span`]: tracing::Span
/// [`Uuid`]: uuid::Uuid
/// [`trace_span!`]: tracing::trace_span!
#[macro_export]
macro_rules! uuid_trace_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::TRACE, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_trace_span!($uuid, $name,)
    };
}

/// Creates a new [`Span`] at the debug level with a specified [`Uuid`].
///
/// This macro provides a useful subset of the functionality of `tracing`s
/// [`debug_span!`] macro, except is accepts a [`Uuid`] as the first argument
/// to set the span's ID.
///
/// # Examples
///
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::{uuid_span, uuid_debug_span};
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// uuid_debug_span!(id, "my_span");
/// // is equivalent to:
/// uuid_span!(id, Level::DEBUG, "my_span");
/// ```
/// Creating a new trace span with a specified [`Uuid`]:
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::uuid_debug_span;
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// let span = uuid_debug_span!(id, "my_span");
/// let _enter = span.enter();
/// // do work inside the span...
/// ```
///
/// [`Span`]: tracing::Span
/// [`Uuid`]: uuid::Uuid
/// [`debug_span!`]: tracing::debug_span!
#[macro_export]
macro_rules! uuid_debug_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::DEBUG, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_debug_span!($uuid, $name,)
    };
}

/// Creates a new [`Span`] at the info level with a specified [`Uuid`].
///
/// This macro provides a useful subset of the functionality of `tracing`s
/// [`info_span!`] macro, except is accepts a [`Uuid`] as the first argument
/// to set the span's ID.
///
/// # Examples
///
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::{uuid_span, uuid_info_span};
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// uuid_info_span!(id, "my_span");
/// // is equivalent to:
/// uuid_span!(id, Level::INFO, "my_span");
/// ```
/// Creating a new trace span with a specified [`Uuid`]:
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::uuid_info_span;
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// let span = uuid_info_span!(id, "my_span");
/// let _enter = span.enter();
/// // do work inside the span...
/// ```
///
/// [`Span`]: tracing::Span
/// [`Uuid`]: uuid::Uuid
/// [`info_span!`]: tracing::info_span!
#[macro_export]
macro_rules! uuid_info_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::INFO, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_info_span!($uuid, $name,)
    };
}

/// Creates a new [`Span`] at the warn level with a specified [`Uuid`].
///
/// This macro provides a useful subset of the functionality of `tracing`s
/// [`warn_span!`] macro, except is accepts a [`Uuid`] as the first argument
/// to set the span's ID.
///
/// # Examples
///
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::{uuid_span, uuid_warn_span};
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// uuid_warn_span!(id, "my_span");
/// // is equivalent to:
/// uuid_span!(id, Level::WARN, "my_span");
/// ```
/// Creating a new trace span with a specified [`Uuid`]:
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::uuid_warn_span;
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// let span = uuid_warn_span!(id, "my_span");
/// let _enter = span.enter();
/// // do work inside the span...
/// ```
///
/// [`Span`]: tracing::Span
/// [`Uuid`]: uuid::Uuid
/// [`warn_span!`]: tracing::warn_span!
#[macro_export]
macro_rules! uuid_warn_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::WARN, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_warn_span!($uuid, $name,)
    };
}

/// Creates a new [`Span`] at the error level with a specified [`Uuid`].
///
/// This macro provides a useful subset of the functionality of `tracing`s
/// [`error_span!`] macro, except is accepts a [`Uuid`] as the first argument
/// to set the span's ID.
///
/// # Examples
///
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::{uuid_span, uuid_error_span};
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// uuid_error_span!(id, "my_span");
/// // is equivalent to:
/// uuid_span!(id, Level::ERROR, "my_span");
/// ```
/// Creating a new trace span with a specified [`Uuid`]:
/// ```
/// # use uuid::Uuid;
/// # use tracing_forest::uuid_error_span;
/// # use tracing::Level;
/// let id = Uuid::new_v4();
/// let span = uuid_error_span!(id, "my_span");
/// let _enter = span.enter();
/// // do work inside the span...
/// ```
///
/// [`Span`]: tracing::Span
/// [`Uuid`]: uuid::Uuid
/// [`error_span!`]: tracing::error_span!
#[macro_export]
macro_rules! uuid_error_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::ERROR, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_error_span!($uuid, $name,)
    };
}
