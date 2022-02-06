/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password: String,
    pub name: String,
}

impl User {
    pub fn new(id: i64, email: String, password: String, name: String) -> Result<Self> {
        let mut user = User {
            id,
            email,
            password: "".to_owned(),
            name,
        };
        user.set_password(password)?;
        Ok(user)
    }

    pub fn set_password(&mut self, password: String) -> Result<()> {
        self.password = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        Ok(())
    }

    pub fn verify_password(&self, password: &str) -> Result<bool> {
        bcrypt::verify(password, &self.password).context(anyhow!("Failed to verify password"))
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Jwt {
    pub jwt: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_password() {
        let user = User::new(
            1,
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned(),
        )
        .unwrap();
        assert!(user.verify_password("password").unwrap());
        assert!(!user.verify_password("different").unwrap());
    }
}
