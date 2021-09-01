use rocket::{serde::json::Json, Route};
use std::collections::BTreeMap;

pub fn routes() -> Vec<Route> {
    routes![get_extensions]
}

#[get("/")]
fn get_extensions() -> Json<BTreeMap<String, String>> {
    Json(BTreeMap::new())
}
