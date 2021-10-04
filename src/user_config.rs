/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::{env, fs::DirBuilder, path::PathBuf};

lazy_static! {
    static ref DIR_BUILDER: DirBuilder = {
        let mut builder = DirBuilder::new();
        builder.recursive(true);
        builder
    };
    pub static ref BASE_DIR: PathBuf = {
        let path = match env::var("WEBTHINGS_HOME") {
            Ok(p) => PathBuf::from(&p),
            Err(_) => dirs::home_dir().unwrap().join(".webthings2"),
        };
        DIR_BUILDER.create(&path).unwrap();
        path
    };
    pub static ref CONFIG_DIR: PathBuf = {
        let path = BASE_DIR.join("config");
        DIR_BUILDER.create(&path).unwrap();
        path
    };
    pub static ref ADDONS_DIR: PathBuf = {
        let path = BASE_DIR.join("addons");
        DIR_BUILDER.create(&path).unwrap();
        path
    };
    pub static ref DATA_DIR: PathBuf = {
        let path = BASE_DIR.join("data");
        DIR_BUILDER.create(&path).unwrap();
        path
    };
    pub static ref LOG_DIR: PathBuf = {
        let path = BASE_DIR.join("log");
        DIR_BUILDER.create(&path).unwrap();
        path
    };
    pub static ref MEDIA_DIR: PathBuf = {
        let path = BASE_DIR.join("media");
        DIR_BUILDER.create(&path).unwrap();
        path
    };
}
