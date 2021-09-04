use crate::{
    addon::Addon,
    addon_manager::{
        AddonManager, DisableAddon, EnableAddon, GetAddon, GetAddons, InstallAddonFromUrl,
        RestartAddon, UninstallAddon,
    },
    db::{Db, GetSetting, SetSetting},
    jwt::JSONWebToken,
    macros::{call, ToRocket},
    user_config,
};
use regex::Regex;
use rocket::{
    http::Status,
    response::status,
    serde::{json::Json, Deserialize, Serialize},
    Route,
};
use rust_manifest_types::Manifest;
use std::{ffi::OsStr, fs, marker::PhantomData};

pub fn routes() -> Vec<Route> {
    routes![
        get_addons,
        put_addon,
        put_addon_config,
        get_addon_config,
        get_addon_license,
        delete_addon,
        post_addons,
        patch_addon,
    ]
}

#[get("/")]
async fn get_addons(
    _jwt: JSONWebToken,
) -> Result<Json<Vec<AddonResponse>>, status::Custom<String>> {
    let addons =
        call!(AddonManager.GetAddons).to_rocket("Failed to get addons", Status::BadRequest)?;
    Ok(Json(
        addons.values().cloned().map(AddonResponse::from).collect(),
    ))
}

#[derive(Serialize, Deserialize)]
struct AddonEnabledState {
    enabled: bool,
}

#[put("/<addon_id>", data = "<data>")]
async fn put_addon(
    addon_id: String,
    data: Json<AddonEnabledState>,
    _jwt: JSONWebToken,
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
struct AddonConfig {
    config: serde_json::Value,
}

#[put("/<addon_id>/config", data = "<data>")]
async fn put_addon_config(
    addon_id: String,
    data: Json<AddonConfig>,
    _jwt: JSONWebToken,
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

#[get("/<addon_id>/config")]
async fn get_addon_config(
    addon_id: String,
    _jwt: JSONWebToken,
) -> Result<Json<serde_json::Value>, status::Custom<String>> {
    let config_key = format!("addons.{}.config", addon_id);
    let config = call!(Db.GetSetting(config_key, PhantomData))
        .to_rocket("Failed to get addon config", Status::BadRequest)?;
    Ok(Json(config))
}

#[get("/<addon_id>/license")]
async fn get_addon_license(
    addon_id: String,
    _jwt: JSONWebToken,
) -> Result<String, status::Custom<String>> {
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

#[delete("/<addon_id>")]
async fn delete_addon(
    addon_id: String,
    _jwt: JSONWebToken,
) -> Result<status::NoContent, status::Custom<String>> {
    call!(AddonManager.UninstallAddon(addon_id.to_owned()))
        .to_rocket("Failed to uninstall add-on", Status::BadRequest)?;
    Ok(status::NoContent)
}

#[derive(Deserialize)]
struct InstallableAddon {
    id: String,
    url: String,
    checksum: String,
}

#[post("/", data = "<data>")]
async fn post_addons(
    data: Json<InstallableAddon>,
    _jwt: JSONWebToken,
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

#[patch("/<addon_id>", data = "<data>")]
async fn patch_addon(
    addon_id: String,
    data: Json<AddonOrigin>,
    _jwt: JSONWebToken,
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
