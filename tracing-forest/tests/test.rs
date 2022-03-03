#[cfg(feature = "sync")]
mod sync_tests {
    use rand::Rng;
    use tokio::time::{sleep, Duration};
    use tracing::{info, trace_span, Instrument};

    async fn sleep_rand() {
        sleep(Duration::from_millis(rand::thread_rng().gen_range(10..200))).await
    }

    #[tokio::test]
    async fn test_two_tasks_random_sleeps() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                let a = async {
                    async {
                        info!(client = %"a", "sent request");
                        sleep_rand().await;
                        info!(client = %"a", "received response");
                    }
                    .instrument(trace_span!("a request"))
                    .await;

                    sleep_rand().await;

                    async {
                        info!(client = %"a", "sending response");
                        sleep_rand().await;
                        info!(client = %"a", "response sent");
                    }
                    .instrument(trace_span!("a response"))
                    .await;
                }
                .instrument(trace_span!("a"));

                let b = async {
                    async {
                        info!(client = %"b", "sent request");
                        sleep_rand().await;
                        info!(client = %"b", "received response");
                    }
                    .instrument(trace_span!("b request"))
                    .await;

                    sleep_rand().await;

                    async {
                        info!(client = %"b", "sending response");
                        sleep_rand().await;
                        info!(client = %"b", "response sent");
                    }
                    .instrument(trace_span!("b response"))
                    .await;
                }
                .instrument(trace_span!("b"));

