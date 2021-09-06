/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::{anyhow, Context, Error};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Thing {
    pub id: String,
}

impl Thing {
    pub fn from_id_and_json(id: &str, mut description: serde_json::Value) -> Result<Self, Error> {
        if let Value::Object(ref mut map) = description {
            map.insert("id".to_owned(), Value::String(id.to_owned()));
        }
        serde_json::from_value(description).context("Parse Thing")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password: String,
    pub name: String,
}

impl User {
    pub fn new(id: i64, email: String, password: String, name: String) -> Result<Self, Error> {
        let mut user = User {
            id,
            email,
            password: "".to_owned(),
            name,
        };
        user.set_password(password)?;
        Ok(user)
    }

    pub fn set_password(&mut self, password: String) -> Result<(), Error> {
        self.password = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        Ok(())
    }

    pub fn verify_password(&self, password: &str) -> Result<bool, Error> {
        bcrypt::verify(password, &self.password).context(anyhow!("Failed to verify password"))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Jwt {
    pub jwt: String,
}
