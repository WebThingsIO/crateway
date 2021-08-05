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
        match new_property.name.clone() {
            Some(name) => match &mut self.description.properties {
                Some(properties) => match properties.get_mut(&name) {
                    Some(property) => {
                        if property.value != new_property.value {
                            debug!(
                                "Property {} of device {} changed from {:?} to {:?}",
                                name, self.description.id, property.value, new_property.value
                            );
                        }
                        properties.insert(name, new_property);
                        Ok(())
                    }
                    None => Err(format!(
                        "Device {} has no property called {}",
                        self.description.id, name
                    )),
                },
                None => Err(format!("Device {} has no properties", self.description.id)),
            },
            None => Err(String::from("Property has no name")),
        }
    }
}

impl Device {
    pub fn new(description: DeviceDescription) -> Self {
        Self { description }
    }
}
