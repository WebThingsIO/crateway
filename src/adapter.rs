/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::device::Device;
use std::collections::HashMap;
use webthings_gateway_ipc_types::Device as DeviceDescription;

pub struct Adapter {
    id: String,
    devices: HashMap<String, Device>,
}

impl Adapter {
    pub fn new(id: String) -> Self {
        Self {
            id,
            devices: HashMap::new(),
        }
    }

    pub fn add_device(&mut self, description: DeviceDescription) {
        let id = description.id.clone();

        match self.devices.get_mut(&id) {
            Some(device) => {
                device.update(description);
                info!("Device {} of adapter {} updated", self.id, id);
            }
            None => {
                let device = Device::new(description);
                self.devices.insert(id.clone(), device);
                info!("Device {} of adapter {} added", self.id, id);
            }
        }
    }
}
