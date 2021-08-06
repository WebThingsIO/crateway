/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon::Addon,
    addon_instance::AddonInstance,
    macros::{bail_fut, try_fut},
    process_manager::{ProcessManager, StartAddon, StopAddon},
};
use actix::prelude::*;
use actix::{Actor, Context};
use anyhow::{anyhow, Context as AnyhowContext, Error};
use futures::future::join_all;
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
#[rtype(result = "Result<(), Error>")]
pub struct LoadAddons {
    pub addon_dir: PathBuf,
}

impl Handler<LoadAddons> for AddonManager {
    type Result = ResponseFuture<Result<(), Error>>;

    fn handle(
        &mut self,
        LoadAddons { addon_dir }: LoadAddons,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        info!("Loading addons from {:?}", addon_dir);
        let (futures, paths): (Vec<_>, Vec<_>) =
            try_fut!(fs::read_dir(addon_dir).context(anyhow!("Could not load addons")))
                .filter_map(|read_dir| {
                    if let Err(err) = &read_dir {
                        error!("Could not enumerate addon dir entry: {}", err);
                    }
                    read_dir.ok()
                })
                .map(|dir_entry| {
                    (
                        Self::from_registry().send(LoadAddon {
                            path: dir_entry.path(),
                        }),
                        dir_entry.path(),
                    )
                })
                .unzip();

        Box::pin(async move {
            join_all(futures)
                .await
                .into_iter()
                .zip(paths)
                .map(|(result, path)| {
                    result
                        .context(anyhow!("Failed to send load addon message for {:?}", path))?
                        .context(anyhow!("Faild to load addon from {:?}", path))?;
                    info!("Loaded addon from {:?}", path);
                    Ok(())
                })
                .collect::<Result<Vec<()>, Error>>()?;
            info!("Finished loading addons");
            Ok(())
        })
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct LoadAddon {
    pub path: PathBuf,
}

impl Handler<LoadAddon> for AddonManager {
    type Result = ResponseFuture<Result<(), Error>>;

    fn handle(&mut self, LoadAddon { path }: LoadAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let file = try_fut!(fs::File::open(path.join("manifest.json")).context(anyhow!(
            "Could not open manifest.json file in {:?} found",
            path,
        )));
        let manifest: Manifest = try_fut!(
            serde_json::from_reader(file).context(anyhow!("Could not read manifest.json"))
        );

        let addon = Addon::new(manifest, path, true);
        let addon_id = addon.id().to_owned();
        let addon_enabled = addon.enabled;
        let path = addon.path.to_owned();
        let exec = addon.exec().to_owned();
        self.installed_addons.insert(addon_id.to_owned(), addon);
        if !addon_enabled {
            bail_fut!("Addon not enabled: {}", addon_id)
        }
        info!("Loading add-on {}", addon_id);
        Box::pin(async move {
            ProcessManager::from_registry()
                .send(StartAddon {
                    path,
                    id: addon_id.to_owned(),
                    exec,
                })
                .await
                .context(anyhow!("Failed to start addon {}", addon_id))?
                .context(anyhow!("Failed to start addon {}", addon_id))?;
            Ok(())
        })
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
#[rtype(result = "Result<(), Error>")]
pub struct EnableAddon(pub String);

impl Handler<EnableAddon> for AddonManager {
    type Result = ResponseFuture<Result<(), Error>>;

    fn handle(&mut self, msg: EnableAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let EnableAddon(id) = msg;
        let addon = try_fut!(self
            .installed_addons
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Package {} not installed", id)));
        if addon.enabled {
            bail_fut!("Addon {} already enabled!", id)
        }
        addon.enabled = true;
        let path = addon.path.clone();

        Box::pin(async move {
            Self::from_registry()
                .send(LoadAddon { path })
                .await
                .context(anyhow!("Failed to load addon {}", id))?
                .context(anyhow!("Failed to load addon {}", id))?;
            Ok(())
        })
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct DisableAddon(pub String);

impl Handler<DisableAddon> for AddonManager {
    type Result = ResponseFuture<Result<(), Error>>;

    fn handle(&mut self, msg: DisableAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let DisableAddon(id) = msg;
        let addon = try_fut!(self
            .installed_addons
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Package {} not installed", id)));

        if !addon.enabled {
            bail_fut!("Addon {} already disabled!", id)
        }
        addon.enabled = false;
        Box::pin(async move {
            ProcessManager::from_registry()
                .send(StopAddon { id: id.to_owned() })
                .await
                .context(anyhow!("Failed to stop addon {}", id))?
                .context(anyhow!("Failed to stop addon {}", id))?;
            Ok(())
        })
    }
}
