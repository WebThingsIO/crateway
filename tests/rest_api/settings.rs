use crate::gateway::Gateway;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_language() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway
        .get::<Value>("/settings/localization/language")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!({
            "current": "en-US",
            "valid": [{
                "code": "en-US",
                "name": "English (United States of America)"
            }]
        })
    );
}

#[tokio::test]
#[serial]
async fn get_units() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway.get::<Value>("/settings/localization/units").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!({
            "temperature": "degree celsius"
        })
    );
}

#[tokio::test]
#[serial]
async fn get_timezone() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;
    let (status, response) = gateway
        .get::<Value>("/settings/localization/timezone")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!({
            "current": "Europe/Berlin",
            "setImplemented": true,
            "valid": ["Europe/Berlin"]
        })
    );
}
