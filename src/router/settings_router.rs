use crate::{config::CONFIG, jwt::JSONWebToken, platform};
use rocket::{response::status, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

pub fn routes() -> Vec<Route> {
    routes![get_language, get_units, get_timezone, get_addons_info]
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CurrentLanguage {
    pub current: String,
    pub valid: Vec<Language>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Language {
    pub code: String,
    pub name: String,
}

#[get("/localization/language")]
fn get_language(_jwt: JSONWebToken) -> Json<CurrentLanguage> {
    Json(CurrentLanguage {
        current: String::from("en-US"),
        valid: vec![Language {
            code: String::from("en-US"),
            name: String::from("English (United States of America)"),
        }],
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Units {
    pub temperature: String,
}

#[get("/localization/units")]
fn get_units(_jwt: JSONWebToken) -> Json<Units> {
    Json(Units {
        temperature: String::from("degree celsius"),
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CurrentTimezone {
    pub current: String,
    pub set_implemented: bool,
    pub valid: Vec<String>,
}

#[get("/localization/timezone")]
fn get_timezone(_jwt: JSONWebToken) -> Json<CurrentTimezone> {
    Json(CurrentTimezone {
        current: String::from("Europe/Berlin"),
        set_implemented: true,
        valid: vec![String::from("Europe/Berlin")],
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AddonsInfo {
    pub urls: Vec<String>,
    pub architecture: String,
    pub version: String,
    pub node_version: u32,
    pub python_versions: Vec<String>,
}

#[get("/addonsInfo")]
fn get_addons_info(_jwt: JSONWebToken) -> Result<Json<AddonsInfo>, status::Custom<String>> {
    Ok(Json(AddonsInfo {
        urls: CONFIG.addon_manager.list_urls.clone(),
        architecture: platform::ARCHITECTURE.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        node_version: platform::NODE_VERSION.to_owned(),
        python_versions: platform::PYTHON_VERSIONS.to_owned(),
    }))
}
