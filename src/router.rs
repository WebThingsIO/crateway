/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon_manager::{AddonManager, DisableAddon, EnableAddon},
    db::{Db, GetThing, GetThings},
    model::Thing,
};
use anyhow::anyhow;
use rocket::{
    http::Status,
    response::status,
    serde::{json::Json, Deserialize, Serialize},
    Route,
};
use std::collections::BTreeMap;
use xactor::Service;

pub fn routes() -> Vec<Route> {
    routes![
        get_things,
        get_thing,
        put_addon,
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
async fn get_things() -> Result<Json<Vec<Thing>>, status::Custom<&'static str>> {
    match Db::from_registry()
        .await
        .expect("Get db")
        .call(GetThings)
        .await
        .map_err(|err| anyhow!(err))
        .flatten()
    {
        Err(e) => {
            error!("Error during db.get_things: {:?}", e);
            Err(status::Custom(Status::InternalServerError, "Err"))
        }
        Ok(t) => Ok(Json(t)),
    }
}

#[get("/thing/<thing_id>")]
async fn get_thing(thing_id: String) -> Result<Option<Json<Thing>>, status::Custom<String>> {
    match Db::from_registry()
        .await
        .expect("Get db")
        .call(GetThing(thing_id.to_owned()))
        .await
        .map_err(|err| anyhow!(err))
        .flatten()
    {
        Err(e) => {
            error!("Error during db.get_things: {:?}", e);
            Err(status::Custom(
                Status::InternalServerError,
                "Err".to_owned(),
            ))
        }
        Ok(t) => {
            if let Some(t) = t {
                Ok(Some(Json(t)))
            } else {
                Err(status::Custom(
                    Status::NotFound,
                    format!("Unable to find thing with title = {}", thing_id),
                ))
            }
        }
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
        match AddonManager::from_registry()
            .await
            .expect("Get addon manager")
            .call(EnableAddon(addon_id))
            .await
            .map_err(|err| anyhow!(err))
            .flatten()
        {
            Ok(()) => Ok(Json(AddonEnabledState { enabled: true })),
            Err(e) => {
                error!("Failed to enable addon: {:?}", e);
                Err(status::Custom(
                    Status::InternalServerError,
                    "Failed to enable addon".to_owned(),
                ))
            }
        }
    } else {
        match AddonManager::from_registry()
            .await
            .expect("Get addon manager")
            .call(DisableAddon(addon_id))
            .await
            .map_err(|err| anyhow!(err))
            .flatten()
        {
            Ok(()) => Ok(Json(AddonEnabledState { enabled: false })),
            Err(e) => {
                error!("Failed to disable addon: {:?}", e);
                Err(status::Custom(
                    Status::InternalServerError,
                    "Failed to disable addon".to_owned(),
                ))
            }
        }
    }
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
    #[serde(rename(serialize = "setImplemented"))]
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
