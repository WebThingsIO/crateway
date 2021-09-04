use crate::{
    db::{CreateUser, Db, DeleteUser, EditUser, GetUser, GetUserCount, GetUsers},
    jwt::{self, JSONWebToken},
    macros::{call, ToRocket},
    model::{Jwt, User},
};
use rocket::{http::Status, response::status, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<Route> {
    routes![
        get_user_count,
        get_user_info,
        get_user,
        post_user,
        put_user,
        delete_user
    ]
}

#[derive(Serialize, Deserialize)]
struct UserCount {
    count: i64,
}

#[get("/count")]
async fn get_user_count() -> Result<Json<UserCount>, status::Custom<String>> {
    let count =
        call!(Db.GetUserCount).to_rocket("Failed to obtain user count", Status::BadRequest)?;
    Ok(Json(UserCount { count }))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserWithLoggedInState {
    #[serde(flatten)]
    user: User,
    logged_in: bool,
}

#[get("/info")]
async fn get_user_info(
    jwt: JSONWebToken,
) -> Result<Json<Vec<UserWithLoggedInState>>, status::Custom<String>> {
    let users = call!(Db.GetUsers).to_rocket("Failed to get users", Status::BadRequest)?;
    Ok(Json(
        users
            .into_iter()
            .map(|user| UserWithLoggedInState {
                logged_in: user.id == jwt.user_id(),
                user: user,
            })
            .collect(),
    ))
}

#[get("/<user_id>")]
async fn get_user(user_id: i64, _jwt: JSONWebToken) -> Result<Json<User>, status::Custom<String>> {
    let user =
        call!(Db.GetUser::ById(user_id)).to_rocket("Failed to get user", Status::BadRequest)?;
    if let Some(user) = user {
        Ok(Json(user))
    } else {
        Err(status::Custom(Status::NotFound, "Unknown user".to_owned()))
    }
}

#[derive(Serialize, Deserialize)]
struct UserForCreate {
    email: String,
    password: String,
    name: String,
}

#[post("/", data = "<data>")]
async fn post_user(
    data: Json<UserForCreate>,
    jwt: Result<JSONWebToken, &str>,
) -> Result<Json<Jwt>, status::Custom<String>> {
    let count =
        call!(Db.GetUserCount).to_rocket("Failed to obtain user count", Status::BadRequest)?;
    if count > 0 && jwt.is_err() {
        return Err(status::Custom(
            Status::Unauthorized,
            "Unauthorized".to_owned(),
        ));
    }
    let UserForCreate {
        email,
        password,
        name,
    } = data.0;
    let user = call!(Db.GetUser::ByEmail(email.to_owned()))
        .to_rocket("Failed to get user".to_owned(), Status::BadRequest)?;
    if let Some(_) = user {
        Err(status::Custom(
            Status::BadRequest,
            "User already exists".to_owned(),
        ))
    } else {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .to_rocket("Failed to hash password", Status::BadRequest)?;
        let user_id = call!(Db.CreateUser(email.to_owned(), hash, name))
            .to_rocket("Failed to create user", Status::BadRequest)?;
        let jwt = jwt::issue_token(user_id)
            .await
            .to_rocket("Failed to issue token", Status::BadRequest)?;
        Ok(Json(Jwt { jwt }))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserForEdit {
    email: String,
    password: String,
    new_password: Option<String>,
    name: String,
}

#[put("/<user_id>", data = "<data>")]
async fn put_user(
    user_id: i64,
    data: Json<UserForEdit>,
    _jwt: JSONWebToken,
) -> Result<status::NoContent, status::Custom<String>> {
    let user =
        call!(Db.GetUser::ById(user_id)).to_rocket("Failed to get user", Status::BadRequest)?;
    if let Some(mut user) = user {
        if bcrypt::verify(data.0.password, &user.password)
            .to_rocket("Failed to verify password hash", Status::BadRequest)?
        {
            return Err(status::Custom(
                Status::BadRequest,
                "Passwords do not match".to_owned(),
            ));
        }

        if let Some(new_password) = data.0.new_password {
            user.password = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
                .to_rocket("Failed to hash new password", Status::BadRequest)?;
        }
        user.email = data.0.email;
        user.name = data.0.name;

        call!(Db.EditUser(user)).to_rocket("Failed to edit user", Status::BadRequest)?;

        Ok(status::NoContent)
    } else {
        Err(status::Custom(
            Status::NotFound,
            "User not found".to_owned(),
        ))
    }
}

#[delete("/<user_id>")]
async fn delete_user(
    user_id: i64,
    _jwt: JSONWebToken,
) -> Result<status::NoContent, status::Custom<String>> {
    call!(Db.DeleteUser(user_id)).to_rocket("Failed to delete user", Status::BadRequest)?;
    Ok(status::NoContent)
}
