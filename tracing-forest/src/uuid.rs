use crate::fail;
use crate::layer::TreeSpanOpened;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Registry;
use uuid::Uuid;

// For internal macro usage only
#[doc(hidden)]
pub fn into_u64_pair(id: &Uuid) -> (u64, u64) {
    let bytes = id.as_bytes();
    let msb = (bytes[0] as u64) << 56
        | (bytes[1] as u64) << 48
        | (bytes[2] as u64) << 40
        | (bytes[3] as u64) << 32
        | (bytes[4] as u64) << 24
        | (bytes[5] as u64) << 16
        | (bytes[6] as u64) << 8
        | bytes[7] as u64;
    let lsb = (bytes[8] as u64) << 56
        | (bytes[9] as u64) << 48
        | (bytes[10] as u64) << 40
        | (bytes[11] as u64) << 32
        | (bytes[12] as u64) << 24
        | (bytes[13] as u64) << 16
        | (bytes[14] as u64) << 8
        | bytes[15] as u64;
    (msb, lsb)
}

// For internal macro usage only
#[doc(hidden)]
pub fn from_u64_pair(msb: u64, lsb: u64) -> Uuid {
    Uuid::from_bytes([
        (msb >> 56) as u8,
        (msb >> 48) as u8,
        (msb >> 40) as u8,
        (msb >> 32) as u8,
        (msb >> 24) as u8,
        (msb >> 16) as u8,
        (msb >> 8) as u8,
        msb as u8,
        (lsb >> 56) as u8,
        (lsb >> 48) as u8,
        (lsb >> 40) as u8,
        (lsb >> 32) as u8,
        (lsb >> 24) as u8,
        (lsb >> 16) as u8,
        (lsb >> 8) as u8,
        lsb as u8,
    ])
}

/// Gets the current [`Uuid`] of an entered span within a [`TreeLayer`]
/// subscriber.
///
/// # Examples
///
/// ```
/// # use tracing::trace_span;
/// # #[tracing_forest::main]
/// # fn main() {
/// trace_span!("my_span").in_scope(|| {
///     let id = tracing_forest::id();
///     tracing::info!("The current id is: {}", id);
/// })
/// # }
/// ```
///
/// # Panics
///
/// This function has many opportunities to panic, but each should be easily
/// preventable by the caller at compile time. It will panic if:
/// * There is no current subscriber.
/// * The current subscriber isn't in a span.
/// * The current span's ID isn't registered with the subscriber.
/// * The current subscriber isn't composed with a [`TreeLayer`].
///
/// [`TreeLayer`]: crate::layer::TreeLayer
#[must_use]
pub fn id() -> Uuid {
    tracing::dispatcher::get_default(|dispatch| {
        let subscriber = dispatch
            .downcast_ref::<Registry>()
            .unwrap_or_else(fail::subscriber_not_found::<Registry>);

        let current = subscriber.current_span();

        let id = current.id().unwrap_or_else(fail::no_current_span);

        subscriber
            .span(id)
            .unwrap_or_else(fail::span_not_in_context)
            .extensions()
            .get::<TreeSpanOpened>()
            .unwrap_or_else(fail::no_tree_layer)
            .uuid()
    })
}
