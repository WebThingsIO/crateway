use test_utils::gateway::Gateway;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_extensions() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway.get::<Value>("/extensions").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!({}));
}
