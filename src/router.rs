/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon_configuration::AddonConfiguration,
    addon_manager::{AddonManager, UpdateAddonConfiguration},
    db::Db,
    model::Thing,
};
use actix::prelude::*;
use rocket::{http::Status, response::status, serde::json::Json, Route, State};

pub fn routes() -> Vec<Route> {
    routes![get_things, get_thing, put_addon]
}

#[get("/things")]
fn get_things(db: &State<Db>) -> Result<Json<Vec<Thing>>, status::Custom<&'static str>> {
    match db.get_things() {
        Err(e) => {
            println!("Error during db.get_things: {:?}", e);
            Err(status::Custom(Status::InternalServerError, "Err"))
        }
        Ok(t) => Ok(Json(t)),
    }
}

#[get("/thing/<thing_id>")]
fn get_thing(
    db: &State<Db>,
    thing_id: String,
) -> Result<Option<Json<Thing>>, status::Custom<String>> {
    match db.get_thing(&thing_id) {
        Err(e) => {
            println!("Error during db.get_things: {:?}", e);
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

#[put("/addons/<addon_id>", data = "<data>")]
fn put_addon(addon_id: String, data: Json<AddonConfiguration>) {
    AddonManager::from_registry().do_send(UpdateAddonConfiguration(addon_id, data.0));
}
