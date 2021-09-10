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