                let _ = tokio::join!(a, b);
            })
            .await;

        assert!(logs.len() == 2);

        for tree in logs {
            let span = tree.span()?;
            assert!(span.children().len() == 2);

            for (child, which) in span.children().iter().zip(["request", "response"]) {
                let inner = child.span()?;
                assert!(inner.name() == format!("{} {}", span.name(), which));
                assert!(inner.children().len() == 2);
                for child in inner.children().iter() {
                    let event = child.event()?;
                    assert!(event.fields()[0].value == span.name());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_filtering() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .on_registry()
            .with(tracing_subscriber::filter::LevelFilter::INFO)
            .on(async {
                tracing::trace!("unimportant information");
                tracing::info!("important information");
            })
            .await;

        assert!(logs.len() == 1);

        let info = logs[0].event()?;

        assert!(info.message() == "important information");

        Ok(())
    }

    #[tokio::test]
    async fn test_doc() {
        tracing_forest::new()
            .set_global(false)
            .on_registry()
            .on(async {
                tracing::info!("Hello, world!");

                tracing::info_span!("my_span").in_scope(|| {
                    tracing::info!("Relevant information");
                })
            })
            .await;
    }
}

#[cfg(feature = "uuid")]
mod uuid_tests {
    use tokio::time::{sleep, Duration};
    use tracing::trace_span;
    use tracing::{info, Instrument};
    use tracing_forest::uuid_trace_span;
    use uuid::Uuid;

    #[tokio::test(flavor = "current_thread")]
    async fn test_panic_get_id_not_in_span() {
        tracing_forest::capture()
            .on_registry()
            .on(async {
                std::panic::set_hook(Box::new(|_| {}));
                assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_panic_get_id_not_in_subscriber() {
        assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_panic_get_id_after_close() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                let uuid = Uuid::new_v4();
                uuid_trace_span!(uuid, "in a span").in_scope(|| {
                    let _ = tracing_forest::id();
                });
                std::panic::set_hook(Box::new(|_| {}));
                assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
            })
            .await;

        assert!(logs.len() == 1);

        let span = logs[0].span()?;
        assert!(span.name() == "in a span");
        assert!(span.children().len() == 0);

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_two_stacks_of_spans() -> Result<(), Box<dyn std::error::Error>> {
        // Tests that two task running concurrently truely do not interfere
        // with each other's span data.
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                let a = async {
                    let first_id = Uuid::new_v4();
                    let second_id = Uuid::new_v4();
                    uuid_trace_span!(first_id, "a_span").in_scope(|| {
                        assert_eq!(first_id, tracing_forest::id());
                        uuid_trace_span!(second_id, "a_span2").in_scope(|| {
                            assert_eq!(second_id, tracing_forest::id());
                        });
                        assert_eq!(first_id, tracing_forest::id());
                    });
                };

                let b = async {
                    let first_id = Uuid::new_v4();
                    let second_id = Uuid::new_v4();
                    uuid_trace_span!(first_id, "b_span").in_scope(|| {
                        assert_eq!(first_id, tracing_forest::id());
                        uuid_trace_span!(second_id, "b_span2").in_scope(|| {
                            assert_eq!(second_id, tracing_forest::id());
                        });
                        assert_eq!(first_id, tracing_forest::id());
                    });
                };

                let _ = tokio::join!(a, b);
            })
            .await;

        assert!(logs.len() == 2);

        for tree in logs {
            let span = tree.span()?;
            assert!(span.children().len() == 1);
            let span2 = span.children()[0].span()?;
            assert!(span2.name() == format!("{}2", span.name()));
        }

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_get_many_times() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                async {
                    let a = tracing_forest::id();
                    let b = tracing_forest::id();
                    assert_eq!(a, b);
                }
                .instrument(trace_span!("my_span"))
                .await;
            })
            .await;

        assert!(logs.len() == 1);

        let span = logs[0].span()?;
        assert!(span.is_empty());
        assert!(span.name() == "my_span");

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_uuid_span_macros() -> Result<(), Box<dyn std::error::Error>> {
        let uuid = Uuid::new_v4();
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                tracing_forest::uuid_trace_span!(uuid, "my_span").in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_trace_span!(uuid, "my_span", ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_debug_span!(uuid, "my_span").in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_debug_span!(uuid, "my_span", ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_info_span!(uuid, "my_span").in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_info_span!(uuid, "my_span", ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_warn_span!(uuid, "my_span").in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_warn_span!(uuid, "my_span", ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_error_span!(uuid, "my_span").in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing_forest::uuid_error_span!(uuid, "my_span", ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
            })
            .await;

        for (tree, level) in logs.into_iter().zip([
            "TRACE", "TRACE", "DEBUG", "DEBUG", "INFO", "INFO", "WARN", "WARN", "ERROR", "ERROR",
        ]) {
            assert!(uuid == tree.uuid());
            assert!(tree.level().as_str() == level);

            let span = tree.span()?;
            assert!(span.is_empty());
        }

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_instrument_with_uuid() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                let id = Uuid::new_v4();
                info!(id = %id, "here's the id");
                async {
                    assert_eq!(id, tracing_forest::id());
                }
                .instrument(uuid_trace_span!(id, "in_async"))
                .await
            })
            .await;

        assert!(logs.len() == 2);

        let event = logs[0].event()?;
        assert!(event.fields().len() == 1);
        let field = &event.fields()[0];
        assert!(field.key == "id");

        let tree = &logs[1];
        assert!(tree.uuid().to_string() == field.value);

        let span = tree.span()?;
        assert!(span.is_empty());

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_new_builder2() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                let handle = tokio::spawn(async {
                    tracing::error!("Test message");
                });

                tracing::error!("Waiting on signal");
                sleep(Duration::from_millis(500)).await;
                handle.await.unwrap();
                tracing::error!("Stopping");
            })
            .await;

        assert!(logs.len() == 3);

        let waiting = logs[0].event()?;
        assert!(waiting.message() == "Waiting on signal");

        let test = logs[1].event()?;
        assert!(test.message() == "Test message");

        let stopping = logs[2].event()?;
        assert!(stopping.message() == "Stopping");

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_docs_example() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .on_registry()
            .on(async {
                info!("Ready");
                info!("Set");
                info!("Go!");
            })
            .await;

        assert!(logs.len() == 3);

        let ready = logs[0].event()?;
        assert!(ready.message() == "Ready");

        let set = logs[1].event()?;
        assert!(set.message() == "Set");

        let go = logs[2].event()?;
        assert!(go.message() == "Go!");

        Ok(())
    }
}

