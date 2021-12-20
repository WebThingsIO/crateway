/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use gateway_addon_rust::{Adapter, AdapterHandle};

pub struct MockAdapter {
    adapter_handle: AdapterHandle,
}

impl MockAdapter {
    pub fn id() -> String {
        String::from("mock-adapter")
    }

    pub fn name() -> String {
        String::from("Mock adapter")
    }

    pub fn new(adapter_handle: AdapterHandle) -> Self {
        MockAdapter { adapter_handle }
    }
}

impl Adapter for MockAdapter {
    fn adapter_handle_mut(&mut self) -> &mut AdapterHandle {
        &mut self.adapter_handle
    }
}
