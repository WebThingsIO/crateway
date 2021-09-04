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
        model::Jwt,
        router::{
            login_router::Login,
            settings_router::{CurrentLanguage, CurrentTimezone, Language, Units},
        },
    };
    use rocket::{http::Status, local::blocking::Client};
    use rusty_fork::rusty_fork_test;
    use serial_test::serial;
    use std::{env, fs};

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

            let expected =  Units {
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
            setup();
            let client = Client::tracked(rocket()).expect("Valid rocket instance");
            let email = String::from("foo@bar");
            let password = String::from("42");
            let jwt = format!("{}:{}", email, password);

            let login = Login {
                email,
                password
            };

            let json = serde_json::to_string(&login).expect("Serialization of test data");
            let response = client.post("/login").body(json).dispatch();

            assert_eq!(response.status(), Status::Ok);

            let expected = Jwt {
                jwt,
            };

            assert_eq!(response.into_json::<Jwt>(), Some(expected));
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
    }
}
