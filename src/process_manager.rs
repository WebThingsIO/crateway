use actix::prelude::*;
use actix::{Actor, Context};
use log::{debug, error, info, log, trace, Level};
use std::io::{Read, Result};
use std::process::Child;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
};

#[derive(Default)]
pub struct ProcessManager;

impl ProcessManager {
    fn spawn(bin: &str, args: &[&str], home: &str) -> Result<Child> {
        Command::new(bin)
            .args(args)
            .env("WEBTHINGS_HOME", home)
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

            trace!("Poll thread for {} {} finished", prefix, name);
        });
    }

    fn wait_in_background(name: String, mut child: Child) {
        thread::spawn(move || {
            let code = child.wait().expect("Obtain exit code");
            info!("Process of {} exited with code {}", name, code);
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
#[rtype(result = "()")]
pub struct StartAddon {
    pub home: PathBuf,
    pub id: String,
    pub path: PathBuf,
    pub exec: String,
}

impl Handler<StartAddon> for ProcessManager {
    type Result = ();

    fn handle(&mut self, msg: StartAddon, _ctx: &mut Context<Self>) -> Self::Result {
        let StartAddon {
            home,
            id,
            path,
            exec,
        } = msg;

        info!("Starting {}", id);

        let path_str = &path.to_str().expect("Convert path to string");
        let exec_cmd = exec.replace("{name}", &id).replace("{path}", path_str);
        let args: Vec<_> = exec_cmd.split_ascii_whitespace().collect();
        let home = home.to_str().expect("Convert home to string");

        debug!("Spawning {}", exec_cmd);

        match ProcessManager::spawn(args[0], &args[1..], &home) {
            Ok(mut child) => {
                let stdout = child.stdout.take().expect("Capture standard error");
                ProcessManager::print(String::from("stdout"), id.clone(), Level::Info, stdout);

                let stderr = child.stderr.take().expect("Capture standard error");
                ProcessManager::print(String::from("stderr"), id.clone(), Level::Error, stderr);

                ProcessManager::wait_in_background(id, child);
            }
            Err(err) => error!(
                "Could not start addon process {} with {}: {}",
                id, exec_cmd, err
            ),
        }
    }
}
