use tokio::time::{sleep, Duration};
use tracing_forest::processor::Processor;

#[tracing::instrument]
async fn my_server() {
    loop {
        tracing::info!("doing server things...");
        sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn join_hanging_task() -> Result<(), Box<dyn std::error::Error>> {
    let logs = tracing_forest::capture()
        .set_global(true)
        .map_sender(Processor::with_ignore_fallback)
        .on_registry()
        .on(async {
            // the task will generate logs, but doesn't finish before
            // the subscriber finishes and so the logs are ignored.
            tokio::spawn(my_server());
            sleep(Duration::from_millis(500)).await;
            tracing::info!("shutting down...");
        })
        .await;

    assert!(logs.len() == 1);

    let shutdown = logs[0].event()?;
    assert!(shutdown.message() == "shutting down...");

    Ok(())
}
