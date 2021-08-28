use figment::{
    providers::{Format, Json},
    Figment,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AddonManager {
    #[serde(rename = "listUrls")]
    pub list_urls: Vec<String>,
}

#[derive(Deserialize)]
pub struct Ports {
    pub http: u16,
    pub ipc: u16,
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(rename = "addonManager")]
    pub addon_manager: AddonManager,
    pub ports: Ports,
}

lazy_static! {
    pub static ref CONFIG: Config = {
        Figment::new()
            .merge(Json::file("Config.json"))
            .extract()
            .expect("Read config")
    };
}
