use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;
use test_utils::gateway::Gateway;

#[tokio::test]
#[serial]
async fn get_things_empty() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway.get::<Value>("/things").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!([]));
}

#[tokio::test]
#[serial]
async fn get_things() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;

    let (status, response) = gateway.get::<Value>("/things").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!([]));

    let (status, _) = gateway
        .post::<String>(
            "/things",
            json!({"id": "mock-device", "title": "Mock Device"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, response) = gateway.get::<Value>("/things").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!([{
            "connected": true,
            "id": "mock-device",
            "title": "Mock Device",
        }])
    );
}

#[tokio::test]
#[serial]
async fn get_unknown_thing() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _response) = gateway.get::<Value>("/things/mock-device").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn get_thing() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;

    let (status, _) = gateway.get::<Value>("/things/mock-device").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, _) = gateway
        .post::<String>(
            "/things",
            json!({"id": "mock-device", "title": "Mock Device"}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, response) = gateway.get::<Value>("/things/mock-device").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!({
            "connected": true,
            "id": "mock-device",
            "title": "Mock Device",
        })
    );
}
