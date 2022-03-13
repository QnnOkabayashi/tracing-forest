type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(feature = "tokio")]
mod tokio_tests {
    use rand::Rng;
    use std::future::Future;
    use tokio::time::{sleep, Duration};
    use tracing::{info, trace, trace_span, Instrument};

    fn do_work() -> impl Future {
        let millis = rand::thread_rng().gen_range(0..200);
        sleep(Duration::from_millis(millis))
    }

    #[tokio::test]
    async fn test_two_tasks_random_sleeps() -> super::Result<()> {
        let logs = tracing_forest::capture()
            .build()
            .on(async {
                let a = async {
                    async {
                        info!(client = %"a", "sent request");
                        do_work().await;
                        info!(client = %"a", "received response");
                    }
                    .instrument(trace_span!("a request"))
                    .await;

                    do_work().await;

                    async {
                        info!(client = %"a", "sending response");
                        do_work().await;
                        info!(client = %"a", "response sent");
                    }
                    .instrument(trace_span!("a response"))
                    .await;
                }
                .instrument(trace_span!("a"));

                let b = async {
                    async {
                        info!(client = %"b", "sent request");
                        do_work().await;
                        info!(client = %"b", "received response");
                    }
                    .instrument(trace_span!("b request"))
                    .await;

                    do_work().await;

                    async {
                        info!(client = %"b", "sending response");
                        do_work().await;
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
                    assert!(event.fields()[0].value() == span.name());
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_filtering() -> super::Result<()> {
        use tracing_forest::ForestLayer;
        use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, Registry};

        let logs = tracing_forest::capture()
            .build_with(|layer: ForestLayer<_, _>| {
                Registry::default().with(layer).with(LevelFilter::INFO)
            })
            .on(async {
                trace!("unimportant information");
                info!("important information");
            })
            .await;

        assert!(logs.len() == 1);

        let info = logs[0].event()?;

        assert!(info.message() == Some("important information"));

        Ok(())
    }

    // #[test]
    // fn test_urgent() {
    //     tracing_forest::init();
    //     trace_span!("my_span").in_scope(|| {
    //         info!("first");
    //         info!("second");
    //         info!(urgent = true, "third, but urgently");
    //     });
    // }

    // #[tracing::instrument]
    // async fn conn(id: u32) {
    //     for i in 0..3 {
    //         do_work().await;
    //         info!(id, "step {}", i);
    //     }
    // }

    // #[tokio::test]
    // async fn test_server() -> super::Result<()> {
    //     tracing_forest::init();

    //     let mut connections = vec![];

    //     for id in 0..3 {
    //         connections.push(tokio::spawn(conn(id)));
    //     }

    //     for conn in connections {
    //         conn.await?;
    //     }

    //     Ok(())
    // }
}

#[cfg(all(feature = "uuid", feature = "tokio"))]
mod uuid_tests {
    use tokio::time::{sleep, Duration};
    use tracing::trace_span;
    use tracing::{info, Instrument};
    use tracing_forest::ForestLayer;
    use tracing_subscriber::{layer::SubscriberExt, registry};
    use uuid::Uuid;

    #[test]
    fn test_panic_get_id_not_in_span() {
        let _guard = tracing::subscriber::set_default(registry().with(ForestLayer::default()));
        std::panic::set_hook(Box::new(|_| {}));
        assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
    }

    #[test]
    fn test_panic_get_id_not_in_subscriber() {
        assert!(std::panic::catch_unwind(tracing_forest::id).is_err());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_panic_get_id_after_close() -> super::Result<()> {
        let logs = tracing_forest::capture()
            .build()
            .on(async {
                let uuid = Uuid::new_v4();
                trace_span!("in a span", %uuid).in_scope(|| {
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
    async fn test_two_stacks_of_spans() -> super::Result<()> {
        // Tests that two task running concurrently truely do not interfere
        // with each other's span data.
        let logs = tracing_forest::capture()
            .build()
            .on(async {
                let a = async {
                    let first_id = Uuid::new_v4();
                    let second_id = Uuid::new_v4();
                    trace_span!("a_span", uuid = %first_id).in_scope(|| {
                        assert_eq!(first_id, tracing_forest::id());
                        trace_span!("a_span2", uuid = %second_id).in_scope(|| {
                            assert_eq!(second_id, tracing_forest::id());
                        });
                        assert_eq!(first_id, tracing_forest::id());
                    });
                };

                let b = async {
                    let first_id = Uuid::new_v4();
                    let second_id = Uuid::new_v4();
                    trace_span!("b_span", uuid = %first_id).in_scope(|| {
                        assert_eq!(first_id, tracing_forest::id());
                        trace_span!("b_span2", uuid = %second_id).in_scope(|| {
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
    async fn test_get_many_times() -> super::Result<()> {
        let logs = tracing_forest::capture()
            .build()
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
        assert!(span.children().is_empty());
        assert!(span.name() == "my_span");

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_uuid_span_macros() -> super::Result<()> {
        let uuid = Uuid::new_v4();
        let logs = tracing_forest::capture()
            .build()
            .on(async {
                tracing::trace_span!("my_span", %uuid).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::trace_span!("my_span", %uuid, ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::debug_span!("my_span", %uuid).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::debug_span!("my_span", %uuid, ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::info_span!("my_span", %uuid).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::info_span!("my_span", %uuid, ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::warn_span!("my_span", %uuid).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::warn_span!("my_span", %uuid, ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::error_span!("my_span", %uuid).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
                tracing::error_span!("my_span", %uuid, ans = 42).in_scope(|| {
                    assert_eq!(uuid, tracing_forest::id());
                });
            })
            .await;

        for (tree, level) in logs.into_iter().zip([
            "TRACE", "TRACE", "DEBUG", "DEBUG", "INFO", "INFO", "WARN", "WARN", "ERROR", "ERROR",
        ]) {
            let span = tree.span()?;
            assert!(span.uuid() == uuid);
            assert!(span.level().as_str() == level);
            assert!(span.children().is_empty());
        }

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_instrument_with_uuid() -> super::Result<()> {
        let logs = tracing_forest::capture()
            .build()
            .on(async {
                let id = Uuid::new_v4();
                info!(id = %id, "here's the id");

                async {
                    assert_eq!(id, tracing_forest::id());
                }
                .instrument(trace_span!("in_async", uuid = %id))
                .await;
            })
            .await;

        assert!(logs.len() == 2);

        let event = logs[0].event()?;
        assert!(event.fields().len() == 1);
        let field = &event.fields()[0];
        assert!(field.key() == "id");

        let span = logs[1].span()?;
        assert!(span.uuid().to_string() == field.value());
        assert!(span.children().is_empty());

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_new_builder2() -> super::Result<()> {
        let logs = tracing_forest::capture()
            .build()
            .on(async {
                let handle = tokio::spawn(async {
                    info!("Test message");
                });

                info!("Waiting on signal");
                sleep(Duration::from_millis(500)).await;
                handle.await.unwrap();
                info!("Stopping");
            })
            .await;

        assert!(logs.len() == 3);

        let waiting = logs[0].event()?;
        assert!(waiting.message() == Some("Waiting on signal"));

        let test = logs[1].event()?;
        assert!(test.message() == Some("Test message"));

        let stopping = logs[2].event()?;
        assert!(stopping.message() == Some("Stopping"));

        Ok(())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_docs_example() -> super::Result<()> {
        let logs = tracing_forest::capture()
            .build()
            .on(async {
                info!("Ready");
                info!("Set");
                info!("Go!");
            })
            .await;

        assert!(logs.len() == 3);

        let ready = logs[0].event()?;
        assert!(ready.message() == Some("Ready"));

        let set = logs[1].event()?;
        assert!(set.message() == Some("Set"));

        let go = logs[2].event()?;
        assert!(go.message() == Some("Go!"));

        Ok(())
    }
}

#[cfg(feature = "tokio")]
mod tag_tests {
    use tracing::{error, info, Event, Level};
    use tracing_forest::tag::Tag;

    fn kanidm_tag(event: &Event) -> Option<Tag> {
        let target = event.metadata().target();
        let level = *event.metadata().level();

        match target {
            "security" if level == Level::ERROR => {
                Some(Tag::new_custom_level(Some(target), "critical", '🔐'))
            }
            "admin" | "request" => Some(Tag::new(Some(target), level)),
            _ => None,
        }
    }

    #[tokio::test]
    async fn test_kanidm_tag() -> super::Result<()> {
        let logs = tracing_forest::capture()
            .set_tag(kanidm_tag)
            .build()
            .on(async {
                info!(target: "admin", "some info for the admin");
                error!(target: "request", "the request timed out");
                error!(target: "security", "the db has been breached");
                info!("no tags here");
                info!(target: "unrecognized", "unrecognizable tag");
            })
            .await;

        assert!(logs.len() == 5);

        let admin_info = logs[0].event()?;
        assert!(admin_info.message() == Some("some info for the admin"));
        assert!(admin_info.tag() == "admin.info");

        let request_error = logs[1].event()?;
        assert!(request_error.message() == Some("the request timed out"));
        assert!(request_error.tag() == "request.error");

        let security_critical = logs[2].event()?;
        assert!(security_critical.message() == Some("the db has been breached"));
        assert!(security_critical.tag() == "security.critical");

        let no_tags = logs[3].event()?;
        assert!(no_tags.message() == Some("no tags here"));
        assert!(no_tags.tag() == "info");

        let unrecognized = logs[4].event()?;
        assert!(unrecognized.message() == Some("unrecognizable tag"));
        assert!(unrecognized.tag() == "info");

        Ok(())
    }
}

// mod nodeps_tests {
//     use tracing::{info, info_span};
//     use tracing_forest::{ForestLayer, Printer};
//     use tracing_subscriber::{Layer, Registry};

//     #[test]
//     fn test_manual_with_json() {
//         let processor = Printer::from_formatter(serde_json::to_string_pretty);
//         let layer = ForestLayer::from(processor);
//         let subscriber = layer.with_subscriber(Registry::default());
//         tracing::subscriber::with_default(subscriber, || {
//             info!("hello, world!");
//             info_span!("my-span").in_scope(|| {
//                 info!("wassup");
//             })
//         });
//     }
// }

