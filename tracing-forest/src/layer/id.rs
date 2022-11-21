use crate::fail;
use crate::layer::OpenedSpan;
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, Registry};
use uuid::Uuid;

/// Gets the current [`Uuid`] of an entered span within a `tracing-forest`
/// subscriber.
///
/// # Examples
///
/// Passing in a `Uuid` to a span, and then retreiving it from within the span:
/// ```
/// # use tracing::{info, info_span};
/// # use uuid::Uuid;
/// # tracing_forest::init();
/// let uuid = Uuid::new_v4();
///
/// // Tracing's syntax allows us to omit the redundent naming of the field here
/// info_span!("my_span", %uuid).in_scope(|| {
///     assert!(tracing_forest::id() == uuid);
/// });
/// ```
///
/// # Panics
///
/// This function panics if there is no current subscriber, if the subscriber
/// isn't composed with a [`ForestLayer`], or if the subscriber isn't in a span.
///
/// [`ForestLayer`]: crate::layer::ForestLayer
#[must_use]
pub fn id() -> Uuid {
    tracing::dispatcher::get_default(|dispatch| {
        let subscriber = dispatch
            .downcast_ref::<Registry>()
            .unwrap_or_else(fail::subscriber_not_found);

        let current = subscriber.current_span();

        let id = current.id().expect(fail::NO_CURRENT_SPAN);

        subscriber
            .span(id)
            .expect(fail::SPAN_NOT_IN_CONTEXT)
            .extensions()
            .get::<OpenedSpan>()
            .expect(fail::NO_FOREST_LAYER)
            .uuid()
    })
}
