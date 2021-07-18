/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use rust_manifest_types::Manifest;

pub struct Addon {
    pub manifest: Manifest,
}

impl Addon {
    pub fn new(manifest: Manifest) -> Self {
        Self { manifest }
    }
}
