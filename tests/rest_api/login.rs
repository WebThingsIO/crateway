use crate::gateway::Gateway;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn login() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    gateway
        .post::<String>(
            "/users",
            json!({"email": "foo@bar", "password": "42", "name": "foo"}),
        )
        .await;
    let (status, response) = gateway
        .post::<Value>("/login", json!({"email": "foo@bar", "password": "42"}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(response.get("jwt").is_some());
}

#[tokio::test]
#[serial]
async fn login_fail() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    gateway
        .post::<String>(
            "/users",
            json!({"email": "foo@bar", "password": "42", "name": "foo"}),
        )
        .await;
    let (status, _) = gateway
        .post::<Value>("/login", json!({"email": "foo@bar", "password": "21"}))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
