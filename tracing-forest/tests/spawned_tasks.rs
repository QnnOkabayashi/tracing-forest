use tokio::time::{sleep, Duration};

#[tokio::test(flavor = "multi_thread")]
async fn spawned_tasks() -> Result<(), Box<dyn std::error::Error>> {
    let logs = tracing_forest::capture()
        .set_global(true)
        .on_registry()
        .on(async {
            tracing::error!("Waiting on signal");
            let handle = tokio::spawn(async {
                tracing::error!("Test message");
            });
            sleep(Duration::from_millis(100)).await;
            handle.await.unwrap();
            tracing::error!("Stopping");
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
