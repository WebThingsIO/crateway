use std::{collections::HashSet, env, process::Command};

use regex::Regex;

lazy_static! {
    pub static ref ARCHITECTURE: String = format!("{}-{}", env::consts::OS, env::consts::ARCH);
    pub static ref NODE_VERSION: u32 = {
        if let Ok(out) = Command::new("node")
            .args(vec!["-p", "process.config.variables.node_module_version"])
            .output()
        {
            if out.status.success() {
                let version = String::from_utf8_lossy(&out.stdout).to_string();
                if let Ok(version) = version.trim().parse() {
                    return version;
                };
            }
        }
        0
    };
    pub static ref PYTHON_VERSIONS: Vec<String> = {
        let parse = |out: String| {
            let re = Regex::new(r"\d+\.\d+").unwrap();
            re.find(&out).map(|v| v.as_str().to_owned())
        };
        let mut versions: HashSet<String> = HashSet::new();
        for bin in vec!["python", "python2", "python3"] {
            if let Ok(out) = Command::new(bin).args(vec!["--version"]).output() {
                if out.status.success() {
                    if let Some(version) = parse(format!(
                        "{} {}",
                        String::from_utf8_lossy(&out.stdout).to_string(),
                        String::from_utf8_lossy(&out.stderr).to_string(),
                    )) {
                        versions.insert(version);
                    }
                }
            }
        }
        let mut versions = versions.into_iter().collect::<Vec<String>>();
        versions.sort();
        versions
    };
}
