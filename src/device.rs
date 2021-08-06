/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use log::debug;
use webthings_gateway_ipc_types::{Device as DeviceDescription, Property};

pub struct Device {
    description: DeviceDescription,
}

impl Device {
    pub(crate) fn update_property(&mut self, new_property: Property) -> Result<(), String> {
        let name = new_property
            .name
            .clone()
            .ok_or(String::from("Property has no name"))?;
        let mut properties = self
            .description
            .properties
            .as_mut()
            .ok_or(format!("Device {} has no properties", self.description.id))?;
        let property = properties.get_mut(&name).ok_or(format!(
            "Device {} has no property called {}",
            self.description.id, name
        ))?;
        if property.value != new_property.value {
            debug!(
                "Property {} of device {} changed from {:?} to {:?}",
                name, self.description.id, property.value, new_property.value
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
