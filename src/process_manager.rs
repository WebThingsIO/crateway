/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    addon_manager::{AddonManager, AddonStopped},
    user_config,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Error};
use async_process::Command;
use futures::{
    future::{AbortHandle, Abortable},
    {io::BufReader, prelude::*},
};
use log::{debug, error, info, log, Level};
use std::{collections::HashMap, path::PathBuf, process::Stdio};
use xactor::{message, Actor, Context, Handler, Service};

#[derive(Default)]
pub struct ProcessManager {
    processes: HashMap<String, AbortHandle>,
}

impl ProcessManager {
    fn print<T>(prefix: String, level: Level, stream: Option<T>)
    where
        T: AsyncRead + Unpin + Send + 'static,
    {
        if let Some(stream) = stream {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stream).lines();

                while let Some(Ok(line)) = lines.next().await {
                    log!(level, "{}: {:?}", prefix, line);
                }
            });
        }
    }
}

impl Actor for ProcessManager {}

impl Service for ProcessManager {}

#[message(result = "Result<(), Error>")]
pub struct StartAddon(pub String, pub PathBuf, pub String);

#[async_trait]
impl Handler<StartAddon> for ProcessManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        StartAddon(id, path, exec): StartAddon,
    ) -> Result<(), Error> {
        if self.processes.contains_key(&id) {
            bail!("Process for {} already running", id)
        }

        info!("Starting {}", id);

        let path_str = path
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

        tokio::spawn(async move {
            match Abortable::new(child.status(), abort_registration).await {
                Ok(Ok(status)) => {
                    info!("Process of {} exited with code {}", id, status);
                    AddonManager::from_registry()
                        .await
                        .expect("Get addon manager")
                        .send(AddonStopped(id.clone()))
                        .expect("Stop addon");
                }
                Ok(Err(err)) => {
                    error!("Failed to wait for process to terminate: {}", err);
                }
                Err(_) => {
                    info!("Killing process {}", child.id());
                    if let Err(err) = child.kill() {
                        error!("Could not kill process {} of {}: {}", child.id(), id, err)
                    }
                    AddonManager::from_registry()
                        .await
                        .expect("Get addon manager")
                        .send(AddonStopped(id.clone()))
                        .expect("Stop addon");
                }
            };
        });

        Ok(())
    }
}

#[message(result = "Result<(), Error>")]
pub struct StopAddon(pub String);

#[async_trait]
impl Handler<StopAddon> for ProcessManager {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        StopAddon(id): StopAddon,
    ) -> Result<(), Error> {
        let abort_handle = self
            .processes
            .remove(&id)
            .ok_or_else(|| anyhow!("Process for {} not running!", id))?;
        info!("Stopping {}", &id);
        abort_handle.abort();
        Ok(())
    }
}
