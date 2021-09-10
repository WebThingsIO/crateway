use crate::gateway::Gateway;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_user_count() {
    let gateway = Gateway::startup().await;
    let (status, response) = gateway.get::<Value>("/users/count").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!({"count": 0}));
}

#[tokio::test]
#[serial]
async fn get_user_info() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway.get::<Value>("/users/info").await;
    assert_eq!(status, StatusCode::OK);
    let arr = response.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].get("email").unwrap(), "test@test");
    assert_eq!(arr[0].get("id").unwrap(), 1);
    assert_eq!(arr[0].get("loggedIn").unwrap(), true);
    assert_eq!(arr[0].get("name").unwrap(), "Tester");
    assert!(arr[0].get("password").is_some());
}

#[tokio::test]
#[serial]
async fn get_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway.get::<Value>("/users/a_user").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn post_user_initial() {
    let gateway = Gateway::startup().await;
    let (status, response) = gateway
        .post::<Value>(
            "/users",
            json!({"email": "test@test", "password": "password", "name": "Tester"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(response.get("jwt").is_some());
}

#[tokio::test]
#[serial]
async fn put_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway
        .put::<Value>(
            "/users/1",
            json!({"email": "foo@bar", "password": "password", "newPassword": "test1234", "name": "Peter"}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
