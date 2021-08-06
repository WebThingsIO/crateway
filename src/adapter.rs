/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::device::Device;
use anyhow::{anyhow, Error};
use std::collections::HashMap;
use webthings_gateway_ipc_types::{Device as DeviceDescription, Property as PropertyDescription};

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
        let device = Device::new(description);
        let old_device = self.devices.insert(id.clone(), device);

        match old_device {
            Some(_) => {
                info!("Device {} of adapter {} updated", id, self.id);
            }
            None => {
                info!("Device {} of adapter {} added", id, self.id);
            }
        }
    }

    pub fn update_property(
        &mut self,
        device_id: String,
        property: PropertyDescription,
    ) -> Result<(), Error> {
        let device = self.devices.get_mut(&device_id).ok_or(anyhow!(
            "Device {} does not exist in adapter {}",
            device_id,
            self.id
        ))?;

        device.update_property(property)
    }
}
