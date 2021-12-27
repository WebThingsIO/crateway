use std::collections::BTreeMap;

pub fn device(id: &str) -> webthings_gateway_ipc_types::Device {
    webthings_gateway_ipc_types::Device {
        at_context: None,
        at_type: None,
        actions: None,
        base_href: None,
        credentials_required: None,
        description: None,
        events: None,
        id: id.to_owned(),
        links: None,
        pin: None,
        properties: None,
        title: None,
    }
}

pub fn property(name: &str, type_: &str) -> webthings_gateway_ipc_types::Property {
    webthings_gateway_ipc_types::Property {
        at_type: None,
        description: None,
        enum_: None,
        links: None,
        maximum: None,
        minimum: None,
        multiple_of: None,
        name: Some(name.to_owned()),
        read_only: None,
        title: None,
        type_: type_.to_owned(),
        unit: None,
        value: None,
        visible: None,
    }
}

pub trait DeviceExt {
    fn add_property(&mut self, property: webthings_gateway_ipc_types::Property);
}

impl DeviceExt for webthings_gateway_ipc_types::Device {
    fn add_property(&mut self, property: webthings_gateway_ipc_types::Property) {
        if let Some(properties) = &mut self.properties {
            properties.insert(property.name.clone().unwrap(), property);
        } else {
            let mut properties = BTreeMap::new();
            properties.insert(property.name.clone().unwrap(), property);
            self.properties = Some(properties);
        }
    }
}
