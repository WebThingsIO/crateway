use crate::gateway::Gateway;
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_new_things() {
    let (gateway, mut addon) = Gateway::startup_with_mock_addon().await;
    addon.create_mock_device().await;
    let (status, response) = gateway.get::<Value>("/new_things").await;
    assert_eq!(status, StatusCode::OK);
    let new_things = response.as_array().unwrap();
    assert_eq!(new_things.len(), 1);
    assert_eq!(
        new_things[0].as_object().unwrap().get("id").unwrap(),
        &json!("mock-device")
    );
}
