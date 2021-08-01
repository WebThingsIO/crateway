/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon::Addon,
    addon_instance::AddonInstance,
    process_manager::{ProcessManager, StartAddon, StopAddon},
};
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
    type Result = ResponseFuture<()>;

    fn handle(
        &mut self,
        LoadAddons { addon_dir }: LoadAddons,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        info!("Loading addons from {:?}", addon_dir);
        let iter = match fs::read_dir(addon_dir) {
            Ok(read_dir) => read_dir,
            Err(err) => {
                error!("Could not load addons: {}", err);
                return Box::pin(async { () });
            }
        }
        .filter_map(|read_dir| {
            if let Err(err) = &read_dir {
                error!("Could not enumerate addon dir entry: {}", err);
            }
            read_dir.ok()
        });

        return Box::pin(async move {
            for dir_entry in iter {
                match Self::from_registry()
                    .send(LoadAddon {
                        path: dir_entry.path(),
                    })
                    .await
                {
                    Err(_) | Ok(Err(_)) => {
                        error!("Failed to load addon from {:?}", dir_entry.path());
                    }
                    Ok(Ok(_)) => {
                        info!("Loaded addon from {:?}", dir_entry.path());
                    }
                }
            }
            info!("Finished loading addons");
        });
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
pub struct LoadAddon {
    pub path: PathBuf,
}

impl Handler<LoadAddon> for AddonManager {
    type Result = ResponseFuture<Result<(), ()>>;

    fn handle(&mut self, LoadAddon { path }: LoadAddon, _ctx: &mut Context<Self>) -> Self::Result {
        match fs::File::open(path.join("manifest.json")) {
            Ok(file) => match serde_json::from_reader(file) {
                Ok(manifest) => {
                    let manifest: Manifest = manifest;
                    let addon = Addon::new(manifest, path, true);
                    let addon_id = addon.id().to_owned();
                    self.installed_addons.insert(addon_id.to_owned(), addon);
                    let addon = self.installed_addons.get(&addon_id).unwrap();
                    if addon.enabled {
                        let path = addon.path.to_owned();
                        let exec = addon.exec().to_owned();
                        info!("Loading add-on {}", addon_id);
                        return Box::pin(async move {
                            match ProcessManager::from_registry()
                                .send(StartAddon {
                                    path,
                                    id: addon_id.to_owned(),
                                    exec,
                                })
                                .await
                            {
                                Err(_) | Ok(Err(_)) => {
                                    error!("Failed to start addon {}", addon_id);
                                    Err(())
                                }
                                Ok(Ok(())) => Ok(()),
                            }
                        });
                    } else {
                        error!("Addon not enabled: {}", addon.id());
                    }
                }
                Err(err) => error!("Could not read manifest.json: {}", err),
            },
            Err(err) => error!(
                "Could not open manifest.json file in {:?} found: {}",
                path, err
            ),
        }
        return Box::pin(async { Err(()) });
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
#[rtype(result = "Result<(), ()>")]
pub struct EnableAddon(pub String);

impl Handler<EnableAddon> for AddonManager {
    type Result = ResponseFuture<Result<(), ()>>;

    fn handle(&mut self, msg: EnableAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let EnableAddon(id) = msg;
        match self.installed_addons.get_mut(&id) {
            Some(addon) => {
                if addon.enabled {
                    error!("Addon {} already enabled!", id);
                    return Box::pin(async { Err(()) });
                }
                addon.enabled = true;
                let path = addon.path.clone();

                return Box::pin(async move {
                    match Self::from_registry().send(LoadAddon { path: path }).await {
                        Err(_) | Ok(Err(_)) => {
                            error!("Failed to load addon {}", id);
                            Err(())
                        }
                        Ok(Ok(())) => Ok(()),
                    }
                });
            }
            None => {
                error!("Package {} not installed", id);
                Box::pin(async { Err(()) })
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
pub struct DisableAddon(pub String);

impl Handler<DisableAddon> for AddonManager {
    type Result = ResponseFuture<Result<(), ()>>;

    fn handle(&mut self, msg: DisableAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let DisableAddon(id) = msg;
        match self.installed_addons.get_mut(&id) {
            Some(addon) => {
                if !addon.enabled {
                    error!("Addon {} already disabled!", id);
                    return Box::pin(async { Err(()) });
                }
                addon.enabled = false;
                Box::pin(async move {
                    match ProcessManager::from_registry()
                        .send(StopAddon { id: id.to_owned() })
                        .await
                    {
                        Err(_) | Ok(Err(_)) => {
                            error!("Failed to stop addon {}", id);
                            Err(())
                        }
                        Ok(Ok(())) => Ok(()),
                    }
                })
            }
            None => {
                error!("Package {} not installed", id);
                Box::pin(async { Err(()) })
            }
        }
    }
}
