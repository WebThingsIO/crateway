/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![feature(proc_macro_hygiene, decl_macro, result_flattening)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate lazy_static;

mod adapter;
mod addon;
mod addon_instance;
mod addon_manager;
mod addon_socket;
mod db;
mod device;
mod model;
mod process_manager;
mod rest_api;
mod router;
mod user_config;

use crate::addon_manager::{AddonManager, LoadAddons};
use anyhow::anyhow;
use log::{info, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use xactor::Service;

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .set_thread_level(LevelFilter::Error)
            .set_target_level(LevelFilter::Error)
            .set_location_level(LevelFilter::Error)
            .set_time_format_str("%Y-%m-%d %H:%M:%S %z")
            .set_time_to_local(true)
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    info!("Starting gateway");

    tokio::spawn(async {
        addon_socket::start().await.expect("Starting addon socket");
    });

    tokio::spawn(async {
        if let Err(e) = AddonManager::from_registry()
            .await
            .expect("Get addon manager")
            .call(LoadAddons(user_config::ADDONS_DIR.clone()))
            .await
            .map_err(|err| anyhow!(err))
            .flatten()
        {
            error!("Failed load addons: {:?}", e);
        }
    });

    rest_api::launch().await;
}
