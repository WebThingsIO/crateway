/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use rocket::serde::Deserialize;

#[derive(Deserialize)]
pub struct AddonConfiguration {
    pub enabled: bool,
}

impl AddonConfiguration {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}
