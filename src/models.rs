use crate::schema::things;
use anyhow::{anyhow, Result};
use core::ops::Deref;
use diesel::{prelude::*, query_builder::UndecoratedInsertRecord, sqlite::Sqlite};
use serde::{Deserialize, Serialize};
use webthings_gateway_ipc_types::{Device, DeviceWithoutId};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Thing {
    #[serde(flatten)]
    pub device: Device,
    pub connected: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ThingWithoutId {
    #[serde(flatten)]
    pub device: DeviceWithoutId,
    pub connected: bool,
}

impl Thing {
    pub async fn find(thing_id: &str) -> Result<Option<Self>> {
        use crate::schema::things::dsl::*;
        let mut result = things.find(thing_id).load(&*crate::db2::connection())?;
        match result.len() {
            0 => Ok(None),
            1 => Ok(Some(result.swap_remove(0))),
            _ => panic!("Should never happen"),
        }
    }

    pub async fn all() -> Result<Vec<Self>> {
        use crate::schema::things::dsl::*;
        things
            .load(&*crate::db2::connection())
            .map_err(|err| anyhow!(err))
    }
    pub async fn create(device: Device) -> Result<usize> {
        diesel::insert_into(things::table)
            .values(Self {
                device,
                connected: true,
            })
            .execute(&*crate::db2::connection())
            .map_err(|err| anyhow!(err))
    }
}

impl Queryable<things::SqlType, Sqlite> for Thing {
    type Row = (String, String);

    fn build(row: Self::Row) -> Self {
        let thing_description: ThingWithoutId = serde_json::from_str(&row.1).unwrap_or_default();
        thing_description.into_thing(row.0)
    }
}

type ThingDescriptionValue = Option<diesel::dsl::Eq<things::description, String>>;
type ThingIDValue = Option<diesel::dsl::Eq<things::id, String>>;
impl Insertable<things::table> for Thing {
    type Values = <(ThingIDValue, ThingDescriptionValue) as Insertable<things::table>>::Values;
    fn values(self) -> Self::Values {
        let description = serde_json::to_string(&self).unwrap_or_else(|_| "".to_string());
        (
            Some(things::id.eq(self.device.id)),
            Some(things::description.eq(description)),
        )
            .values()
    }
}

impl UndecoratedInsertRecord<things::table> for Thing {}

impl Default for ThingWithoutId {
    fn default() -> Self {
        Self {
            device: DeviceWithoutId {
                at_context: None,
                at_type: None,
                actions: None,
                base_href: None,
                credentials_required: None,
                description: None,
                events: None,
                links: None,
                pin: None,
                properties: None,
                title: None,
            },
            connected: false,
        }
    }
}

impl Deref for Thing {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests_common::setup;
    use two_rusty_forks::test_fork;
    use webthings_gateway_ipc_types::DeviceWithoutId;

    #[async_test]
    #[test_fork]
    async fn test_create_things() {
        let _ = setup();
        let description = DeviceWithoutId {
            at_context: None,
            at_type: None,
            actions: None,
            base_href: None,
            credentials_required: None,
            description: None,
            events: None,
            links: None,
            pin: None,
            properties: None,
            title: None,
        };
        Thing::create(description.clone().into_device("test1".to_owned()))
            .await
            .unwrap();
        Thing::create(description.clone().into_device("test2".to_owned()))
            .await
            .unwrap();
        let things = Thing::all().await.unwrap();
        assert_eq!(things.len(), 2);
        assert_eq!(
            things[0],
            ThingWithoutId {
                device: description.clone(),
                connected: true
            }
            .into_thing("test1".to_owned())
        );
        assert_eq!(
            things[1],
            ThingWithoutId {
                device: description,
                connected: true
            }
            .into_thing("test2".to_owned())
        );
    }

    #[async_test]
    #[test_fork]
    async fn test_get_thing() {
        let _ = setup();
        let description = DeviceWithoutId {
            at_context: None,
            at_type: None,
            actions: None,
            base_href: None,
            credentials_required: None,
            description: None,
            events: None,
            links: None,
            pin: None,
            properties: None,
            title: None,
        };
        Thing::create(description.clone().into_device("test".to_owned()))
            .await
            .unwrap();
        let thing = Thing::find("test").await.unwrap();
        assert_eq!(
            thing,
            Some(
                ThingWithoutId {
                    device: description,
                    connected: true,
                }
                .into_thing("test".to_owned())
            )
        );
    }
}
