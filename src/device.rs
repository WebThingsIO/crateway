/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::{
    macros::send,
    things_socket::{PropertyStatusMessage, ThingsMessage, ThingsMessages, ThingsSocket},
};
use anyhow::{anyhow, Result};
use log::debug;
use rocket::serde::json::Value;
use webthings_gateway_ipc_types::{Device as DeviceDescription, Property};

pub struct Device {
    pub description: DeviceDescription,
    connected: bool,
}

impl Device {
    pub(crate) async fn update_property(&mut self, new_property: Property) -> Result<()> {
        let id = self.description.id.clone();

        let name = new_property
            .name
            .clone()
            .ok_or_else(|| anyhow!("Property has no name"))?;
        let properties = self
            .description
            .properties
            .as_mut()
            .ok_or_else(|| anyhow!("Device {} has no properties", id))?;
        let property = properties
            .get_mut(&name)
            .ok_or_else(|| anyhow!("Device {} has no property called {}", id, name))?;
        if property.value != new_property.value {
            debug!(
                "Property {} of device {} changed from {:?} to {:?}",
                name, id, property.value, new_property.value
            );

            send!(
                ThingsSocket.ThingsMessage(ThingsMessages::PropertyStatusMessage(
                    PropertyStatusMessage::new(
                        id,
                        name.clone(),
                        new_property.value.clone().unwrap_or(Value::Null)
                    )
                ))
            )?;
        }
        properties.insert(name.clone(), new_property.clone());
        Ok(())
    }

    pub fn set_connect_state(&mut self, state: bool) {
        self.connected = state;
    }
}

impl Device {
    pub fn new(description: DeviceDescription) -> Self {
        Self {
            description,
            connected: true,
        }
    }
}
