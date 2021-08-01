/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate lazy_static;

mod addon;
mod addon_instance;
mod addon_manager;
mod addon_socket;
mod db;
mod model;
mod process_manager;
mod rest_api;
mod router;
mod user_config;

use crate::addon_manager::{AddonManager, LoadAddons};
use actix::prelude::*;
use log::{info, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

#[actix_web::main]
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

    actix::spawn(async {
        addon_socket::start().await.expect("Starting addon socket");
    });

    AddonManager::from_registry().do_send(LoadAddons {
        addon_dir: user_config::ADDONS_DIR.clone(),
    });

    rest_api::launch().await;
}
