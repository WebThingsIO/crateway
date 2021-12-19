/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.*
 */

mod adapter;
mod control_socket;
mod device;

use crate::{adapter::MockAdapter, control_socket::ControlSocket};
use gateway_addon_rust::{api_error::ApiError, plugin::connect};
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

    if let Err(err) = run().await {
        log::error!("Failed to start addon: {}", err);
    }

    log::info!("Exiting addon");
}

async fn run() -> Result<(), ApiError> {
    let mut plugin = connect("mock-addon").await?;
    log::debug!("Plugin registered");

    let adapter = plugin
        .create_adapter(&MockAdapter::id(), &MockAdapter::name(), |adapter_handle| {
            MockAdapter::new(adapter_handle)
        })
        .await?;

    let control_socket = ControlSocket::new(adapter.clone());

    tokio::join!(control_socket.event_loop(), plugin.event_loop());

    Ok(())
}
