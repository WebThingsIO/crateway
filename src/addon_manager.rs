/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon::Addon,
    addon_instance::{self, AddonInstance},
    db::{Db, GetSetting, SetSetting, SetSettingIfNotExists},
    macros::call,
    process_manager::{ProcessManager, StartAddon, StopAddon},
    user_config,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use flate2::read::GzDecoder;
use fs_extra::{dir::CopyOptions, move_items};
use log::{error, info};
use rust_manifest_types::Manifest;
use serde_json::json;
use sha256::digest_bytes;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    marker::PhantomData,
    path::PathBuf,
};
use tar::Archive;
use tempdir::TempDir;
use webthings_gateway_ipc_types::Device as DeviceDescription;
use xactor::{message, Actor, Addr, Context, Handler, Service};

#[derive(Default)]
pub struct AddonManager {
    installed_addons: HashMap<String, Addon>,
    running_addons: HashMap<String, Addr<AddonInstance>>,
}

impl AddonManager {
    async fn load_addon(&mut self, path: PathBuf) -> Result<()> {
        let file = fs::File::open(path.join("manifest.json")).context(anyhow!(
            "Could not open manifest.json file in {:?} found",
            path,
        ))?;
        let manifest: Manifest =
            serde_json::from_reader(file).context(anyhow!("Could not read manifest.json"))?;

        let mut addon = Addon::new(manifest, path);
        let addon_id = addon.id().to_owned();
        let path = addon.path.to_owned();
        let exec = addon.exec().to_owned();
        let enabled_key = format!("addons.{}.enabled", addon_id);
        let config_key = format!("addons.{}.config", addon_id);
        call!(Db.SetSettingIfNotExists(enabled_key.to_owned(), false))?;
        call!(Db.SetSettingIfNotExists(config_key.to_owned(), json!({})))?;
        let addon_enabled = call!(Db.GetSetting::<bool>(enabled_key, PhantomData))?;
        addon.enabled = addon_enabled;
        self.installed_addons.insert(addon_id.to_owned(), addon);
        if !addon_enabled {
            warn!("Addon not enabled: {}", addon_id);
            return Ok(());
        }
        info!("Loading add-on {}", addon_id);
        call!(ProcessManager.StartAddon(addon_id.to_owned(), path, exec))?;
        Ok(())
    }

    async fn unload_addon(&mut self, id: String) -> Result<()> {
        call!(ProcessManager.StopAddon(id.clone()))
    }

    async fn addon_enabled(&mut self, id: String) -> Result<bool> {
        let addon = self
            .installed_addons
            .get(&id)
            .ok_or_else(|| anyhow!("Package {} not installed", id))?;
        Ok(addon.enabled)
    }
    async fn install_addon(
        &mut self,
        package_id: String,
        package_path: PathBuf,
        enable: bool,
    ) -> Result<()> {
        if !package_path.is_file() {
            return Err(anyhow!(format!(
                "Cannot extract invalid path: {:?}",
                package_path,
            )));
        }

        info!("Expanding add-on {:?}", package_path);

        let package_dir = package_path
            .parent()
            .ok_or_else(|| anyhow!("Missing parent directory"))?;

        let file = File::open(package_path.to_owned()).map_err(|err| anyhow!(err))?;
        Archive::new(GzDecoder::new(file))
            .unpack(package_dir)
            .context("Failed to extract package")?;

        self.uninstall_addon(package_id.to_owned(), false).await?;

        let addon_path = user_config::ADDONS_DIR.join(package_id.to_owned());
        let entries: Vec<PathBuf> = package_dir
            .join("package")
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .collect();
        fs::create_dir(addon_path.to_owned())?;
        move_items(&entries, addon_path.to_owned(), &CopyOptions::new())
            .context("Failed to move package")?;

        let enabled_key = format!("addons.{}.enabled", package_id);
        if enable {
            call!(Db.SetSetting(enabled_key, true))?;
        }

        self.load_addon(addon_path).await?;

        Ok(())
    }

    async fn uninstall_addon(&mut self, package_id: String, disable: bool) -> Result<()> {
        if let Err(err) = self.unload_addon(package_id.to_owned()).await {
            error!("Failed to unload {} properly: {:?}", package_id, err);
        }

        let addon_path = user_config::ADDONS_DIR.join(package_id.to_owned());
        if addon_path.exists() && addon_path.is_dir() {
            fs::remove_dir_all(addon_path).context(format!("Error removing {}", package_id))?;
        }

        let enabled_key = format!("addons.{}.enabled", package_id);
        if disable {
            call!(Db.SetSetting(enabled_key, false))?;
        }

        self.installed_addons.remove(&package_id);

        Ok(())
    }
}

impl Actor for AddonManager {}

impl Service for AddonManager {}

#[message(result = "Result<()>")]
pub struct LoadAddons(pub PathBuf);

#[async_trait]
impl Handler<LoadAddons> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        LoadAddons(addon_dir): LoadAddons,
    ) -> Result<()> {
        info!("Loading addons from {:?}", addon_dir);
        let entries = fs::read_dir(addon_dir)
            .context(anyhow!("Could not load addons"))?
            .filter_map(|read_dir| {
                if let Err(err) = &read_dir {
                    error!("Could not enumerate addon dir entry: {}", err);
                }
                read_dir.map(|entry| entry.path()).ok()
            });
        let mut res = Ok(());
        for path in entries {
            if let Err(err) = self.load_addon(path.clone()).await {
                error!("Failed to load addon from {:?}: {:?}", path, err);
                res = Err(anyhow!("Failed to load some addons!"))
            }
        }
        info!("Finished loading addons");
        res
    }
}

#[message(result = "Result<()>")]
pub struct RestartAddon(pub String);