#[cfg(feature = "derive")]
tracing_forest::declare_tags! {
    use tracing_forest::Tag;

    #[allow(dead_code)]
    #[derive(Tag)]
    pub(crate) enum KanidmTag {
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

    #[allow(dead_code)]
    #[derive(Tag)]
    pub(crate) enum MyTag {
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

    #[allow(dead_code)]
    #[derive(Tag)]
    pub(crate) enum GreetingTag {
        #[tag(lvl = "info", msg = "greeting", macro = "greeting")]
        Greeting,
    }

    #[allow(dead_code)]
    #[derive(Tag)]
    pub(crate) enum BearTag {
        #[tag(lvl = "info", msg = "brown.bear", macro = "brown_bear")]
        BrownBear,
        #[tag(lvl = "warn", msg = "black.bear", macro = "black_bear")]
        BlackBear,
        #[tag(lvl = "error", msg = "polar.bear", macro = "polar_bear")]
        PolarBear,
    }
}

#[cfg(all(feature = "derive", feature = "sync"))]
mod tag_tests {
    use tracing_forest::Tag;

    #[tokio::test]
    async fn test_macros() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .set_tag(crate::tracing_forest_tag::KanidmTag::from_field)
            .on_registry()
            .on(async {
                admin_info!("some info for the admin");
                request_error!("the request timed out");
                security_critical!("the db has been breached");
            })
            .await;

        assert!(logs.len() == 3);

        let admin_info = logs[0].event()?;
        assert!(admin_info.message() == "some info for the admin");
        assert!(admin_info.tag().unwrap().message == "admin.info");

        let request_error = logs[1].event()?;
        assert!(request_error.message() == "the request timed out");
        assert!(request_error.tag().unwrap().message == "request.error");

        let security_critical = logs[2].event()?;
        assert!(security_critical.message() == "the db has been breached");
        assert!(security_critical.tag().unwrap().message == "security.critical");

        Ok(())
    }

    #[tokio::test]
    async fn test_demo_macros() -> Result<(), Box<dyn std::error::Error>> {
        let logs = tracing_forest::capture()
            .set_tag(crate::tracing_forest_tag::MyTag::from_field)
            .on_registry()
            .on(async {
                use tracing_forest::Tag;
                tracing::trace!(
                    __event_tag = crate::tracing_forest_tag::MyTag::Simple.as_field(),
                    "a simple log"
                );
                all_features!("all the features wow");
            })
            .await;

        assert!(logs.len() == 2);

        let simple = logs[0].event()?;
        assert!(simple.message() == "a simple log");
        assert!(simple.tag().unwrap().message == "simple");

        let all_features = logs[1].event()?;
        assert!(all_features.message() == "all the features wow");
        assert!(all_features.tag().unwrap().message == "all.features");

        Ok(())
    }

    #[tokio::test]
    async fn test_panic_tag_unset() {
        tracing_forest::capture()
            .on_registry()
            .on(async {
                std::panic::set_hook(Box::new(|_| {}));
                assert!(std::panic::catch_unwind(|| {
                    all_features!("this should be panicking");
                })
                .is_err());
            })
            .await;
    }
}

#[cfg(feature = "attribute")]
mod attribute_tests {
    use std::time::Duration;
    use tokio::time::{interval, sleep};
    use tracing::{info, trace_span, Instrument};

    mod blocking {
        use super::*;

        #[tracing_forest::test]
        fn test_early_return() {
            // tests that returning in the test doesn't prevent logging
            info!("a log");
            return;
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
        fn test_immediate() {
            trace_span!("my_span").in_scope(|| {
                info!("first");
                info!("second");
                info!(immediate = true, "third, but immediately");
            })
        }
    }

    #[cfg(feature = "sync")]
    mod sync {
        use super::*;

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
        async fn test_instrument() {
            use std::time::Duration;
            use tokio::time::sleep;

            async {
                for i in 0..3 {
                    info!("iter: {}", i);
                    sleep(Duration::from_millis(50)).await;
                }
            }
            .instrument(trace_span!("takes awhile"))
            .await;
        }

        #[tracing_forest::test]
        #[tokio::test]
        async fn test_counting() {
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
        }

        #[tracing_forest::main]
        #[tokio::main(flavor = "current_thread")]
        #[test]
        async fn test_main() {
            info!("running as a main function");
        }

        #[tracing_forest::test]
        #[tokio::test]
        async fn test_immediate() {
            async {
                info!("logged first chronologically");
                info!(immediate = true, "logged second, but printed immediately");
            }
            .instrument(trace_span!("my_span"))
            .await
        }
    }
}

// TODO: write better tests
// * Test for a filter filtering out some logs
// * fallbacks
// *
