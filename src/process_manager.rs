/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon_manager::{AddonManager, AddonStopped},
    user_config,
};
use actix::{prelude::*, Actor, Context};
use log::{debug, error, info, log, Level};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Result as IOResult},
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
};

#[derive(Default)]
pub struct ProcessManager {
    processes: HashMap<String, Child>,
}

impl ProcessManager {
    fn spawn(bin: &str, args: &[&str]) -> IOResult<Child> {
        Command::new(bin)
            .args(args)
            .env("WEBTHINGS_HOME", user_config::BASE_DIR.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }

    fn print<T>(name: String, prefix: String, level: Level, stream: T)
    where
        T: Read + Send + 'static,
    {
        thread::spawn(move || {
            BufReader::new(stream)
                .lines()
                .filter_map(|line| line.ok())
                .for_each(|line| log!(level, "{} {}", prefix, line));

            if name == "stdout" {
                System::new().block_on(async {
                    AddonManager::from_registry().do_send(AddonStopped(prefix.to_owned()));
                    info!("Process of {} exited", prefix);
                });
            }
        });
    }
}

impl Actor for ProcessManager {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("ProcessManager started");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        info!("ProcessManager stopped");
    }
}

impl actix::Supervised for ProcessManager {}

impl SystemService for ProcessManager {
    fn service_started(&mut self, _ctx: &mut Context<Self>) {
        info!("ProcessManager service started");
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
pub struct StartAddon {
    pub id: String,
    pub path: PathBuf,
    pub exec: String,
}

impl Handler<StartAddon> for ProcessManager {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: StartAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let StartAddon { id, path, exec } = msg;
        if self.processes.contains_key(&id) {
            error!("Process for {} already running", id);
            return Err(());
        }

        info!("Starting {}", id);

        let path_str = &path.to_str().expect("Convert path to string");
        let exec_cmd = exec.replace("{name}", &id).replace("{path}", path_str);
        let args: Vec<_> = exec_cmd.split_ascii_whitespace().collect();

        debug!("Spawning {}", exec_cmd);

        match ProcessManager::spawn(args[0], &args[1..]) {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("Capture standard error");
                ProcessManager::print(String::from("stdout"), id.clone(), Level::Info, stdout);

                let stderr = child.stderr.take().expect("Capture standard error");
                ProcessManager::print(String::from("stderr"), id.clone(), Level::Error, stderr);

                self.processes.insert(id, child);
                Ok(())
            }
            Err(err) => {
                error!(
                    "Could not start addon process {} with {}: {}",
                    id, exec_cmd, err
                );
                Err(())
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), ()>")]
pub struct StopAddon {
    pub id: String,
}

impl Handler<StopAddon> for ProcessManager {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: StopAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let StopAddon { id } = msg;
        if let Some(mut child) = self.processes.remove(&id) {
            info!("Stopping {}", &id);
            if let Err(_) = child.kill() {
                error!("Failed to kill process for {}", id);
                return Err(());
            }
            Ok(())
        } else {
            error!("Process for {} not running!", id);
            Err(())
        }
    }
}
