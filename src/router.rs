/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon::Addon,
    addon_manager::{
        AddonManager, DisableAddon, EnableAddon, GetAddon, GetAddons, InstallAddonFromUrl,
        RestartAddon, UninstallAddon,
    },
    config::CONFIG,
    db::{Db, GetSetting, GetThing, GetThings, SetSetting},
    macros::{call, ToRocket},
    model::Thing,
    platform, user_config,
};
use regex::Regex;
use rocket::{
    http::Status,
    response::status,
    serde::{json::Json, Deserialize, Serialize},
    Route,
};
use rust_manifest_types::Manifest;
use serde_json::json;
use std::{collections::BTreeMap, ffi::OsStr, fs, marker::PhantomData};

pub fn routes() -> Vec<Route> {
    routes![
        get_things,
        get_thing,
        put_addon,
        put_addon_config,
        get_addons,
        get_addon_config,
        get_addon_license,
        get_settings_addons_info,
        delete_addon,
        post_addons,
        patch_addon,
        get_user_count,
        get_language,
        get_units,
        get_timezone,
        login,
        ping,
        get_extensions
    ]
}

#[get("/things")]
async fn get_things() -> Result<Json<Vec<Thing>>, status::Custom<String>> {
    let t =
        call!(Db.GetThings).to_rocket("Error during db.get_things", Status::InternalServerError)?;

    Ok(Json(t))
}

