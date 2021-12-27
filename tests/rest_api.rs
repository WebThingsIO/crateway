use reqwest::{Method, RequestBuilder, StatusCode};
use serde_json::json;
use serial_test::serial;
use test_utils::gateway::{Gateway, GatewayRequest};

extern crate serial_test;

#[path = "rest_api/extensions.rs"]
mod extensions;
#[path = "rest_api/login.rs"]
mod login;
#[path = "rest_api/new_things.rs"]
mod new_things;
#[path = "rest_api/ping.rs"]
mod ping;
#[path = "rest_api/settings.rs"]
mod settings;
#[path = "rest_api/things.rs"]
mod things;
#[path = "rest_api/users.rs"]
mod users;

#[tokio::test]
#[serial]
async fn get_index() {
    let gateway = Gateway::startup().await;
    let (status, response) = gateway.get::<String>("/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response, "foo");
}

#[tokio::test]
#[serial]
async fn test_protected_routes() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;

    let protected_routes = vec![
        (Method::GET, "/addons", json!({})),
        (Method::PUT, "/addons/an_addon", json!({"enabled": true})),
        (
            Method::PUT,
            "/addons/an_addon/config",
            json!({"config": {}}),
        ),
        (Method::GET, "/addons/an_addon/config", json!({})),
        (Method::GET, "/addons/an_addon/license", json!({})),
        (Method::DELETE, "/addons/an_addon", json!({})),
        (
            Method::POST,
            "/addons",
            json!({"id": "", "url": "", "checksum": ""}),
        ),
        (
            Method::PATCH,
            "/addons/an_addon",
            json!({"url": "", "checksum": ""}),
        ),
        (Method::GET, "/extensions", json!({})),
        (Method::GET, "/settings/localization/language", json!({})),
        (Method::GET, "/settings/localization/units", json!({})),
        (Method::GET, "/settings/localization/timezone", json!({})),
        (Method::GET, "/settings/addonsInfo", json!({})),
        (Method::GET, "/things", json!({})),
        (Method::GET, "/things/a_thing", json!({})),
        (Method::GET, "/new_things", json!({})),
        (Method::GET, "/users/info", json!({})),
        (Method::GET, "/users/a_user", json!({})),
        (
            Method::PUT,
            "/users/a_user",
            json!({"email": "", "password": "", "newPassword": "", "name": ""}),
        ),
        (Method::DELETE, "/users/a_user", json!({})),
    ];

    for (method, route, param) in protected_routes {
        let param = serde_json::to_string(&param).unwrap();
        let (status, _) = RequestBuilder::build_from(&gateway, method.to_owned(), route)
            .body(param.to_owned())
            .send_req::<String>()
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = RequestBuilder::build_from(&gateway, method.to_owned(), route)
            .add_authorization(&gateway)
            .body(param.to_owned())
            .send_req::<String>()
            .await;
        assert_ne!(status, StatusCode::UNAUTHORIZED);
        assert_ne!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
#[serial]
async fn test_unprotected_routes() {
    let gateway = Gateway::startup().await;

    let protected_routes = vec![
        (Method::GET, "/ping", json!({})),
        (Method::GET, "/users/count", json!({})),
    ];

    for (method, route, param) in protected_routes {
        let param = serde_json::to_string(&param).unwrap();
        let (status, _) = RequestBuilder::build_from(&gateway, method.to_owned(), route)
            .body(param.to_owned())
            .send_req::<String>()
            .await;
        assert_ne!(status, StatusCode::UNAUTHORIZED);
        assert_ne!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
#[serial]
async fn test_jwt_header() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;

    let (status, _) = RequestBuilder::build_from(&gateway, Method::GET, "/things")
        .bearer_auth(gateway.jwt.clone().unwrap())
        .send_req::<String>()
        .await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    assert_ne!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
#[serial]
async fn test_jwt_query() {
    let mut gateway = Gateway::startup().await;
    gateway.authorize().await;

    let (status, _) = RequestBuilder::build_from(&gateway, Method::GET, "/things")
        .query(&[("jwt", gateway.jwt.clone().unwrap())])
        .send_req::<String>()
        .await;
    assert_ne!(status, StatusCode::UNAUTHORIZED);
    assert_ne!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
