/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use webthings_gateway_ipc_types::{Device, DeviceWithoutId};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Thing {
    #[serde(flatten)]
    pub device: Device,
    pub connected: bool,
}

impl Deref for Thing {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl Into<ThingWithoutId> for Thing {
    fn into(self) -> ThingWithoutId {
        ThingWithoutId {
            device: self.device.into_device_without_id(),
            connected: self.connected,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ThingWithoutId {
    #[serde(flatten)]
    pub device: DeviceWithoutId,
    pub connected: bool,
}

pub trait IntoThing {
    fn into_thing(self, id: String) -> Thing;
}

impl IntoThing for ThingWithoutId {
    fn into_thing(self, id: String) -> Thing {
        Thing {
            device: self.device.into_device(id),
            connected: self.connected,
        }
    }
}

pub trait IntoDeviceWithoutId {
    fn into_device_without_id(self) -> DeviceWithoutId;
}

impl IntoDeviceWithoutId for Device {
    fn into_device_without_id(self) -> DeviceWithoutId {
        DeviceWithoutId {
            at_context: self.at_context,
            at_type: self.at_type,
            actions: self.actions,
            base_href: self.base_href,
            credentials_required: self.credentials_required,
            description: self.description,
            events: self.events,
            links: self.links,
            pin: self.pin,
            properties: self.properties,
            title: self.title,
        }
    }
}

pub trait IntoDevice {
    fn into_device(self, id: String) -> Device;
}

impl IntoDevice for DeviceWithoutId {
    fn into_device(self, id: String) -> Device {
        Device {
            at_context: self.at_context,
            at_type: self.at_type,
            actions: self.actions,
            base_href: self.base_href,
            credentials_required: self.credentials_required,
            description: self.description,
            events: self.events,
            links: self.links,
            pin: self.pin,
            properties: self.properties,
            title: self.title,
            id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password: String,
    pub name: String,
}

impl User {
    pub fn new(id: i64, email: String, password: String, name: String) -> Result<Self> {
        let mut user = User {
            id,
            email,
            password: "".to_owned(),
            name,
        };
        user.set_password(password)?;
        Ok(user)
    }

    pub fn set_password(&mut self, password: String) -> Result<()> {
        self.password = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        Ok(())
    }

    pub fn verify_password(&self, password: &str) -> Result<bool> {
        bcrypt::verify(password, &self.password).context(anyhow!("Failed to verify password"))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Jwt {
    pub jwt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_password() {
        let user = User::new(
            1,
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned(),
        )
        .unwrap();
        assert!(user.verify_password("password").unwrap());
        assert!(!user.verify_password("different").unwrap());
    }
}
