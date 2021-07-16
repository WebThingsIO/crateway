/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
use log::{debug, error, trace};
use std::error::Error;
use webthings_gateway_ipc_types::{
    Message, MessageBase, PluginRegisterResponseMessageData, Preferences, Units, UserProfile,
};

pub struct AddonInstance;

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

                match self.on_msg(msg, ctx) {
                    Ok(()) => {}
                    Err(err) => {
                        error!("Addon instance {:?} failed to handle message: {}", id, err)
                    }
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
        Self
    }

    pub fn on_msg(
        &self,
        msg: Message,
        ctx: &mut ws::WebsocketContext<Self>,
    ) -> Result<(), Box<dyn Error>> {
        debug!("Received {:?}", msg);

        if let Message::PluginRegisterRequest(msg) = msg {
            let id = msg.plugin_id();

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

        Ok(())
    }
}
