use rocket::{serde::json::Json, Route};
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<Route> {
    routes![get_user_count]
}

#[derive(Serialize, Deserialize)]
struct UserCount {
    count: u32,
}

#[get("/count")]
fn get_user_count() -> Json<UserCount> {
    Json(UserCount { count: 1 })
}
