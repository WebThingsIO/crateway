use crate::{
    db::{Db, GetUser},
    jwt,
    macros::{call, ToRocket},
    model::Jwt,
};
use rocket::{http::Status, response::status, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<Route> {
    routes![login]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[post("/", data = "<data>")]
async fn login(data: Json<Login>) -> Result<Json<Jwt>, status::Custom<String>> {
    let user = call!(Db.GetUser::ByEmail(data.0.email))
        .to_rocket("Failed to get user", Status::InternalServerError)?;
    if let Some(user) = user {
        if !user
            .verify_password(&data.0.password)
            .to_rocket("Unauthorized", Status::Unauthorized)?
        {
            return Err(status::Custom(
                Status::Unauthorized,
                "Unauthorized".to_owned(),
            ));
        }

        let jwt = jwt::issue_token(user.id)
            .await
            .to_rocket("Failed to issue token", Status::InternalServerError)?;
        Ok(Json(Jwt { jwt }))
    } else {
        Err(status::Custom(
            Status::Unauthorized,
            "Unauthorized".to_owned(),
        ))
    }
}
