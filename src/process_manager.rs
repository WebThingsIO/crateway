/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::user_config;
use actix::{prelude::*, Actor, Context};
use anyhow::{anyhow, bail, Context as AnyhowContext, Error};
use async_process::Command;
use futures::{
    future::{AbortHandle, Abortable},
    {io::BufReader, prelude::*},
};
use log::{debug, error, info, log, Level};
use std::{collections::HashMap, path::PathBuf, process::Stdio};

#[derive(Default)]
pub struct ProcessManager {
    processes: HashMap<String, AbortHandle>,
}

impl ProcessManager {
    fn print<T>(prefix: String, level: Level, stream: Option<T>)
    where
        T: AsyncRead + Unpin + 'static,
    {
        if let Some(stream) = stream {
            actix::spawn(async move {
                let mut lines = BufReader::new(stream).lines();

                while let Some(Ok(line)) = lines.next().await {
                    log!(level, "{}: {:?}", prefix, line);
                }
            });
        }
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
#[rtype(result = "Result<(), Error>")]
pub struct StartAddon {
    pub id: String,
    pub path: PathBuf,
    pub exec: String,
}

impl Handler<StartAddon> for ProcessManager {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: StartAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let StartAddon { id, path, exec } = msg;
        if self.processes.contains_key(&id) {
            bail!("Process for {} already running", id)
        }

        info!("Starting {}", id);

        let path_str = &path
            .to_str()
            .ok_or_else(|| anyhow!("Convert path to string"))?;
        let exec_cmd = exec.replace("{name}", &id).replace("{path}", path_str);
        let args: Vec<_> = exec_cmd.split_ascii_whitespace().collect();

        debug!("Spawning {}", exec_cmd);

        let mut child = Command::new(args[0])
            .args(&args[1..])
            .env("WEBTHINGS_HOME", user_config::BASE_DIR.clone())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(anyhow!(
                "Could not start addon process {} with {}",
                id,
                exec_cmd,
            ))?;

        debug!("Started process {} for {}", child.id(), id);

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        self.processes.insert(id.clone(), abort_handle);

        Self::print(id.to_owned(), Level::Info, child.stdout.take());
        Self::print(id.to_owned(), Level::Error, child.stderr.take());

        actix::spawn(async move {
            match Abortable::new(child.status(), abort_registration).await {
                Ok(Ok(status)) => {
                    info!("Process of {} exited with code {}", id, status);
                }
                Ok(Err(err)) => {
                    error!("Failed to wait for process to terminate: {}", err);
                }
                Err(_) => {
                    info!("Killing process {}", child.id());
                    if let Err(err) = child.kill() {
                        error!("Could not kill process {} of {}: {}", child.id(), id, err)
                    }
                }
            };
        });

        Ok(())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), Error>")]
pub struct StopAddon {
    pub id: String,
}

impl Handler<StopAddon> for ProcessManager {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: StopAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let StopAddon { id } = msg;
        let abort_handle = self
            .processes
            .remove(&id)
            .ok_or_else(|| anyhow!("Process for {} not running!", id))?;
        info!("Stopping {}", &id);
        abort_handle.abort();
        Ok(())
    }
}
