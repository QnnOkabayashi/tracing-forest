use tokio::time::{sleep, timeout, Duration};
use tracing::{info_span, info, error};
use tracing::instrument::Instrument;
use surf::Url;

use std::convert::TryInto;

#[tokio::test(flavor = "multi_thread")]
async fn test_nested_awaits() {
    let client: surf::Client = surf::Config::new()
        .set_tcp_no_delay(true)
        .set_timeout(None)
        .set_max_connections_per_host(10)
        .try_into()
        .expect("Failed to build surf client");

    let mc_url = Url::parse("http://mirror.internode.on.net").unwrap();

    let f = tracing_forest::worker_task()
        .set_global(true)
        .build()
        .on(async {
            async {
                async {
                    async {
                        eprintln!("Start");
                        let x = client
                            .send(surf::head(&mc_url))
                            .await
                            .map(|resp| {
                                info!("upstream check {} -> {:?}", mc_url.as_str(), resp.status());
                                resp.status() == surf::StatusCode::Ok
                            })
                            .unwrap_or_else(|e| {
                                error!("upstream check error {} -> {:?}", mc_url.as_str(), e);
                                false
                            });

                    }
                    .instrument(info_span!("inner"))
                    .await
                }
                .instrument(info_span!("middle"))
                .await
            }
            .instrument(info_span!("outer"))
            .await
        });

    timeout(Duration::from_millis(5000), f)
        .await
        .expect("Shutdown signal wasn't sent");
}
