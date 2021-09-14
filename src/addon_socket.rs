/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon_instance::{AddonInstance, Msg},
    config::CONFIG,
};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use log::{debug, info};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite;
use webthings_gateway_ipc_types::{Message, MessageBase};
use xactor::Actor;

async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    debug!("Incoming websocket connection from {:?}", addr);
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");
    let (sink, mut stream) = ws_stream.split();
    let addon_instance = AddonInstance::new(sink)
        .start()
        .await
        .expect("Start addon instance");

    while let Some(msg) = stream.next().await {
        let msg = msg.expect("Receive message");
        if let tungstenite::Message::Text(msg) = msg {
            debug!("Received a message from {}: {}", addr, msg);
            let msg = msg.parse::<Message>().unwrap();
            let id = msg.plugin_id().to_owned();

            if let Err(err) = addon_instance
                .call(Msg(msg))
                .await
                .map_err(|err| anyhow!(err))
                .flatten()
            {
                error!("Addon instance {:?} failed to handle message: {}", id, err);
            }
        } else {
            warn!("Received unexpected message")
        }
    }
}

pub async fn start() -> Result<()> {
    info!("Starting addon socket");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", CONFIG.ports.ipc)).await?;
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }

    Ok(())
}
