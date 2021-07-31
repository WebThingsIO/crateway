/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{addon::Addon, addon_configuration::AddonConfiguration, addon_instance::AddonInstance};
use actix::prelude::*;
use actix::{Actor, Context};
use log::{error, info};
use rust_manifest_types::Manifest;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Default)]
pub struct AddonManager {
    installed_addons: HashMap<String, Addon>,
    running_addons: HashMap<String, Addr<AddonInstance>>,
}

impl Actor for AddonManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("AddonManager started");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        info!("AddonManager stopped");
    }
}

impl actix::Supervised for AddonManager {}

impl SystemService for AddonManager {
    fn service_started(&mut self, _ctx: &mut Context<Self>) {
        info!("AddonManager service started");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LoadAddons {
    pub addon_dir: PathBuf,
}

impl Handler<LoadAddons> for AddonManager {
    type Result = ();

    fn handle(
        &mut self,
        LoadAddons { addon_dir }: LoadAddons,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        info!("Loading addons from {:?}", addon_dir);
        self.load_addons(addon_dir);
        info!("Finished loading addons");
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddonStarted {
    pub id: String,
    pub addr: Addr<AddonInstance>,
}

impl Handler<AddonStarted> for AddonManager {
    type Result = ();

    fn handle(
        &mut self,
        AddonStarted { id, addr }: AddonStarted,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        self.running_addons.insert(id, addr);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddonStopped(pub String);

impl Handler<AddonStopped> for AddonManager {
    type Result = ();

    fn handle(&mut self, msg: AddonStopped, _ctx: &mut Context<Self>) -> Self::Result {
        self.running_addons.remove(&msg.0);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateAddonConfiguration(pub String, pub AddonConfiguration);

impl Handler<UpdateAddonConfiguration> for AddonManager {
    type Result = ();

    fn handle(&mut self, msg: UpdateAddonConfiguration, _ctx: &mut Context<Self>) -> Self::Result {
        let UpdateAddonConfiguration(id, config) = msg;
        let addon = self.installed_addons.get_mut(&id);
        match addon {
            Some(addon) => {
                addon.stop();
                addon.config = config;
                addon.start();
            }
            None => {
                error!("Package {} not installed", id)
            }
        }
    }
}

impl AddonManager {
    pub fn load_addons(&mut self, addon_dir: PathBuf) {
        match fs::read_dir(addon_dir) {
            Ok(read_dir) => read_dir,
            Err(err) => {
                error!("Could not load addons: {}", err);
                return;
            }
        }
        .filter_map(|read_dir| {
            if let Err(err) = &read_dir {
                error!("Could not enumerate addon dir entry: {}", err);
            }
            read_dir.ok()
        })
        .for_each(|dir_entry| self.load_addon(dir_entry.path()));
    }

    fn load_addon(&mut self, path: PathBuf) {
        match fs::File::open(path.join("manifest.json")) {
            Ok(file) => {
                if let Ok(manifest) = serde_json::from_reader(file) {
                    let manifest: Manifest = manifest;
                    let addon = Addon::new(manifest, path, AddonConfiguration::new(true));
                    info!("Loading add-on {}", addon.id());
                    addon.start();
                    self.installed_addons.insert(addon.id().to_owned(), addon);
                }
            }
            Err(err) => error!(
                "Could not open manifest.json file in {:?} found: {}",
                path, err
            ),
        }
    }
}
