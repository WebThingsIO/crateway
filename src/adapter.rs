/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::device::Device;
use crate::macros::send;
use crate::things_socket::{ConnectedMessage, ThingsMessage, ThingsMessages, ThingsSocket};
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
        let device = self.get_device(&device_id)?;

        device.update_property(property)
    }

    pub async fn set_connect_state(&mut self, device_id: String, state: bool) -> Result<(), Error> {
        let device = self.get_device(&device_id)?;
        device.set_connect_state(state);

        send!(ThingsSocket.ThingsMessage(ThingsMessages::ConnectedMessage(
            ConnectedMessage::new(device_id, state)
        )))?;

        Ok(())
    }

    fn get_device(&mut self, device_id: &str) -> Result<&mut Device, Error> {
        let id = self.id.clone();
        self.devices
            .get_mut(device_id)
            .ok_or_else(|| anyhow!("Device {} does not exist in adapter {}", device_id, id))
    }
}
