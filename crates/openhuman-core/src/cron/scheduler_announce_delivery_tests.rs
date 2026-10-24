use super::*;
use tinyflows_bus::schedule::types::DeliveryStatus;

#[tokio::test]
async fn announce_to_unavailable_channel_is_reported_as_failed() {
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp).await;
    let missing_channel = format!("missing-{}", uuid::Uuid::new_v4());
    let mut job = test_job("echo ok");
    job.delivery = DeliveryConfig {
        mode: "announce".into(),
        channel: Some(missing_channel),
        to: Some("target".into()),
        best_effort: false,
    };

    let result = deliver_run(
        &crate::core::runtime::DispatchCtx::ambient(),
        &config,
        &job,
        "run-1",
        "hello",
        true,
    )
    .await;

    assert_eq!(
        result.unwrap(),
        DeliveryStatus::Failed,
        "an announce without a registered channel must not report Delivered"
    );
}
