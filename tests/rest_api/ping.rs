use reqwest::StatusCode;
use serial_test::serial;
use test_utils::gateway::Gateway;

#[tokio::test]
#[serial]
async fn ping() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway.get::<String>("/ping").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
