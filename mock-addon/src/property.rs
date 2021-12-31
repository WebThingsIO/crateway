/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use gateway_addon_rust::{
    type_::Type, Property, PropertyBuilder, PropertyDescription, PropertyHandle,
};
use serde_json::json;
use std::marker::PhantomData;

pub struct MockPropertyBuilder(webthings_gateway_ipc_types::Property);

impl MockPropertyBuilder {
    pub fn new(name: String, mut description: webthings_gateway_ipc_types::Property) -> Self {
        description.name = Some(name);
        Self(description)
    }
}

impl PropertyBuilder for MockPropertyBuilder {
    type Property = MockProperty;
    type Value = serde_json::Value;

    fn name(&self) -> String {
        self.0.name.clone().unwrap()
    }

    fn description(&self) -> PropertyDescription<Self::Value> {
        let mut description = PropertyDescription::default();
        description.description = self.0.description.clone();
        description.enum_ = self.0.enum_.clone();
        description.links = self.0.links.clone();
        description.maximum = self.0.maximum;
        description.minimum = self.0.minimum;
        description.multiple_of = self.0.multiple_of;
        description.read_only = self.0.read_only;
        description.title = self.0.title.clone();
        description.type_ = match self.0.type_.as_str() {
            "null" => Type::Null,
            "boolean" => Type::Boolean,
            "number" => Type::Number,
            "integer" => Type::Integer,
            "object" => Type::Object,
            "array" => Type::Array,
            _ => panic!("Invalid type"),
        };
        description.unit = self.0.unit.clone();
        description.value = self.0.value.clone().unwrap_or_else(|| json!(null));
        description.visible = self.0.visible;
        description
    }

    fn build(self: Box<Self>, property_handle: PropertyHandle<Self::Value>) -> Self::Property {
        MockProperty::new(property_handle)
    }
}

pub struct MockProperty(PropertyHandle<serde_json::Value>);

impl MockProperty {
    pub fn new(property_handle: PropertyHandle<serde_json::Value>) -> Self {
        Self(property_handle)
    }
}

impl Property for MockProperty {
    type Value = serde_json::Value;

    fn property_handle_mut(&mut self) -> &mut PropertyHandle<Self::Value> {
        &mut self.0
    }
}
