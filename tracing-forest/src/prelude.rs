//! The module re-exports a number of useful components that you may wish
//! to consume.

pub use tracing::Instrument;

pub use tracing::{
    debug,
    debug_span,
    error,
    error_span,
    event,
    info,
    info_span,
    span,
    trace,
    trace_span,
    warn,
    warn_span,
};

// As these macros are macro_export, they are published at the crate root.
/*
#[cfg(feature = "uuid")]
pub use crate::{
    uuid_debug_span,
    uuid_error_span,
    uuid_info_span,
    uuid_span,
    uuid_trace_span,
    uuid_warn_span,
};
*/

#[cfg(feature = "attributes")]
pub use tracing;
#[cfg(feature = "attributes")]
pub use tracing::instrument;

pub mod filter {
    pub use tracing_subscriber::filter::LevelFilter;
    #[cfg(feature = "env-filter")]
    pub use tracing_subscriber::filter::EnvFilter;
}

