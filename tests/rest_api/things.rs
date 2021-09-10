use crate::gateway::Gateway;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_things() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway.get::<Value>("/things").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!([]));
}

#[tokio::test]
#[serial]
async fn get_thing() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _response) = gateway.get::<Value>("/things/test").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
