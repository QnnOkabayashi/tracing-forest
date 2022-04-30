use tracing_forest::{traits::*, util::*, ForestLayer, Printer, Tag};
use tracing_subscriber::Registry;

#[test]
#[ignore]
fn test_manual_with_json() {
    let processor = Printer::new().formatter(serde_json::to_string_pretty);
    let layer = ForestLayer::from(processor);
    let subscriber = Registry::default().with(layer);
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
        let layer = ForestLayer::new(Printer::default(), |event: &Event| {
            let level = *event.metadata().level();
            let target = event.metadata().target();

            let tag = match target {
                "security" if level == Level::ERROR => Tag::builder()
                    .set_icon('🔐')
                    .set_prefix(target)
                    .set_suffix("critical")
                    .finish(),
                "security" if level == Level::INFO => Tag::builder()
                    .set_icon('🔓')
                    .set_prefix(target)
                    .set_suffix("access")
                    .finish(),
                "admin" | "request" | "filter" => {
                    Tag::builder().set_prefix(target).set_level(level).finish()
                }
                _ => return None,
            };

            Some(tag)
        });

        let subscriber = Registry::default().with(layer);
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
                                info!(target: "admin", alive = false, status = "sad", "there's been a big mistake")
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
