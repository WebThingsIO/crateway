use rocket::{serde::json::Json, Route};
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<Route> {
    routes![login]
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

#[post("/", data = "<data>")]
fn login(data: Json<Login>) -> Json<Jwt> {
    Json(Jwt {
        jwt: format!("{}:{}", data.email, data.password),
    })
}
