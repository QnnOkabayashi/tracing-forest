/// Creates a new [`Span`], passing in a [`Uuid`].
///
/// This macro provides a useful subset of the functionality of [`tracing`]s
/// [`span!`] macro, except it accepts a [`Uuid`] as the first argument to set
/// as the spans ID.
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

#[macro_export]
macro_rules! uuid_trace_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::TRACE, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_trace_span!($uuid, $name,)
    };
}

#[macro_export]
macro_rules! uuid_debug_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::DEBUG, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_debug_span!($uuid, $name,)
    };
}

#[macro_export]
macro_rules! uuid_info_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::INFO, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_info_span!($uuid, $name,)
    };
}

#[macro_export]
macro_rules! uuid_warn_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::WARN, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_warn_span!($uuid, $name,)
    };
}

#[macro_export]
macro_rules! uuid_error_span {
    ($uuid:expr, $name:expr, $( $fields:tt )*) => {
        ::tracing_forest::uuid_span!($uuid, ::tracing::Level::ERROR, $name, $( $fields )*)
    };
    ($uuid:expr, $name:expr) => {
        ::tracing_forest::uuid_error_span!($uuid, $name,)
    };
}
