use crate::jwt::JSONWebToken;
use rocket::{serde::json::Json, Route};
use std::collections::BTreeMap;

pub fn routes() -> Vec<Route> {
    routes![get_extensions]
}

#[get("/")]
fn get_extensions(_jwt: JSONWebToken) -> Json<BTreeMap<String, String>> {
    Json(BTreeMap::new())
}
