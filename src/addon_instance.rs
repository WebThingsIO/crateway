/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    adapter::Adapter,
    addon_manager::{AddonManager, AddonStarted},
};
use actix::prelude::*;
use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
use anyhow::{anyhow, Error};
use log::{debug, error, trace};
use std::collections::HashMap;
use webthings_gateway_ipc_types::{
    Message, MessageBase, PluginRegisterResponseMessageData, Preferences, Units, UserProfile,
};

pub struct AddonInstance {
    adapters: HashMap<String, Adapter>,
}

impl Actor for AddonInstance {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for AddonInstance {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                trace!("Received text message: {:?}", text);

                let msg = text.parse::<Message>().unwrap();
                let id = msg.plugin_id().to_owned();

                if let Err(err) = self.on_msg(msg, ctx) {
                    error!("Addon instance {:?} failed to handle message: {}", id, err);
                }
            }
            Ok(ws::Message::Binary(_)) => {
                debug!("Received unexpected binary message")
            }
            _ => (),
        }
    }
}

impl AddonInstance {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    pub fn on_msg(
        &mut self,
        msg: Message,
        ctx: &mut ws::WebsocketContext<Self>,
    ) -> Result<(), Error> {
        debug!("Received {:?}", msg);

        match msg {
            Message::PluginRegisterRequest(msg) => {
                let id = msg.plugin_id();

                AddonManager::from_registry().do_send(AddonStarted {
                    id: id.to_owned(),
                    addr: ctx.address(),
                });

                let response: Message = PluginRegisterResponseMessageData {
                    gateway_version: env!("CARGO_PKG_VERSION").to_owned(),
                    plugin_id: id.to_owned(),
                    preferences: Preferences {
                        language: "en-US".to_owned(),
                        units: Units {
                            temperature: "degree celsius".to_owned(),
                        },
                    },
                    user_profile: UserProfile {
                        addons_dir: "".to_owned(),
                        base_dir: "".to_owned(),
                        config_dir: "".to_owned(),
                        data_dir: "".to_owned(),
                        gateway_dir: "".to_owned(),
                        log_dir: "".to_owned(),
                        media_dir: "".to_owned(),
                    },
                }
                .into();

                debug!("Sending {:?}", &response);
                ctx.text(serde_json::to_string(&response)?);
            }
            Message::AdapterAddedNotification(msg) => {
                let adapter = Adapter::new(msg.data.adapter_id.clone());
                self.adapters.insert(msg.data.adapter_id, adapter);
            }
            Message::DeviceAddedNotification(msg) => {
                let adapter = self.get_adapter_mut(&msg.data.adapter_id)?;
                adapter.add_device(msg.data.device);
            }
            Message::DevicePropertyChangedNotification(msg) => {
                let adapter = self.get_adapter_mut(&msg.data.adapter_id)?;
                adapter.update_property(msg.data.device_id, msg.data.property)?;
            }
            Message::DeviceConnectedStateNotification(msg) => {
                let adapter = self.get_adapter_mut(&msg.data.adapter_id)?;
                adapter.set_connect_state(msg.data.device_id, msg.data.connected)?;
            }
            _ => {}
        };

        Ok(())
    }

    fn get_adapter_mut(&mut self, id: &str) -> Result<&mut Adapter, Error> {
        self.adapters
            .get_mut(id)
            .ok_or_else(|| anyhow!("No adapter with id {} found", id))
    }
}
