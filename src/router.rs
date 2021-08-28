/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon_manager::{AddonManager, DisableAddon, EnableAddon, RestartAddon},
    db::{Db, GetThing, GetThings, SetSetting},
    macros::{call, ToRocket},
    model::Thing,
};
use rocket::{
    http::Status,
    response::status,
    serde::{json::Json, Deserialize, Serialize},
    Route,
};
use std::collections::BTreeMap;

pub fn routes() -> Vec<Route> {
    routes![
        get_things,
        get_thing,
        put_addon,
        put_addon_config,
        get_user_count,
        get_language,
        get_units,
        get_timezone,
        login,
        ping,
        get_extensions
    ]
}

#[get("/things")]
async fn get_things() -> Result<Json<Vec<Thing>>, status::Custom<String>> {
    let t =
        call!(Db.GetThings).to_rocket("Error during db.get_things", Status::InternalServerError)?;

    Ok(Json(t))
}

#[get("/thing/<thing_id>")]
async fn get_thing(thing_id: String) -> Result<Option<Json<Thing>>, status::Custom<String>> {
    let t = call!(Db.GetThing(thing_id.to_owned()))
        .to_rocket("Error during db.get_thing", Status::InternalServerError)?;
    if let Some(t) = t {
        Ok(Some(Json(t)))
    } else {
        Err(status::Custom(
            Status::NotFound,
            format!("Unable to find thing with title = {}", thing_id),
        ))
    }
}

#[derive(Serialize, Deserialize)]
struct AddonEnabledState {
    enabled: bool,
}

#[put("/addons/<addon_id>", data = "<data>")]
async fn put_addon(
    addon_id: String,
    data: Json<AddonEnabledState>,
) -> Result<Json<AddonEnabledState>, status::Custom<String>> {
    if data.0.enabled {
        call!(AddonManager.EnableAddon(addon_id))
            .to_rocket("Failed to enable addon", Status::InternalServerError)?;
    } else {
        call!(AddonManager.DisableAddon(addon_id))
            .to_rocket("Failed to disable addon", Status::InternalServerError)?;
    }
    Ok(Json(AddonEnabledState {
        enabled: data.0.enabled,
    }))
}

#[derive(Serialize, Deserialize)]
struct UserCount {
    count: u32,
}

#[get("/users/count")]
async fn get_user_count() -> Json<UserCount> {
    Json(UserCount { count: 1 })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CurrentLanguage {
    pub current: String,
    pub valid: Vec<Language>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Language {
    pub code: String,
    pub name: String,
}

#[get("/settings/localization/language")]
async fn get_language() -> Json<CurrentLanguage> {
    Json(CurrentLanguage {
        current: String::from("en-US"),
        valid: vec![Language {
            code: String::from("en-US"),
            name: String::from("English (United States of America)"),
        }],
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Units {
    pub temperature: String,
}

#[get("/settings/localization/units")]
async fn get_units() -> Json<Units> {
    Json(Units {
        temperature: String::from("degree celsius"),
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CurrentTimezone {
    pub current: String,
    #[serde(rename = "setImplemented")]
    pub set_implemented: bool,
    pub valid: Vec<String>,
}

#[get("/settings/localization/timezone")]
async fn get_timezone() -> Json<CurrentTimezone> {
    Json(CurrentTimezone {
        current: String::from("Europe/Berlin"),
        set_implemented: true,
        valid: vec![String::from("Europe/Berlin")],
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Jwt {
    pub jwt: String,
}

#[post("/login", data = "<data>")]
async fn login(data: Json<Login>) -> Json<Jwt> {
    Json(Jwt {
        jwt: format!("{}:{}", data.email, data.password),
    })
}

#[get("/ping")]
async fn ping() -> Status {
    Status::Ok
}

#[get("/extensions")]
async fn get_extensions() -> Json<BTreeMap<String, String>> {
    Json(BTreeMap::new())
}

#[derive(Serialize, Deserialize)]
struct AddonConfig {
    config: serde_json::Value,
}

#[put("/addons/<addon_id>/config", data = "<data>")]
async fn put_addon_config(
    addon_id: String,
    data: Json<AddonConfig>,
) -> Result<Json<AddonConfig>, status::Custom<String>> {
    let config_key = format!("addons.{}.config", addon_id);
    call!(Db.SetSetting(config_key, data.0.config.clone())).to_rocket(
        format!("Failed to update config for addon {}", addon_id),
        Status::BadRequest,
    )?;
    call!(AddonManager.RestartAddon(addon_id.to_owned())).to_rocket(
        format!("Failed to restart addon {}", addon_id),
        Status::BadRequest,
    )?;
    Ok(Json(data.0))
}
