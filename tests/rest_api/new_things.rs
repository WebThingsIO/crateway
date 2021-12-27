use test_utils::{
    gateway::Gateway,
    mock_thing::{self, DeviceExt},
};
use reqwest::StatusCode;
use serde_json::{json, Value};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn get_new_things_empty() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;

    let (status, response) = gateway.get::<Value>("/new_things").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, json!([]));
}

#[tokio::test]
#[serial]
async fn get_new_things() {
    let mut device = mock_thing::device("mock-device");
    device.add_property(mock_thing::property("mock-property", "integer"));
    let (gateway, mut addon) = Gateway::startup_with_mock_addon().await;
    addon.create_mock_device(device).await;

    let (status, response) = gateway.get::<Value>("/new_things").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!([{
            "actions": {},
            "events": {},
            "href": "/things/mock-device",
            "id": "mock-device",
            "properties": {
                "mock-property": {
                    "href": "/things/mock-device/properties/mock-property",
                    "name": "mock-property",
                    "type": "integer"
                }
            },
        }])
    );
}
