use tracing::{info, trace_span};
use tracing_forest::uuid_trace_span;

mod uuid_tests {
    use super::*;
    use uuid::Uuid;

    #[tracing_forest::test]
    fn test_get_uuid() {
        trace_span!("first").in_scope(|| {
            let _ = tracing_forest::id();
        });
    }

    #[tracing_forest::test]
    fn test_set_get_uuid() {
        let id = Uuid::new_v4();
        info!("Using id: {}", id);
        uuid_trace_span!(id, "my_span").in_scope(|| {
            let span_id = tracing_forest::id();
            assert_eq!(id, span_id);
        });
    }

    #[tracing_forest::test]
    #[should_panic]
    fn test_get_uuid_not_in_span_panic() {
        let _ = tracing_forest::id();
    }

    #[tracing_forest::test]
    #[should_panic]
    fn test_get_uuid_not_in_span_after_span_panic() {
        trace_span!("in a span").in_scope(|| {
            let _ = tracing_forest::id();
        });
        let _ = tracing_forest::id();
    }

    #[tracing_forest::test]
    fn test_stack_of_spans() {
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();

        tracing::info!("first_id: {}", first_id);
        tracing::info!("second_id: {}", second_id);

        // Explicitly pass `first_id` into a new span
        uuid_trace_span!(first_id, "first").in_scope(|| {
            // Check that the ID we passed in is the current ID
            assert_eq!(first_id, tracing_forest::id());

            // Open another span, explicitly passing in a new ID
            uuid_trace_span!(second_id, "second").in_scope(|| {
                // Check that the second ID was set
                assert_eq!(second_id, tracing_forest::id());
            });

            // `first_id` should still be the current ID
            assert_eq!(first_id, tracing_forest::id());
        });
    }

    #[tracing_forest::test]
    #[tokio::test]
    async fn test_instrument_with_uuid() {
        use tracing::Instrument;
        let id = Uuid::new_v4();
        info!("id: {}", id);
        async {
            assert_eq!(id, tracing_forest::id());
        }
        .instrument(uuid_trace_span!(id, "in_async"))
        .await;
    }

    #[tracing_forest::test]
    fn test_small_stack_of_spans() {
        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();

        // Explicitly pass `first_id` into a new span
        uuid_trace_span!(first_id, "first").in_scope(|| {
            // Check that the ID we passed in is the current ID
            assert_eq!(first_id, tracing_forest::id());

            // Open another span, explicitly passing in a new ID
            uuid_trace_span!(second_id, "second").in_scope(|| {
                // Check that the second ID was set
                assert_eq!(second_id, tracing_forest::id());
            });

            // Now that `second` has closed, check that `first_id` is back
            assert_eq!(first_id, tracing_forest::id());
        });
    }

    #[tracing_forest::test]
    fn test_get_many_times() {
        trace_span!("first").in_scope(|| {
            let _ = tracing_forest::id();
            let _ = tracing_forest::id();
            let _ = tracing_forest::id();
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

mod tag_tests {
    use tracing_forest::Tag;

    #[derive(Tag)]
    pub enum KanidmTag {
        #[tag(lvl = "info", msg = "admin.info", macro = "admin_info")]
        AdminInfo,
        #[tag(lvl = "error", msg = "request.error", macro = "request_error")]
        RequestError,
        #[tag(
            lvl = "error",
            msg = "security.critical",
            icon = '🔐',
            macro = "security_critical"
        )]
        SecurityCritical,
    }

    #[tracing_forest::test(tag = "KanidmTag")]
    fn test_macros() {
        admin_info!("some info for the admin");
        request_error!("the request timed out");
        security_critical!("the db has been breached");
    }

    #[derive(Tag)]
    pub enum MyTag {
        #[tag(lvl = "trace", msg = "simple")]
        Simple,
        #[tag(
            lvl = "error",
            msg = "all.features",
            icon = '🔐',
            macro = "all_features"
        )]
        AllFeatures,
    }

    #[tracing_forest::test(tag = "MyTag")]
    fn test_demo_macros() {
        use tracing_forest::Tag;
        tracing::trace!(__event_tag = MyTag::Simple.as_field(), "a simple log");
        all_features!("all the features wow");
    }

    #[tracing_forest::test]
    #[should_panic]
    fn test_tag_unset_panic() {
        all_features!("this should be panicking");
    }
}

mod attribute_tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::{interval, sleep};
    use tracing::Instrument;

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
                sleep(Duration::from_millis(50)).await;
            }
        }
        .instrument(trace_span!("takes awhile"))
        .await;
    }

    #[tokio::test]
    async fn test_counting() {
        tracing_forest::builder()
            .with_test_writer()
            .build_async()
            .with(tracing_subscriber::filter::LevelFilter::WARN)
            .in_future(async {
                let pause = Duration::from_millis(50);
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
            })
            .await
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

    #[tracing_forest::test]
    #[tokio::test]
    async fn test_immediate_async() {
        async {
            info!("logged first chronologically");
            info!(immediate = true, "logged second, but printed immediately");
        }
        .instrument(trace_span!("my_span"))
        .await
    }

    #[tracing_forest::test]
    fn test_immediate() {
        trace_span!("my_span").in_scope(|| {
            info!("first");
            info!("second");
            info!(immediate = true, "third, but immediately");
        })
    }

    #[tracing_forest::test(fmt = "json")]
    fn test_json_example() {
        info!(answer = 42, "my event");
    }

    // #[tracing_forest::test]
    // #[tokio::test]
    // async fn test_forever() {
    //     for iter in 0.. {
    //         sleep(Duration::from_secs(1)).await;
    //         info!("epoch: {}", iter);
    //     }
    // }
}

mod builder_tests {
    #[test]
    fn test_json() {
        tracing_forest::builder()
            .pretty()
            .build_blocking()
            .in_closure(|| {
                tracing::info!("Hello, world!");
            })
    }

    use tracing_forest::Tag;
    #[derive(Tag)]
    enum MyTag {
        #[tag(lvl = "info", msg = "greeting", macro = "greeting")]
        Greeting,
    }

    #[test]
    fn test_with_tag() {
        tracing_forest::builder()
            .with_tag::<MyTag>()
            .build_blocking()
            .in_closure(|| {
                greeting!("Hello, world!");
            })
    }

    #[derive(Tag)]
    enum BearTag {
        #[tag(lvl = "info", msg = "brown.bear", macro = "brown_bear")]
        BrownBear,
        #[tag(lvl = "warn", msg = "black.bear", macro = "black_bear")]
        BlackBear,
        #[tag(lvl = "error", msg = "polar.bear", macro = "polar_bear")]
        PolarBear,
    }

    #[test]
    fn main() {
        tracing_forest::builder()
            .pretty()
            .with_writer(std::io::stderr)
            .with_tag::<BearTag>()
            .build_blocking()
            .with(tracing_subscriber::filter::LevelFilter::WARN)
            .in_closure(|| {
                brown_bear!("if it's brown get down");
                black_bear!("if it's black fight back");
                polar_bear!("if it's white good night");
            })
    }
}
