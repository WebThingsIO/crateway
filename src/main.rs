/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

mod addon_instance;
mod addon_socket;
mod process_manager;

use crate::process_manager::{ProcessManager, StartAddon};
use actix::prelude::*;
use dirs::home_dir;
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

    let mut home_dir = home_dir().expect("Get home dir");

    home_dir.push(".webthings2");

    let mut addon_dir = home_dir.clone();
    addon_dir.push("addons");
    addon_dir.push("test-adapter");

    ProcessManager::from_registry().do_send(StartAddon {
        home: home_dir,
        id: String::from("test-adapter"),
        path: addon_dir,
        exec: String::from("{path}/target/debug/{name}"),
    });

    addon_socket::start().await.expect("Starting addon socket");
}
