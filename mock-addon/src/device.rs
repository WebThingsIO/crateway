/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::property::MockPropertyBuilder;
use gateway_addon_rust::{Device, DeviceBuilder, DeviceDescription, DeviceHandle, Properties};

pub struct MockDeviceBuilder(webthings_gateway_ipc_types::Device);

impl MockDeviceBuilder {
    pub fn new(description: webthings_gateway_ipc_types::Device) -> Self {
        Self(description)
    }
}

impl DeviceBuilder for MockDeviceBuilder {
    type Device = MockDevice;

    fn id(&self) -> String {
        self.0.id.clone()
    }

    fn description(&self) -> DeviceDescription {
        let mut description = DeviceDescription::default();
        description.at_context = self.0.at_context.clone();
        description.base_href = self.0.base_href.clone();
        description.credentials_required = self.0.credentials_required;
        description.description = self.0.description.clone();
        description.links = self.0.links.clone();
        description.pin = self.0.pin.clone();
        description.title = self.0.title.clone();
        description
    }

    fn properties(&self) -> Properties {
        let mut properties: Properties = Vec::new();
        if let Some(property_descriptions) = &self.0.properties {
            for (name, description) in property_descriptions {
                properties.push(Box::new(MockPropertyBuilder::new(
                    name.clone(),
                    description.clone(),
                )))
            }
        }
        properties
    }

    fn build(self, device_handle: DeviceHandle) -> Self::Device {
        MockDevice::new(device_handle)
    }
}

pub struct MockDevice(DeviceHandle);

impl MockDevice {
    pub fn new(device_handle: DeviceHandle) -> Self {
        Self(device_handle)
    }
}

impl Device for MockDevice {
    fn device_handle_mut(&mut self) -> &mut DeviceHandle {
        &mut self.0
    }
}
