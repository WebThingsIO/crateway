/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{config::CONFIG, router};
use rocket::{
    fs::{relative, FileServer},
    Build, Rocket,
};
use std::env::{self, VarError};

fn rocket() -> Rocket<Build> {
    let ui_path = match env::var("WEBTHINGS_UI") {
        Ok(value) => value,
        Err(VarError::NotPresent) => relative!("gateway/build/static").to_owned(),
        Err(VarError::NotUnicode(s)) => {
            panic!(
                "Environment variable WEBTHINGS_UI_DIR contains invalid characters: {:?}",
                s
            )
        }
    };

    let rocket = rocket::build().mount("/", FileServer::from(ui_path));
    router::mount(rocket)
}

pub async fn launch() {
    env::set_var("ROCKET_PORT", CONFIG.ports.http.to_string());
    rocket()
        .ignite()
        .await
        .expect("Ignite rocket")
        .launch()
        .await
        .expect("Launch rocket");
}

#[cfg(test)]
mod test {
    extern crate rusty_fork;
    extern crate serial_test;
    use super::*;
    use crate::{
        db::{CreateUser, Db},
        macros::call,
        router::{
            login_router::Login,
            settings_router::{CurrentLanguage, CurrentTimezone, Language, Units},
        },
    };
    use rocket::{
        http::{Header, Method, Status},
        local::blocking::Client,
    };
    use rusty_fork::rusty_fork_test;
    use serde_json::json;
    use serial_test::serial;
    use std::{env, fs, thread};
    use tokio::runtime::Runtime;

    #[allow(unused_must_use)]
    fn setup() {
        let dir = env::temp_dir().join(".webthingsio");
        fs::remove_dir_all(&dir); // We really don't want to handle this result, since we don't care if the directory never existed
        env::set_var("WEBTHINGS_HOME", dir);

        let ui_dir = env::temp_dir().join(".webthingsui");
        fs::remove_dir_all(&ui_dir);
        fs::create_dir(&ui_dir);
        env::set_var("WEBTHINGS_UI", &ui_dir);
        fs::write(ui_dir.join("index.html"), "foo");
    }

    rusty_fork_test! {
        #[test]
        #[serial]
        fn get_things() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/things").dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.into_string(), Some("[]".into()));
        }

