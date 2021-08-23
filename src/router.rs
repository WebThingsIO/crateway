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
use xactor::Service;

pub fn routes() -> Vec<Route> {
    routes![get_things, get_thing, put_addon]
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