#[async_trait]
impl Handler<RestartAddon> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        RestartAddon(id): RestartAddon,
    ) -> Result<()> {
        self.unload_addon(id.to_owned()).await?;
        if self.addon_enabled(id.to_owned()).await? {
            self.load_addon(user_config::ADDONS_DIR.join(id)).await?;
        }
        Ok(())
    }
}

#[message(result = "()")]
pub struct AddonStarted(pub String, pub Addr<AddonInstance>);

#[async_trait]
impl Handler<AddonStarted> for AddonManager {
    async fn handle(&mut self, _ctx: &mut Context<Self>, AddonStarted(id, addr): AddonStarted) {
        self.running_addons.insert(id, addr);
    }
}

#[message(result = "()")]
pub struct AddonStopped(pub String);

#[async_trait]
impl Handler<AddonStopped> for AddonManager {
    async fn handle(&mut self, _ctx: &mut Context<Self>, AddonStopped(id): AddonStopped) {
        self.running_addons.remove(&id);
    }
}

#[message(result = "Result<()>")]
pub struct EnableAddon(pub String);

#[async_trait]
impl Handler<EnableAddon> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        EnableAddon(id): EnableAddon,
    ) -> Result<()> {
        let addon = self
            .installed_addons
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Package {} not installed", id))?;
        if addon.enabled {
            bail!("Addon {} already enabled!", id)
        }
        let enabled_key = format!("addons.{}.enabled", id);
        addon.enabled = true;
        call!(Db.SetSetting(enabled_key, true))?;
        let path = addon.path.clone();

        self.load_addon(path)
            .await
            .context(anyhow!("Failed to load addon {}", id))?;
        Ok(())
    }
}

#[message(result = "Result<()>")]
pub struct DisableAddon(pub String);

#[async_trait]
impl Handler<DisableAddon> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        DisableAddon(id): DisableAddon,
    ) -> Result<()> {
        let addon = self
            .installed_addons
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Package {} not installed", id))?;

        if !addon.enabled {
            bail!("Addon {} already disabled!", id)
        }
        let enabled_key = format!("addons.{}.enabled", id);
        addon.enabled = false;
        call!(Db.SetSetting(enabled_key, false))?;
        self.unload_addon(id.to_owned())
            .await
            .context(anyhow!("Failed to unload addon {}", id))?;

        Ok(())
    }
}

#[message(result = "Result<HashMap<String, Addon>>")]
pub struct GetAddons;

#[async_trait]
impl Handler<GetAddons> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        _msg: GetAddons,
    ) -> Result<HashMap<String, Addon>> {
        Ok(self.installed_addons.clone())
    }
}

#[message(result = "Result<Addon>")]
pub struct GetAddon(pub String);

#[async_trait]
impl Handler<GetAddon> for AddonManager {
    async fn handle(&mut self, _ctx: &mut Context<Self>, GetAddon(id): GetAddon) -> Result<Addon> {
        self.installed_addons
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("Unknown addon"))
    }
}

#[message(result = "Result<bool>")]
pub struct HasAddon(pub String);

#[async_trait]
impl Handler<HasAddon> for AddonManager {
    async fn handle(&mut self, _ctx: &mut Context<Self>, HasAddon(id): HasAddon) -> Result<bool> {
        Ok(self.installed_addons.contains_key(&id))
    }
}

#[message(result = "Result<()>")]
pub struct UninstallAddon(pub String);

#[async_trait]
impl Handler<UninstallAddon> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        UninstallAddon(addon_id): UninstallAddon,
    ) -> Result<()> {
        if let Err(err) = self.unload_addon(addon_id.to_owned()).await {
            error!("Failed to unload {} properly: {:?}", addon_id, err);
        }
        let addon_path = user_config::ADDONS_DIR.join(addon_id.to_owned());
        if addon_path.exists() && addon_path.is_dir() {
            fs::remove_dir_all(addon_path).context(format!("Error removing {}", addon_id))?;
        }
        let enabled_key = format!("addons.{}.enabled", addon_id);
        call!(Db.SetSetting(enabled_key, false))
            .context(format!("Failed to disable {}", addon_id))?;
        self.installed_addons.remove(&addon_id);
        Ok(())
    }
}

#[message(result = "Result<()>")]
pub struct InstallAddonFromUrl(pub String, pub String, pub String, pub bool);

#[async_trait]
impl Handler<InstallAddonFromUrl> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        InstallAddonFromUrl(id, url, checksum, enable): InstallAddonFromUrl,
    ) -> Result<()> {
        let temp_dir = TempDir::new(&id)?;
        let dest_path = temp_dir.path().join(format!("{}.tar.gz", id));

        info!("Fetching add-on {} as {:?}", url, dest_path);
        let res = reqwest::get(&url).await?.bytes().await?;
        let mut file = File::create(dest_path.clone())?;
        file.write_all(res.as_ref())?;

        if digest_bytes(res.as_ref()) != checksum.to_lowercase() {
            return Err(anyhow!(format!(
                "Checksum did not match for add-on: {}",
                id,
            )));
        }
        self.install_addon(id, dest_path, enable).await?;
        Ok(())
    }
}

#[message(result = "Result<HashMap<String, DeviceDescription>>")]
pub struct GetDevices;

#[async_trait]
impl Handler<GetDevices> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        _msg: GetDevices,
    ) -> Result<HashMap<String, DeviceDescription>> {
        let mut devices = HashMap::new();
        for (_, instance) in &self.running_addons {
            devices.extend(
                instance
                    .call(addon_instance::GetDevices)
                    .await
                    .map_err(|err| anyhow!(err))
                    .flatten()?,
            );
        }
        Ok(devices)
    }
}