        #[test]
        #[serial]
        fn get_thing() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/thing/test").dispatch();
            assert_eq!(response.status(), Status::NotFound);
        }

        #[test]
        #[serial]
        fn get_index() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/").dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.into_string(), Some("foo".into()));
        }

        #[test]
        #[serial]
        fn get_language() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/settings/localization/language").dispatch();
            assert_eq!(response.status(), Status::Ok);

            let expected = CurrentLanguage {
                current: String::from("en-US"),
                valid: vec![Language {
                    code: String::from("en-US"),
                    name: String::from("English (United States of America)"),
                }],
            };

            assert_eq!(response.into_json::<CurrentLanguage>(), Some(expected));
        }

        #[test]
        #[serial]
        fn get_units() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/settings/localization/units").dispatch();
            assert_eq!(response.status(), Status::Ok);

            let expected = Units {
                temperature: String::from("degree celsius"),
            };

            assert_eq!(response.into_json::<Units>(), Some(expected));
        }

        #[test]
        #[serial]
        fn get_timezone() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/settings/localization/timezone").dispatch();
            assert_eq!(response.status(), Status::Ok);

            let expected = CurrentTimezone {
                current: String::from("Europe/Berlin"),
                set_implemented: true,
                valid: vec![String::from("Europe/Berlin")],
            };

            assert_eq!(response.into_json::<CurrentTimezone>(), Some(expected));
        }

        #[test]
        #[serial]
        fn login() {
            thread::spawn(|| {
                Runtime::new().unwrap().block_on(async {
                    setup();
                    env::set_var("CHECK_JWT", "1");
                    call!(Db.CreateUser("foo@bar".to_owned(), "42".to_owned(), "foo".to_owned()))
                        .expect("Create user");
                    let client = Client::tracked(rocket()).expect("Valid rocket instance");
                    let email = String::from("foo@bar");
                    let password = String::from("42");

                    let login = Login { email, password };

                    let json = serde_json::to_string(&login).expect("Serialization of test data");
                    let response = client.post("/login").body(json).dispatch();

                    assert_eq!(response.status(), Status::Ok);
                });
            });
        }

        #[test]
        #[serial]
        fn login_fail() {
            thread::spawn(|| {
                Runtime::new().unwrap().block_on(async {
                    setup();
                    env::set_var("CHECK_JWT", "1");
                    call!(Db.CreateUser("foo@bar".to_owned(), "42".to_owned(), "foo".to_owned()))
                        .expect("Create user");
                    let client = Client::tracked(rocket()).expect("Valid rocket instance");
                    let email = String::from("test@test");
                    let password = String::from("42");

                    let login = Login { email, password };

                    let json = serde_json::to_string(&login).expect("Serialization of test data");
                    let response = client.post("/login").body(json).dispatch();

                    assert_eq!(response.status(), Status::Unauthorized);
                });
            });
        }

        #[test]
        #[serial]
        fn ping() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/ping").dispatch();
            assert_eq!(response.status(), Status::NoContent);
            assert_eq!(response.into_string(), None);
        }

        #[test]
        #[serial]
        fn extensions() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/extensions").dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.into_string(), Some("{}".into()));
        }

        #[test]
        #[serial]
        fn get_user_count() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/users/count").dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.into_string(), Some("{\"count\":0}".into()));
        }

        #[test]
        #[serial]
        fn get_user_info() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/users/info").dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.into_string(), Some("[]".into()));
        }

        #[test]
        #[serial]
        fn get_user() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.get("/users/a_user").dispatch();
            assert_eq!(response.status(), Status::NotFound);
        }

        #[test]
        #[serial]
        fn post_user_initial() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client
                .post("/users")
                .body(
                    serde_json::to_string(
                        &json!({"email": "test@test", "password": "password", "name": "Tester"}),
                    )
                    .unwrap(),
                )
                .dispatch();
            assert_eq!(response.status(), Status::Ok);
        }

        #[test]
        #[serial]
        fn put_user() {
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let response = client.put("/users/1").body(serde_json::to_string(&json!({"email": "foo@bar", "password": "password", "newPassword": "test1234", "name": "Peter"})).unwrap()).dispatch();
            assert_eq!(response.status(), Status::NotFound);
        }

        #[test]
        #[serial]
        fn test_protected_routes() {
            setup();
            env::set_var("CHECK_JWT", "1");
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let header = Header::new("Authorization", "Bearer token");

            let protected_routes = vec![
                (Method::Get, "/addons", json!({})),
                (Method::Put, "/addons/an_addon", json!({"enabled": true})),
                (
                    Method::Put,
                    "/addons/an_addon/config",
                    json!({"config": {}}),
                ),
                (Method::Get, "/addons/an_addon/config", json!({})),
                (Method::Get, "/addons/an_addon/license", json!({})),
                (Method::Delete, "/addons/an_addon", json!({})),
                (
                    Method::Post,
                    "/addons",
                    json!({"id": "", "url": "", "checksum": ""}),
                ),
                (
                    Method::Patch,
                    "/addons/an_addon",
                    json!({"url": "", "checksum": ""}),
                ),
                (Method::Get, "/extensions", json!({})),
                (Method::Get, "/settings/localization/language", json!({})),
                (Method::Get, "/settings/localization/units", json!({})),
                (Method::Get, "/settings/localization/timezone", json!({})),
                (Method::Get, "/settings/addonsInfo", json!({})),
                (Method::Get, "/things", json!({})),
                (Method::Get, "/things/a_thing", json!({})),
                (Method::Get, "/users/info", json!({})),
                (Method::Get, "/users/a_user", json!({})),
                (
                    Method::Put,
                    "/users/a_user",
                    json!({"email": "", "password": "", "newPassword": "", "name": ""}),
                ),
                (Method::Delete, "/users/a_user", json!({})),
            ];

            for (method, route, param) in protected_routes {
                let response = client
                    .req(method, route)
                    .body(serde_json::to_string(&param).unwrap())
                    .dispatch();
                assert_eq!(response.status(), Status::Unauthorized);
                let response = client
                    .req(method, route)
                    .body(serde_json::to_string(&param).unwrap())
                    .header(header.clone())
                    .dispatch();
                assert_ne!(response.status(), Status::Unauthorized);
                assert_ne!(response.status(), Status::UnprocessableEntity);
            }
        }

        #[test]
        #[serial]
        fn test_unprotected_routes() {
            setup();
            env::set_var("CHECK_JWT", "1");
            let client = Client::tracked(rocket()).expect("Valid rocket instance");

            let unprotected_routes = vec![
                (Method::Get, "/ping", json!({})),
                (Method::Get, "/users/count", json!({})),
            ];

            for (method, route, param) in unprotected_routes {
                let response = client
                    .req(method, route)
                    .body(serde_json::to_string(&param).unwrap())
                    .dispatch();
                assert_ne!(response.status(), Status::Unauthorized);
                assert_ne!(response.status(), Status::UnprocessableEntity);
            }
        }

        #[test]
        #[serial]
        fn test_jwt_header() {
            setup();
            env::set_var("CHECK_JWT", "1");
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let header = Header::new("Authorization", "Bearer token");

            let response = client.get("/things").body("").header(header).dispatch();
            assert_ne!(response.status(), Status::Unauthorized);
            assert_ne!(response.status(), Status::UnprocessableEntity);
        }

        #[test]
        #[serial]
        fn test_jwt_query() {
            setup();
            env::set_var("CHECK_JWT", "1");
            let client = Client::tracked(rocket()).expect("Valid rocket instance");

            let response = client.get("/things?jwt=token").body("").dispatch();
            assert_ne!(response.status(), Status::Unauthorized);
            assert_ne!(response.status(), Status::UnprocessableEntity);
        }
    }
}
