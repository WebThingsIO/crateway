use crate::{
    db::{Db, GetThing, GetThings},
    jwt::JSONWebToken,
    macros::{call, ToRocket},
    model::Thing,
};
use rocket::{http::Status, response::status, serde::json::Json, Route};

pub fn routes() -> Vec<Route> {
    routes![get_things, get_thing]
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
