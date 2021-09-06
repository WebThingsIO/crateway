/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{config::CONFIG, macros::call};
use anyhow::Error;
use futures::StreamExt;
use futures::{stream::SplitSink, SinkExt};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite, WebSocketStream};
use xactor::{message, Actor, Context, Handler, Service};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedMessage {
    id: String,
    message_type: String,
    data: bool,
}

impl ConnectedMessage {
    pub fn new(id: String, data: bool) -> ConnectedMessage {
        Self {
            id,
            message_type: String::from("connected"),
            data,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum ThingsMessages {
    ConnectedMessage(ConnectedMessage),
}

#[message(result = "()")]
pub struct ThingsMessage(pub ThingsMessages);

#[async_trait]
impl Handler<ThingsMessage> for ThingsSocket {
    async fn handle(&mut self, _: &mut Context<Self>, ThingsMessage(msg): ThingsMessage) {
        for sink in &mut self.sinks {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if let Err(err) = sink.send(tungstenite::Message::Text(json)).await {
                        error!("Failed to send things message: {:?}", err);
                    }
                }
                Err(err) => {
                    error!("Failed to serialize message: {}", err)
                }
            }
        }
    }
}

#[message(result = "Result<(), Error>")]
struct RegisterSink(SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>);

#[async_trait]
impl Handler<RegisterSink> for ThingsSocket {
    async fn handle(
        &mut self,
        _: &mut Context<Self>,
        RegisterSink(sink): RegisterSink,
    ) -> Result<(), Error> {
        self.sinks.push(sink);

        Ok(())
    }
}

#[derive(Default)]
pub struct ThingsSocket {
    sinks: Vec<SplitSink<WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>>,
}

impl Actor for ThingsSocket {}

impl Service for ThingsSocket {}

async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    debug!("Incoming things websocket connection from {:?}", addr);

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the things websocket handshake occurred");

    let (sink, _) = ws_stream.split();

    if let Err(err) = call!(ThingsSocket.RegisterSink(sink)) {
        error!("Error sending sink to ThingsSocket: {}", err);
    }
}

pub async fn start() -> Result<(), Error> {
    info!("Starting things websocket");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", CONFIG.ports.websocket)).await?;

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }

    Ok(())
}
