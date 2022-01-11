use tracing::{info, trace_span};
use tracing_forest::uuid_trace_span;
use tracing_subscriber::Registry;

mod uuid_tests {
    use super::*;
    use uuid::Uuid;

    #[tracing_forest::test]
    fn test_get_uuid() {
        trace_span!("first").in_scope(|| {
            let _ = tracing_forest::id::<Registry>();
        });
    }

    #[tracing_forest::test]
    fn test_set_get_uuid() {
        let id = Uuid::new_v4();
        info!("Using id: {}", id);
        uuid_trace_span!(id, "my_span").in_scope(|| {
            let span_id = tracing_forest::id::<Registry>();
            assert_eq!(id, span_id);
        });
    }

    #[tracing_forest::test]
    #[should_panic]
    fn test_get_uuid_not_in_span() {
        let _ = tracing_forest::id::<Registry>();
    }

    #[tracing_forest::test]
    #[should_panic]
    fn test_get_uuid_not_in_span_after_span() {
        trace_span!("in a span").in_scope(|| {
            let _ = tracing_forest::id::<Registry>();
        });
        let _ = tracing_forest::id::<Registry>();
    }

    #[tracing_forest::test]
    fn test_stack_of_spans() {
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();

        // Explicitly pass `first_id` into a new span
        uuid_trace_span!(first_id, "first").in_scope(|| {
            // Check that the ID we passed in is the current ID
            assert_eq!(first_id, tracing_forest::id::<Registry>());

            // Open another span, implicitly adopting the parent ID
            trace_span!("first inner").in_scope(|| {
                // Check that the ID was adopted
                assert_eq!(first_id, tracing_forest::id::<Registry>());

                // Open another span, explicitly passing in a new ID
                uuid_trace_span!(second_id, "second").in_scope(|| {
                    // Check that the second ID was set
                    assert_eq!(second_id, tracing_forest::id::<Registry>());
                });

                // Now that `second` has closed, check that `first_id` is back
                assert_eq!(first_id, tracing_forest::id::<Registry>());
            });

            // `first_id` should still be the current ID
            assert_eq!(first_id, tracing_forest::id::<Registry>());
        });
    }

    #[tracing_forest::test]
    fn test_small_stack_of_spans() {
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();

        // Explicitly pass `first_id` into a new span
        uuid_trace_span!(first_id, "first").in_scope(|| {
            // Check that the ID we passed in is the current ID
            assert_eq!(first_id, tracing_forest::id::<Registry>());

            // Open another span, explicitly passing in a new ID
            uuid_trace_span!(second_id, "second").in_scope(|| {
                // Check that the second ID was set
                assert_eq!(second_id, tracing_forest::id::<Registry>());
            });

            // Now that `second` has closed, check that `first_id` is back
            assert_eq!(first_id, tracing_forest::id::<Registry>());
        });
    }

    #[tracing_forest::test]
    fn test_get_many_times() {
        trace_span!("first").in_scope(|| {
            let _ = tracing_forest::id::<Registry>();
            let _ = tracing_forest::id::<Registry>();
            let _ = tracing_forest::id::<Registry>();
        })
    }

    #[tracing_forest::test]
    fn test_uuid_span_macros() {
        let uuid = Uuid::new_v4();
        tracing_forest::uuid_trace_span!(uuid, "my span").in_scope(|| {
            tracing::trace!("hello");
        });
        tracing_forest::uuid_trace_span!(uuid, "my span", ans = 42).in_scope(|| {
            tracing::trace!("hello");
        });
        tracing_forest::uuid_debug_span!(uuid, "my span").in_scope(|| {
            tracing::debug!("hello");
        });
        tracing_forest::uuid_debug_span!(uuid, "my span", ans = 42).in_scope(|| {
            tracing::debug!("hello");
        });
        tracing_forest::uuid_info_span!(uuid, "my span").in_scope(|| {
            tracing::info!("hello");
        });
        tracing_forest::uuid_info_span!(uuid, "my span", ans = 42).in_scope(|| {
            tracing::info!("hello");
        });
        tracing_forest::uuid_warn_span!(uuid, "my span").in_scope(|| {
            tracing::warn!("hello");
        });
        tracing_forest::uuid_warn_span!(uuid, "my span", ans = 42).in_scope(|| {
            tracing::warn!("hello");
        });
        tracing_forest::uuid_error_span!(uuid, "my span").in_scope(|| {
            tracing::error!("hello");
        });
        tracing_forest::uuid_error_span!(uuid, "my span", ans = 42).in_scope(|| {
            tracing::error!("hello");
        });
    }
}

#[macro_use]
mod kanidm;
use kanidm::KanidmTag;

mod tag_tests {
    use super::*;

    #[tracing_forest::test(tag = "KanidmTag")]
    fn test_macros() {
        admin_info!("some info for the admin");
        request_error!("the request timed out");
        security_critical!("the db has been breached");
    }
}

mod attribute_tests {
    use super::*;

    #[tracing_forest::test(fmt = "json")]
    fn test_sync_early_return() {
        // tests that returning in the test doesn't prevent logging
        info!("a log");
        return;
    }

    #[tracing_forest::test]
    #[tokio::test]
    async fn test_async_early_return() {
        // test that returning in the test doesn't prevent
        // the processing thread handle from being awaited
        info!("a log");
        return;
    }

    #[tracing_forest::test]
    #[tokio::test]
    async fn test_tokio() {
        use std::time::Duration;
        use tokio::time::sleep;
        use tracing::Instrument;

        async {
            for i in 0..3 {
                info!("iter: {}", i);
                sleep(Duration::from_millis(200)).await;
            }
        }
        .instrument(trace_span!("takes awhile"))
        .await;
    }

    #[tracing_forest::test]
    #[tokio::test]
    async fn test_counting() {
        use std::time::Duration;
        use tokio::time::{interval, sleep};
        use tracing::{trace_span, Instrument};

        let pause = Duration::from_millis(100);
        let evens = async {
            let mut interval = interval(pause);
            for i in 0..3 {
                interval.tick().await;
                info!("{}", i * 2);
            }
        }
        .instrument(trace_span!("count evens"));

        let odds = async {
            sleep(pause / 2).await;
            let mut interval = interval(pause);
            for i in 0..3 {
                interval.tick().await;
                info!("{}", i * 2 + 1);
            }
        }
        .instrument(trace_span!("count odds"));

        let _ = tokio::join!(evens, odds);
    }

    #[tracing_forest::main]
    #[tokio::main(flavor = "current_thread")]
    #[test]
    async fn test_main() {
        info!("running as a main function");
    }

    #[tracing_forest::test]
    fn test_many_messages() {
        info!(
            message = "first field",
            message = "second field",
            "the message"
        );
    }

    #[tracing_forest::test]
    fn test_subscriber() {
        tracing::info!("Hello, world!");
    }

    #[tracing_forest::test]
    #[tokio::test]
    async fn test_tokio2() {
        tracing::info!("Hello from Tokio!");
    }
}
