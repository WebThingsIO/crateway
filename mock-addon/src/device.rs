/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use gateway_addon_rust::{Device, DeviceBuilder, DeviceDescription, DeviceHandle};

pub struct MockDeviceBuilder;

impl MockDeviceBuilder {
    pub fn new() -> Self {
        Self
    }
}

impl DeviceBuilder for MockDeviceBuilder {
    type Device = MockDevice;

    fn id(&self) -> String {
        "mock-device".to_owned()
    }

    fn description(&self) -> DeviceDescription {
        DeviceDescription::default()
    }

    fn build(self, device_handle: DeviceHandle) -> Self::Device {
        MockDevice::new(device_handle)
    }
}

pub struct MockDevice {
    device_handle: DeviceHandle,
}

impl MockDevice {
    pub fn new(device_handle: DeviceHandle) -> Self {
        Self { device_handle }
    }
}

impl Device for MockDevice {
    fn device_handle_mut(&mut self) -> &mut DeviceHandle {
        &mut self.device_handle
    }
}
