/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use rust_manifest_types::Manifest;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Addon {
    pub manifest: Manifest,
    pub path: PathBuf,
    pub enabled: bool,
}

impl Addon {
    pub fn new(manifest: Manifest, path: PathBuf) -> Self {
        Self {
            manifest,
            path,
            enabled: false,
        }
    }

    pub fn exec(&self) -> &str {
        &self.manifest.gateway_specific_settings.webthings.exec
    }

    pub fn id(&self) -> &str {
        &self.manifest.id
    }
}
