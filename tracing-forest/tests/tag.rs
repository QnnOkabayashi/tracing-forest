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
async fn test_kanidm_tag() -> Result<(), Box<dyn std::error::Error>> {
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
