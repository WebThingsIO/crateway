use reqwest::{Method, RequestBuilder, StatusCode};
use serde_json::{json, Value};
use serial_test::serial;
use test_utils::gateway::{Gateway, GatewayRequest};

#[tokio::test]
#[serial]
async fn login() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    gateway.create_secondary_user().await;
    let (status, response) = RequestBuilder::build_from(&gateway, Method::POST, "/login")
        .body(serde_json::to_string(&json!({"email": "foo@bar", "password": "42"})).unwrap())
        .send_req::<Value>()
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(response.get("jwt").is_some());
}

#[tokio::test]
#[serial]
async fn login_fail_unknown_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = RequestBuilder::build_from(&gateway, Method::POST, "/login")
        .body(serde_json::to_string(&json!({"email": "foo@bar", "password": "42"})).unwrap())
        .send_req::<Value>()
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn login_fail_bad_password() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    gateway.create_secondary_user().await;
    let (status, _) = gateway
        .post::<Value>("/login", json!({"email": "foo@bar", "password": "21"}))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
