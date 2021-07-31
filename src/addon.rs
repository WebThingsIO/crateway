/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon_configuration::AddonConfiguration,
    process_manager::{ProcessManager, StartAddon, StopAddon},
};
use actix::prelude::*;
use rust_manifest_types::Manifest;
use std::path::PathBuf;

pub struct Addon {
    pub manifest: Manifest,
    pub path: PathBuf,
    pub config: AddonConfiguration,
}

impl Addon {
    pub fn new(manifest: Manifest, path: PathBuf, config: AddonConfiguration) -> Self {
        Self {
            manifest,
            path,
            config,
        }
    }

    pub fn exec(&self) -> &str {
        &self.manifest.gateway_specific_settings.webthings.exec
    }

    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    pub fn start(&self) {
        if !self.config.enabled {
            error!("Addon not enabled: {}", self.id());
            return;
        }
        info!("Starting add-on {}", self.id());
        ProcessManager::from_registry().do_send(StartAddon {
            path: self.path.clone(),
            id: self.id().to_owned(),
            exec: self.exec().to_owned(),
        });
    }

    pub fn stop(&self) {
        info!("Stopping add-on {}", self.id());
        ProcessManager::from_registry().do_send(StopAddon {
            id: self.id().to_owned(),
        });
    }
}
