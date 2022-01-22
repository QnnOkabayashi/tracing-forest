use tracing_forest::cfg_derive;

#[cfg(feature = "sync")]
mod util {
    use std::future::Future;
    use tokio::sync::mpsc;
    use tracing_forest::layer::Tree;
    use tracing_forest::processor::Processor;
    use tracing_forest::Tag;

    struct Capture(mpsc::UnboundedSender<Tree>);

    impl Processor for Capture {
        fn process(&self, tree: Tree) {
            self.0.send(tree).unwrap();
        }
    }

    #[allow(dead_code)]
    pub async fn capture<T, F>(future: F) -> mpsc::UnboundedReceiver<Tree>
    where
        T: Tag,
        F: Future<Output = ()>,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        let subscriber = Capture(tx).into_layer().tag::<T>().into_subscriber();
        let _guard = tracing::subscriber::set_default(subscriber);
        future.await;
        rx
    }

    #[allow(dead_code)]
    pub async fn capture_untagged<F>(future: F) -> mpsc::UnboundedReceiver<Tree>
    where
        F: Future<Output = ()>,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        let subscriber = Capture(tx).into_layer().into_subscriber();
        let _guard = tracing::subscriber::set_default(subscriber);
        future.await;
        rx
    }
}

#[cfg(feature = "sync")]
mod sync_tests {
    use crate::util::capture_untagged;
    use rand::Rng;
    use tokio::time::{sleep, Duration};
    use tracing::{info, trace_span, Instrument};

    async fn sleep_rand(low: u64, high: u64) {
        sleep(Duration::from_millis(
            rand::thread_rng().gen_range(low..high),
        ))
        .await
    }

    #[tokio::test]
    async fn test_two_tasks_random_sleeps() {
        let mut rx = capture_untagged(async {
            let a = async {
                async {
                    info!(client = %"a", "sent request");
                    sleep_rand(10, 100).await;
                    info!(client = %"a", "received response");
                }
                .instrument(trace_span!("a request"))
                .await;

                sleep_rand(10, 100).await;

                async {
                    info!(client = %"a", "sending response");
                    sleep_rand(10, 100).await;
                    info!(client = %"a", "response sent");
                }
                .instrument(trace_span!("a response"))
                .await;
            }
            .instrument(trace_span!("a"));

            let b = async {
                async {
                    info!(client = %"b", "sent request");
                    sleep_rand(10, 100).await;
                    info!(client = %"b", "received response");
                }
                .instrument(trace_span!("b request"))
                .await;

                sleep_rand(10, 100).await;

                async {
                    info!(client = %"b", "sending response");
                    sleep_rand(10, 100).await;
                    info!(client = %"b", "response sent");
                }
                .instrument(trace_span!("b response"))
                .await;
            }
            .instrument(trace_span!("b"));

            let _ = tokio::join!(a, b);
        })
        .await;

        for _ in 0..2 {
            let client = rx.recv().await.unwrap();
            let span = client.kind.into_span().unwrap();
            assert!(span.children.len() == 2);

            for (child, which) in span
                .children
                .into_iter()
                .zip(["request", "response"].iter())
            {
                // one for req, other for res
                let inner = child.kind.into_span().unwrap();
                assert!(inner.name == format!("{} {}", span.name, which));
                assert!(inner.children.len() == 2);
                for child in inner.children.into_iter() {
                    // one for start, one for end
                    let event = child.kind.into_event().unwrap();
                    // println!("{} vs {}", event.fields[0].value, span.name);
                    assert!(event.fields[0].value == span.name);
                }
            }
        }

        assert!(rx.recv().await.is_none());
    }
}

#[cfg(feature = "uuid")]
mod uuid_tests {
    use tracing::trace_span;
    use tracing_forest::uuid_trace_span;
    use uuid::Uuid;

    mod blocking {
        use super::*;

        #[test]
        fn test_set_get_uuid() {
            tracing_forest::builder()
                .with_test_writer()
                .blocking_layer()
                .on_closure(|| {
                    let id = Uuid::new_v4();
                    uuid_trace_span!(id, "my_span").in_scope(|| {
                        assert_eq!(id, tracing_forest::id());
                    });
                });
        }

        #[test]
        fn test_panic_get_id_not_in_span() {
            tracing_forest::builder()
                .with_test_writer()
                .blocking_layer()
                .on_closure(|| {
                    assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
                });
        }

        #[test]
        fn test_panic_get_id_not_in_subscriber() {
            assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
        }

