use tokio::time::Duration;
use tracing::{info, info_span, trace};

#[tokio::test]
async fn test_filtering() -> Result<(), Box<dyn std::error::Error>> {
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

#[tokio::test]
async fn duration_checked_sub() -> Result<(), Box<dyn std::error::Error>> {
    let logs = tracing_forest::capture()
        .build()
        .on(async {
            let parent = info_span!("parent");
            info_span!(parent: &parent, "child").in_scope(|| {
                // cursed blocking in async lol
                std::thread::sleep(Duration::from_millis(100));
            });
        })
        .await;

    assert!(logs.len() == 1);

    let parent = logs[0].span()?;
    assert!(parent.total_duration() >= parent.inner_duration());

    Ok(())
}
