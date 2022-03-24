use tracing::{debug, error, info, info_span, trace, warn, Level};
use tracing_forest::{tag::Tag, ForestLayer, Printer};
use tracing_subscriber::{Layer, Registry};

#[test]
#[ignore]
fn test_manual_with_json() {
    let processor = Printer::from_formatter(serde_json::to_string_pretty);
    let layer = ForestLayer::from(processor);
    let subscriber = layer.with_subscriber(Registry::default());
    tracing::subscriber::with_default(subscriber, || {
        info!("hello, world!");
        info_span!("my-span").in_scope(|| {
            info!("wassup");
        })
    });
}

#[test]
#[ignore]
fn pretty_example() {
    let _guard = {
        let layer = ForestLayer::new(Printer::default(), |event: &tracing::Event| {
            let level = *event.metadata().level();
            let target = event.metadata().target();
            match (target, level) {
                ("security", Level::ERROR) => {
                    Some(Tag::new_custom_level(Some(target), "critical", '🔐'))
                }
                ("security", Level::INFO) => {
                    Some(Tag::new_custom_level(Some(target), "access", '🔓'))
                }
                ("admin" | "request" | "filter", _) => Some(Tag::new(Some(target), level)),
                _ => None,
            }
        });

        let subscriber = layer.with_subscriber(Registry::default());
        tracing::subscriber::set_default(subscriber)
    };

    info_span!("try_from_entry_ro").in_scope(|| {
            info_span!("server::internal_search").in_scope(|| {
                info!(target: "filter", "Some filter info...");
                info_span!("server::search").in_scope(|| {
                    info_span!("be::search").in_scope(|| {
                        info_span!("be::search -> filter2idl").in_scope(|| {
                            info_span!("be::idl_arc_sqlite::get_idl").in_scope(|| {
                                info!(target: "filter", "Some filter info...");
                            });
                            info_span!("be::idl_arc_sqlite::get_idl").in_scope(|| {
                                error!(target: "admin", "On no, an admin error occurred :(");
                                debug!("An untagged debug log");
                                info!(target: "admin", alive = false, status = "very sad", "there's been a big mistake")
                            });
                        });
                    });
                    info_span!("be::idl_arc_sqlite::get_identry").in_scope(|| {
                        error!(target: "security", "A security critical log");
                        info!(target: "security", "A security access log");
                    });
                });
                info_span!("server::search<filter_resolve>").in_scope(|| {
                    warn!(target: "filter", "Some filter warning");
                });

            });
            trace!("Finished!");
        });
}
