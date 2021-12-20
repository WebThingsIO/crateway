/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

use crate::device::MockDeviceBuilder;
use futures_util::stream::StreamExt;
use gateway_addon_rust::Adapter;
use log::{debug, warn};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

pub struct ControlSocket {
    adapter: Arc<Mutex<Box<dyn Adapter>>>,
}

#[derive(Deserialize, Debug)]
enum Message {
    CreateMockDevice,
}

async fn handle_connection(
    adapter: Arc<Mutex<Box<dyn Adapter>>>,
    stream: TcpStream,
    addr: SocketAddr,
) {
    debug!("Incoming websocket connection from {:?}", addr);
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");
    let (_sink, mut stream) = ws_stream.split();

    while let Some(msg) = stream.next().await {
        let msg = msg.expect("Receive message");
        if let tungstenite::Message::Text(msg) = msg {
            debug!("Received a message from {}: {}", addr, msg);
            let msg: Message = serde_json::from_str(&msg).unwrap();
            handle_message(adapter.clone(), msg).await;
        } else {
            warn!("Received unexpected message")
        }
    }
}

async fn handle_message(adapter: Arc<Mutex<Box<dyn Adapter>>>, msg: Message) {
    match msg {
        Message::CreateMockDevice => {
            adapter
                .lock()
                .await
                .adapter_handle_mut()
                .add_device(MockDeviceBuilder::new())
                .await
                .unwrap();
        }
    }
}

impl ControlSocket {
    pub fn new(adapter: Arc<Mutex<Box<dyn Adapter>>>) -> Self {
        Self { adapter }
    }

    pub async fn event_loop(self) {
        let listener = TcpListener::bind("127.0.0.1:9501")
            .await
            .expect("Create TCP Listener");
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(handle_connection(self.adapter.clone(), stream, addr));
        }
    }
}