        #[test]
        fn test_panic_get_id_after_close() {
            tracing_forest::builder()
                .with_test_writer()
                .blocking_layer()
                .on_closure(|| {
                    trace_span!("in a span").in_scope(|| {
                        assert!(std::panic::catch_unwind(tracing_forest::id).is_ok());
                    });
                    assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
                });
        }

        #[test]
        fn test_stack_of_spans() {
            tracing_forest::builder()
                .with_test_writer()
                .blocking_layer()
                .on_closure(|| {
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

                        // `first_id` should still be the current ID
                        assert_eq!(first_id, tracing_forest::id());
                    });
                });
        }

        #[test]
        fn test_get_many_times() {
            tracing_forest::builder()
                .with_test_writer()
                .blocking_layer()
                .on_closure(|| {
                    trace_span!("first").in_scope(|| {
                        let a = tracing_forest::id();
                        let b = tracing_forest::id();
                        assert_eq!(a, b);
                    });
                });
        }

        #[test]
        fn test_uuid_span_macros() {
            tracing_forest::builder()
                .with_test_writer()
                .blocking_layer()
                .on_closure(|| {
                    let uuid = Uuid::new_v4();
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
        }
    }

    #[cfg(feature = "sync")]
    mod sync {
        use super::*;
        use crate::util::capture_untagged;
        use tracing::{info, Instrument};

        #[tokio::test]
        async fn test_panic_get_id_not_in_span() {
            tracing_forest::builder()
                .with_test_writer()
                .async_layer()
                .on_future(async {
                    assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
                })
                .await;
        }

        #[tokio::test]
        async fn test_panic_get_id_not_in_subscriber() {
            assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
        }

        #[tokio::test]
        async fn test_panic_get_id_after_close() {
            let mut rx = capture_untagged(async {
                trace_span!("in a span").in_scope(|| {
                    assert!(std::panic::catch_unwind(tracing_forest::id).is_ok());
                });
                assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
            })
            .await;

            let in_a_span = rx.recv().await.unwrap();
            let span = in_a_span.kind.into_span().unwrap();
            assert!(span.name == "in a span");
            assert!(span.children.len() == 0);

            assert!(rx.recv().await.is_none());
        }

        #[tokio::test]
        async fn test_two_stacks_of_spans() {
            // Tests that two task running concurrently truely do not interfere
            // with each other's span data.
            let mut rx = capture_untagged(async {
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

            for _ in 0..2 {
                let tree = rx.recv().await.unwrap();
                let mut span = tree.kind.into_span().unwrap();
                assert!(span.children.len() == 1);
                let child_tree = span.children.remove(0);
                let child_span = child_tree.kind.into_span().unwrap();
                assert!(child_span.name == format!("{}2", span.name));
            }

            assert!(rx.recv().await.is_none());
        }

        #[tokio::test]
        async fn test_get_many_times() {
            let mut rx = capture_untagged(async {
                async {
                    let a = tracing_forest::id();
                    let b = tracing_forest::id();
                    assert_eq!(a, b);
                }
                .instrument(trace_span!("my_span"))
                .await;
            })
            .await;

            let my_span = rx.recv().await.unwrap();
            let span = my_span.kind.into_span().unwrap();
            assert!(span.children.len() == 0);
            assert!(span.name == "my_span");

            assert!(rx.recv().await.is_none());
        }

        #[tokio::test]
        async fn test_uuid_span_macros() {
            let uuid = Uuid::new_v4();
            let mut rx = capture_untagged(async {
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

            for level in [
                "TRACE", "TRACE", "DEBUG", "DEBUG", "INFO", "INFO", "WARN", "WARN", "ERROR",
                "ERROR",
            ] {
                let tree = rx.recv().await.unwrap();
                assert!(uuid == tree.attrs.uuid);
                assert!(tree.attrs.level.as_str() == level);
                let span = tree.kind.into_span().unwrap();
                assert!(span.children.len() == 0);
            }

            assert!(rx.recv().await.is_none());
        }

        #[tokio::test]
        async fn test_instrument_with_uuid() {
            let mut rx = capture_untagged(async {
                let id = Uuid::new_v4();
                info!(id = %id, "here's the id");
                async {
                    assert_eq!(id, tracing_forest::id());
                }
                .instrument(uuid_trace_span!(id, "in_async"))
                .await
            })
            .await;

            let the_id_event = rx.recv().await.unwrap();
            let mut event = the_id_event.kind.into_event().unwrap();
            assert!(event.fields.len() == 1);
            let field = event.fields.remove(0);
            assert!(field.key == "id");
            let id = field.value;

            let in_async_span = rx.recv().await.unwrap();
            assert!(id == in_async_span.attrs.uuid.to_string());
            let span = in_async_span.kind.into_span().unwrap();
            assert!(span.children.len() == 0);

            assert!(rx.recv().await.is_none());
        }
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
    use crate::util::capture;

    #[tokio::test]
    async fn test_macros() {
        let mut rx = capture::<crate::tracing_forest_tag::KanidmTag, _>(async {
            admin_info!("some info for the admin");
            request_error!("the request timed out");
            security_critical!("the db has been breached");
        })
        .await;

        let admin_info = rx.recv().await.unwrap();
        let event = admin_info.kind.into_event().unwrap();
        assert!(event.message == "some info for the admin");
        assert!(event.tag.unwrap().message == "admin.info");

        let request_error = rx.recv().await.unwrap();
        let event = request_error.kind.into_event().unwrap();
        assert!(event.message == "the request timed out");
        assert!(event.tag.unwrap().message == "request.error");

        let security_critical = rx.recv().await.unwrap();
        let event = security_critical.kind.into_event().unwrap();
        assert!(event.message == "the db has been breached");
        assert!(event.tag.unwrap().message == "security.critical");

        assert!(rx.recv().await.is_none());
    }

    #[tokio::test]
    async fn test_demo_macros() {
        let mut rx = capture::<crate::tracing_forest_tag::MyTag, _>(async {
            use tracing_forest::Tag;
            tracing::trace!(
                __event_tag = crate::tracing_forest_tag::MyTag::Simple.as_field(),
                "a simple log"
            );
            all_features!("all the features wow");
        })
        .await;

        let simple = rx.recv().await.unwrap();
        let event = simple.kind.into_event().unwrap();
        assert!(event.message == "a simple log");
        assert!(event.tag.unwrap().message == "simple");

        let all_features = rx.recv().await.unwrap();
        let event = all_features.kind.into_event().unwrap();
        assert!(event.message == "all the features wow");
        assert!(event.tag.unwrap().message == "all.features");
    }

    #[test]
    fn test_panic_tag_unset() {
        tracing_forest::builder()
            .with_test_writer()
            .blocking_layer()
            .on_closure(|| {
                assert!(std::panic::catch_unwind(|| {
                    all_features!("this should be panicking");
                })
                .is_err());
            });
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

mod builder_tests {
    use super::*;
    use tracing::info;

    mod blocking {
        use super::*;

        #[test]
        fn test_json() {
            tracing_forest::builder()
                .with_test_writer()
                .blocking_layer()
                .on_closure(|| {
                    info!("Hello, world!");
                })
        }

        cfg_derive! {
        #[test]
        fn test_with_tag() {
            tracing_forest::builder()
                .with_test_writer()
                .with_tag::<crate::tracing_forest_tag::GreetingTag>()
                .blocking_layer()
                .on_closure(|| {
                    greeting!("Hello, world!");
                })
        }

        #[test]
        fn test_all_the_marbles() {
            tracing_forest::builder()
                .with_test_writer()
                .with_tag::<crate::tracing_forest_tag::BearTag>()
                .blocking_layer()
                .with(tracing_subscriber::filter::LevelFilter::WARN)
                .on_closure(|| {
                    brown_bear!("if it's brown get down");
                    black_bear!("if it's black fight back");
                    polar_bear!("if it's white good night");
                })
            }
        }
    }

    #[cfg(feature = "sync")]
    mod sync {
        use super::*;

        cfg_derive! {
            #[tokio::test]
            async fn test_with_tag() {
                tracing_forest::builder()
                    .with_test_writer()
                    .with_tag::<crate::tracing_forest_tag::GreetingTag>()
                    .async_layer()
                    .on_future(async {
                        greeting!("Hello, world!");
                    })
                    .await
            }

            #[tokio::test]
            async fn test_all_the_marbles() {
                tracing_forest::builder()
                    .with_test_writer()
                    .with_tag::<crate::tracing_forest_tag::BearTag>()
                    .async_layer()
                    .with(tracing_subscriber::filter::LevelFilter::WARN)
                    .on_future(async {
                        brown_bear!("if it's brown get down");
                        black_bear!("if it's black fight back");
                        polar_bear!("if it's white good night");
                    })
                    .await
            }
        }
    }
}
