use crate::{
    addon_manager::{AddonManager, GetDevices},
    db::{Db, GetThings},
    jwt::JSONWebToken,
    macros::{call, ToRocket},
};
use rocket::{http::Status, response::status, serde::json::Json, Route};
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;
use webthings_gateway_ipc_types::{Device as DeviceDescription, Property as PropertyDescription};

pub fn routes() -> Vec<Route> {
    routes![get_new_things]
}

fn serialize_device_description_without_properties<S>(
    device_description: &DeviceDescription,
    s: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut obj = serde_json::to_value(device_description).unwrap();
    let obj = obj.as_object_mut().unwrap();
    obj.remove("properties");
    s.collect_map(obj)
}

#[derive(Serialize)]
struct PropertyDescriptionWithHref {
    #[serde(flatten)]
    property: PropertyDescription,
    href: String,
}

#[derive(Serialize)]
struct DeviceDescriptionWithHref {
    #[serde(
        serialize_with = "serialize_device_description_without_properties",
        flatten
    )]
    device: DeviceDescription,
    href: String,
    properties: Option<BTreeMap<String, PropertyDescriptionWithHref>>,
}

impl DeviceDescriptionWithHref {
    fn from(device: DeviceDescription) -> Self {
        let device_id = device.id.clone();
        let properties = device.properties.clone().map(|properties| {
            properties
                .iter()
                .map(|(property_id, property)| {
                    (
                        property_id.to_owned(),
                        PropertyDescriptionWithHref {
                            property: property.clone(),
                            href: format!(
                                "/things/{}/properties/{}",
                                urlencoding::encode(&device_id).to_owned(),
                                urlencoding::encode(&property_id).to_owned()
                            ),
                        },
                    )
                })
                .collect()
        });
        Self {
            device,
            href: format!("/things/{}", urlencoding::encode(&device_id).to_owned()),
            properties,
        }
    }
}

#[get("/")]
async fn get_new_things(
    _jwt: JSONWebToken,
) -> Result<Json<Vec<DeviceDescriptionWithHref>>, status::Custom<String>> {
    let stored_things = call!(Db.GetThings)
        .to_rocket("Failed to get stored things", Status::InternalServerError)?;
    let connected_devices = call!(AddonManager.GetDevices).to_rocket(
        "Failed to get connected things",
        Status::InternalServerError,
    )?;
    let new_devices = connected_devices
        .into_iter()
        .filter(|(id, _)| !stored_things.iter().any(|thing| &thing.id == id))
        .map(|(_, device)| DeviceDescriptionWithHref::from(device))
        .collect();
    Ok(Json(new_devices))
}