#[get("/thing/<thing_id>")]
async fn get_thing(thing_id: String) -> Result<Option<Json<Thing>>, status::Custom<String>> {
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

#[derive(Serialize, Deserialize)]
struct AddonEnabledState {
    enabled: bool,
}

#[put("/addons/<addon_id>", data = "<data>")]
async fn put_addon(
    addon_id: String,
    data: Json<AddonEnabledState>,
) -> Result<Json<AddonEnabledState>, status::Custom<String>> {
    if data.0.enabled {
        call!(AddonManager.EnableAddon(addon_id))
            .to_rocket("Failed to enable addon", Status::InternalServerError)?;
    } else {
        call!(AddonManager.DisableAddon(addon_id))
            .to_rocket("Failed to disable addon", Status::InternalServerError)?;
    }
    Ok(Json(AddonEnabledState {
        enabled: data.0.enabled,
    }))
}

#[derive(Serialize, Deserialize)]
struct UserCount {
    count: u32,
}

#[get("/users/count")]
async fn get_user_count() -> Json<UserCount> {
    Json(UserCount { count: 1 })
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

#[get("/settings/localization/language")]
async fn get_language() -> Json<CurrentLanguage> {
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

#[get("/settings/localization/units")]
async fn get_units() -> Json<Units> {
    Json(Units {
        temperature: String::from("degree celsius"),
    })
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CurrentTimezone {
    pub current: String,
    #[serde(rename = "setImplemented")]
    pub set_implemented: bool,
    pub valid: Vec<String>,
}

#[get("/settings/localization/timezone")]
async fn get_timezone() -> Json<CurrentTimezone> {
    Json(CurrentTimezone {
        current: String::from("Europe/Berlin"),
        set_implemented: true,
        valid: vec![String::from("Europe/Berlin")],
    })
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

#[post("/login", data = "<data>")]
async fn login(data: Json<Login>) -> Json<Jwt> {
    Json(Jwt {
        jwt: format!("{}:{}", data.email, data.password),
    })
}

#[get("/ping")]
async fn ping() -> Status {
    Status::Ok
}

#[get("/extensions")]
async fn get_extensions() -> Json<BTreeMap<String, String>> {
    Json(BTreeMap::new())
}

#[derive(Serialize, Deserialize)]
struct AddonConfig {
    config: serde_json::Value,
}

#[put("/addons/<addon_id>/config", data = "<data>")]
async fn put_addon_config(
    addon_id: String,
    data: Json<AddonConfig>,
) -> Result<Json<AddonConfig>, status::Custom<String>> {
    let config_key = format!("addons.{}.config", addon_id);
    call!(Db.SetSetting(config_key, data.0.config.clone())).to_rocket(
        format!("Failed to update config for addon {}", addon_id),
        Status::BadRequest,
    )?;
    call!(AddonManager.RestartAddon(addon_id.to_owned())).to_rocket(
        format!("Failed to restart addon {}", addon_id),
        Status::BadRequest,
    )?;
    Ok(Json(data.0))
}

#[derive(Serialize)]
struct AddonResponse {
    #[serde(flatten)]
    pub manifest: Manifest,
    pub enabled: bool,
}

impl From<Addon> for AddonResponse {
    fn from(addon: Addon) -> AddonResponse {
        AddonResponse {
            manifest: addon.manifest,
            enabled: addon.enabled,
        }
    }
}

#[get("/addons")]
async fn get_addons() -> Result<Json<Vec<AddonResponse>>, status::Custom<String>> {
    let addons =
        call!(AddonManager.GetAddons).to_rocket("Failed to get addons", Status::BadRequest)?;
    Ok(Json(
        addons
            .values()
            .cloned()
            .map(|addon| AddonResponse::from(addon))
            .collect(),
    ))
}

#[get("/addons/<addon_id>/config")]
async fn get_addon_config(
    addon_id: String,
) -> Result<Json<serde_json::Value>, status::Custom<String>> {
    let config_key = format!("addons.{}.config", addon_id);
    let config = call!(Db.GetSetting(config_key, PhantomData))
        .to_rocket("Failed to get addon config", Status::BadRequest)?;
    Ok(Json(config))
}

#[get("/addons/<addon_id>/license")]
async fn get_addon_license(addon_id: String) -> Result<String, status::Custom<String>> {
    let addon_dir = user_config::ADDONS_DIR.join(addon_id.to_owned());
    let entries = fs::read_dir(addon_dir).to_rocket(
        "Failed to obtain license: Failed to access addon directory",
        Status::BadRequest,
    )?;
    let files: Vec<_> = entries
        .filter_map(|res| res.ok())
        .map(|entry| entry.path())
        .filter(|entry| entry.is_file())
        .filter(|file| {
            let name = file
                .file_name()
                .unwrap_or_else(|| OsStr::new(""))
                .to_str()
                .unwrap_or("");
            let re = Regex::new(r"^LICENSE(\..*)?$").unwrap();
            re.is_match(name)
        })
        .collect();
    if files.is_empty() {
        return Err(status::Custom(
            Status::BadRequest,
            "License not found".to_owned(),
        ));
    }
    fs::read_to_string(files[0].to_owned()).to_rocket(
        format!("Failed to read license file for addon {}", addon_id),
        Status::BadRequest,
    )
}

#[delete("/addons/<addon_id>")]
async fn delete_addon(addon_id: String) -> Result<status::NoContent, status::Custom<String>> {
    call!(AddonManager.UninstallAddon(addon_id.to_owned()))
        .to_rocket("Failed to uninstall add-on", Status::BadRequest)?;
    Ok(status::NoContent)
}

#[get("/settings/addonsInfo")]
async fn get_settings_addons_info() -> Result<Json<serde_json::Value>, status::Custom<String>> {
    Ok(Json(json!({
        "urls": CONFIG.addon_manager.list_urls,
        "architecture": platform::ARCHITECTURE.to_owned(),
        "version": env!("CARGO_PKG_VERSION"),
        "nodeVersion": platform::NODE_VERSION.to_owned(),
        "pythonVersions": platform::PYTHON_VERSIONS.to_owned(),
    })))
}

#[derive(Deserialize)]
struct InstallableAddon {
    id: String,
    url: String,
    checksum: String,
}

#[post("/addons", data = "<data>")]
async fn post_addons(
    data: Json<InstallableAddon>,
) -> Result<Json<AddonResponse>, status::Custom<String>> {
    let inst = data.0;
    let addon_id = inst.id.clone();
    call!(AddonManager.InstallAddonFromUrl(inst.id, inst.url, inst.checksum, true)).to_rocket(
        format!("Failed to install add-on {}", addon_id.clone()),
        Status::BadRequest,
    )?;
    let addon = call!(AddonManager.GetAddon(addon_id.to_owned())).to_rocket(
        format!("Failed to get addon {}", addon_id),
        Status::BadRequest,
    )?;
    Ok(Json(AddonResponse::from(addon)))
}

#[derive(Deserialize)]
struct AddonOrigin {
    url: String,
    checksum: String,
}

#[patch("/addons/<addon_id>", data = "<data>")]
async fn patch_addon(
    addon_id: String,
    data: Json<AddonOrigin>,
) -> Result<Json<AddonResponse>, status::Custom<String>> {
    let inst = data.0;
    call!(AddonManager.InstallAddonFromUrl(addon_id.clone(), inst.url, inst.checksum, false))
        .to_rocket(
            format!("Failed to update add-on {}", addon_id.clone()),
            Status::BadRequest,
        )?;
    let addon = call!(AddonManager.GetAddon(addon_id.to_owned())).to_rocket(
        format!("Failed to get addon {}", addon_id),
        Status::BadRequest,
    )?;
    Ok(Json(AddonResponse::from(addon)))
}
