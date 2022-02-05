use crate::{jwt::JSONWebToken, macros::ToRocket, models::Thing};
use anyhow::anyhow;
use rocket::{http::Status, response::status, serde::json::Json, Route};
use webthings_gateway_ipc_types::Device;

pub fn routes() -> Vec<Route> {
    routes![get_things, get_thing, post_things]
}

#[get("/")]
async fn get_things(_jwt: JSONWebToken) -> Result<Json<Vec<Thing>>, status::Custom<String>> {
    let t = Thing::all()
        .await
        .to_rocket("Error during db.get_things", Status::InternalServerError)?;

    Ok(Json(t))
}

#[get("/<thing_id>")]
async fn get_thing(
    thing_id: String,
    _jwt: JSONWebToken,
) -> Result<Option<Json<Thing>>, status::Custom<String>> {
    let t = Thing::find(&thing_id)
        .await
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

#[post("/", data = "<data>")]
async fn post_things(
    data: Json<Device>,
    _jwt: JSONWebToken,
) -> Result<status::Created<Json<Thing>>, status::Custom<String>> {
    let device = data.0;
    let t = Thing::find(&device.id)
        .await
        .to_rocket("Error during db.get_thing", Status::InternalServerError)?;
    if t.is_some() {
        Err(status::Custom(
            Status::BadRequest,
            "Thing already added".to_owned(),
        ))
    } else {
        let device_id = device.id.clone();
        let count = Thing::create(device)
            .await
            .to_rocket("Error saving new thing", Status::InternalServerError)?;
        if count == 1 {
            let t = Thing::find(&device_id)
                .await.and_then(|v| v.ok_or_else(|| anyhow!("Thing not found")))
                .to_rocket("Error during db.get_thing", Status::InternalServerError)?;
            info!(
                "Successfully created new thing {}",
                t.title.clone().unwrap_or_else(|| "".to_owned())
            );
            Ok(status::Created::new("").body(Json(t)))
        } else {
            Err(status::Custom(
                Status::BadRequest,
                "Could not save thing".to_owned(),
            ))
        }
    }
}
