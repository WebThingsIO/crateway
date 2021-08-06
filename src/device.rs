/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use anyhow::{anyhow, Error};
use log::debug;
use webthings_gateway_ipc_types::{Device as DeviceDescription, Property};

pub struct Device {
    description: DeviceDescription,
}

impl Device {
    pub(crate) fn update_property(&mut self, new_property: Property) -> Result<(), Error> {
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
        }
        properties.insert(name, new_property);
        Ok(())
    }
}

impl Device {
    pub fn new(description: DeviceDescription) -> Self {
        Self { description }
    }
}
