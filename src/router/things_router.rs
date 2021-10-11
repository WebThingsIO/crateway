use crate::{
    db::{CreateThing, Db, GetThing, GetThings},
    jwt::JSONWebToken,
    macros::{call, ToRocket},
    model::Thing,
};
use rocket::{http::Status, response::status, serde::json::Json, Route};
use webthings_gateway_ipc_types::Device;

pub fn routes() -> Vec<Route> {
    routes![get_things, get_thing, post_things]
}

#[get("/")]
async fn get_things(_jwt: JSONWebToken) -> Result<Json<Vec<Thing>>, status::Custom<String>> {
    let t =
        call!(Db.GetThings).to_rocket("Error during db.get_things", Status::InternalServerError)?;

    Ok(Json(t))
}

#[get("/<thing_id>")]
async fn get_thing(
    thing_id: String,
    _jwt: JSONWebToken,
) -> Result<Option<Json<Thing>>, status::Custom<String>> {
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

#[post("/", data = "<data>")]
async fn post_things(
    data: Json<Device>,
    _jwt: JSONWebToken,
) -> Result<status::Created<Json<Thing>>, status::Custom<String>> {
    let device = data.0;
    let t = call!(Db.GetThing(device.id.to_owned()))
        .to_rocket("Error during db.get_thing", Status::InternalServerError)?;
    if t.is_some() {
        Err(status::Custom(
            Status::BadRequest,
            "Thing already added".to_owned(),
        ))
    } else {
        let t = call!(Db.CreateThing(device))
            .to_rocket("Error saving new thing", Status::InternalServerError)?;
        info!(
            "Successfully created new thing {}",
            t.title.clone().unwrap_or_else(|| "".to_owned())
        );
        Ok(status::Created::new("").body(Json(t)))
    }
}
