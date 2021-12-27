use test_utils::gateway::Gateway;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_user_count_initial() {
    let gateway = Gateway::startup().await;
    let (status, response) = gateway.get::<Value>("/users/count").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!({"count": 0}));
}

#[tokio::test]
#[serial]
async fn get_user_count() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway.get::<Value>("/users/count").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!({"count": 1}));
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
async fn get_user_info_multiple() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    gateway.create_secondary_user().await;

    let (status, response) = gateway.get::<Value>("/users/info").await;
    assert_eq!(status, StatusCode::OK);
    let arr = response.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    assert_eq!(arr[0].get("email").unwrap(), "test@test");
    assert_eq!(arr[0].get("id").unwrap(), 1);
    assert_eq!(arr[0].get("loggedIn").unwrap(), true);
    assert_eq!(arr[0].get("name").unwrap(), "Tester");
    assert!(arr[0].get("password").is_some());

    assert_eq!(arr[1].get("email").unwrap(), "foo@bar");
    assert_eq!(arr[1].get("id").unwrap(), 2);
    assert_eq!(arr[1].get("loggedIn").unwrap(), false);
    assert_eq!(arr[1].get("name").unwrap(), "foo");
    assert!(arr[1].get("password").is_some());
}

#[tokio::test]
#[serial]
async fn get_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway.get::<Value>("/users/1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response.get("email").unwrap(), "test@test");
    assert_eq!(response.get("id").unwrap(), 1);
    assert_eq!(response.get("name").unwrap(), "Tester");
    assert!(response.get("password").is_some());
}

#[tokio::test]
#[serial]
async fn get_user_fail() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway.get::<Value>("/users/22").await;
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
async fn post_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway
        .post::<Value>(
            "/users",
            json!({"email": "foo@bar", "password": "42", "name": "foo"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(response.get("jwt").is_some());
}

#[tokio::test]
#[serial]
async fn post_user_fail() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway
        .post::<Value>(
            "/users",
            json!({"email": "test@test", "password": "42", "name": "Tester"}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn put_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway
        .put::<Value>(
            "/users/1",
            json!({"email": "foo@bar", "password": "test", "newPassword": "test1234", "name": "Peter"}),
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
#[serial]
async fn put_user_fail_unknown_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway
        .put::<Value>(
            "/users/2",
            json!({"email": "foo@bar", "password": "42", "newPassword": "test1234", "name": "Peter"}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn put_user_fail_bad_password() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, _) = gateway
        .put::<Value>(
            "/users/1",
            json!({"email": "foo@bar", "password": "42", "newPassword": "test1234", "name": "Peter"}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[serial]
async fn delete_user() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    gateway.create_secondary_user().await;
    let (status, _) = gateway.delete::<String>("/users/2").await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
