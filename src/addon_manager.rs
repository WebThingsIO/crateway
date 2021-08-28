/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon::Addon,
    addon_instance::AddonInstance,
    db::{Db, GetSetting, SetSetting, SetSettingIfNotExists},
    macros::call,
    process_manager::{ProcessManager, StartAddon, StopAddon},
    user_config,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Error};
use log::{error, info};
use rust_manifest_types::Manifest;
use serde_json::json;
use std::{collections::HashMap, fs, marker::PhantomData, path::PathBuf};
use xactor::{message, Actor, Addr, Context, Handler, Service};

#[derive(Default)]
pub struct AddonManager {
    installed_addons: HashMap<String, Addon>,
    running_addons: HashMap<String, Addr<AddonInstance>>,
}

impl AddonManager {
    async fn load_addon(&mut self, path: PathBuf) -> Result<(), Error> {
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
            bail!("Addon not enabled: {}", addon_id)
        }
        info!("Loading add-on {}", addon_id);
        call!(ProcessManager.StartAddon(addon_id.to_owned(), path, exec))?;
        Ok(())
    }

    async fn unload_addon(&mut self, id: String) -> Result<(), Error> {
        call!(ProcessManager.StopAddon(id.clone()))
    }

    async fn addon_enabled(&mut self, id: String) -> Result<bool, Error> {
        let addon = self
            .installed_addons
            .get(&id)
            .ok_or_else(|| anyhow!("Package {} not installed", id))?;
        Ok(addon.enabled)
    }
}

impl Actor for AddonManager {}

impl Service for AddonManager {}

#[message(result = "Result<(), Error>")]
pub struct LoadAddons(pub PathBuf);

#[async_trait]
impl Handler<LoadAddons> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        LoadAddons(addon_dir): LoadAddons,
    ) -> Result<(), Error> {
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

#[message(result = "Result<(), Error>")]
pub struct RestartAddon(pub String);

#[async_trait]
impl Handler<RestartAddon> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        RestartAddon(id): RestartAddon,
    ) -> Result<(), Error> {
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

#[message(result = "Result<(), Error>")]
pub struct EnableAddon(pub String);

#[async_trait]
impl Handler<EnableAddon> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        EnableAddon(id): EnableAddon,
    ) -> Result<(), Error> {
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

#[message(result = "Result<(), Error>")]
pub struct DisableAddon(pub String);

#[async_trait]
impl Handler<DisableAddon> for AddonManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        DisableAddon(id): DisableAddon,
    ) -> Result<(), Error> {
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
