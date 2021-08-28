/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{model::Thing, user_config};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::{fmt::Debug, marker::PhantomData, ops::Deref, str::FromStr};
use xactor::{message, Actor, Context, Handler, Message, Service};

pub struct Db(Connection);

impl Actor for Db {}

impl Service for Db {}

impl Default for Db {
    fn default() -> Self {
        let conn = Connection::open(user_config::CONFIG_DIR.join("db.sqlite3"))
            .expect("Open database file");
        create_tables(&conn);
        Self(conn)
    }
}

#[message(result = "Result<Vec<Thing>>")]
pub struct GetThings;

#[async_trait]
impl Handler<GetThings> for Db {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: GetThings) -> Result<Vec<Thing>> {
        let mut stmt = self
            .prepare("SELECT id, description FROM things")
            .context("Prepare statement")?;
        let mut rows = stmt.query([]).context("Execute query")?;
        let mut things = Vec::new();
        while let Some(row) = rows.next().context("Next row")? {
            let id: String = row.get(0).context("Get parameter")?;
            let description: String = row.get(1).context("Get parameter")?;
            let description =
                serde_json::from_str(&description).context("Parse JSON description")?;
            things.push(Thing::from_id_and_json(&id, description)?);
        }
        Ok(things)
    }
}

#[message(result = "Result<Option<Thing>>")]
pub struct GetThing(pub String);

#[async_trait]
impl Handler<GetThing> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        GetThing(id): GetThing,
    ) -> Result<Option<Thing>> {
        let mut stmt = self
            .prepare("SELECT id, description FROM things WHERE id = ?")
            .context("Prepare statement")?;
        let row = stmt
            .query_row(params![id], |row| {
                let id: String = row.get(0)?;
                let description: String = row.get(1)?;
                Ok((id, description))
            })
            .optional()
            .context("Query database")?;

        match row {
            None => Ok(None),
            Some(entry) => {
                let id = entry.0;
                let description =
                    serde_json::from_str(&entry.1).context("Parse JSON description")?;
                Ok(Some(Thing::from_id_and_json(&id, description)?))
            }
        }
    }
}

#[message(result = "Result<Thing>")]
struct CreateThing(String, serde_json::Value);

#[async_trait]
impl Handler<CreateThing> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        CreateThing(id, description): CreateThing,
    ) -> Result<Thing> {
        let thing = Thing::from_id_and_json(&id, description)
            .context("Get thing from id and description")?;
        let description = serde_json::to_string(&thing).context("Stringify thing")?;
        self.execute(
            "INSERT INTO things (id, description) VALUES (?, ?)",
            params![id, description],
        )
        .context("Insert into database")?;
        Ok(thing)
    }
}

pub struct SetSetting<T>(pub String, pub T);

impl<T: Send + 'static> Message for SetSetting<T> {
    type Result = Result<()>;
}

#[async_trait]
impl<T: ToString + Send + 'static> Handler<SetSetting<T>> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        SetSetting(key, value): SetSetting<T>,
    ) -> Result<()> {
        self.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) 
                    ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value.to_string()],
        )
        .context("Update database")?;
        Ok(())
    }
}

pub struct SetSettingIfNotExists<T>(pub String, pub T);

impl<T: Send + 'static> Message for SetSettingIfNotExists<T> {
    type Result = Result<()>;
}

#[async_trait]
impl<T: ToString + Send + 'static> Handler<SetSettingIfNotExists<T>> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        SetSettingIfNotExists(key, value): SetSettingIfNotExists<T>,
    ) -> Result<()> {
        self.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)",
            params![key, value.to_string()],
        )
        .context("Update database")?;
        Ok(())
    }
}

pub struct GetSetting<T>(pub String, pub PhantomData<T>);

impl<T: Send + 'static> Message for GetSetting<T> {
    type Result = Result<T>;
}

#[async_trait]
impl<T: FromStr + Send + 'static> Handler<GetSetting<T>> for Db
where
    <T as FromStr>::Err: Debug,
{
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        GetSetting(key, _): GetSetting<T>,
    ) -> Result<T> {
        let mut stmt = self
            .prepare("SELECT value FROM settings WHERE key = ?")
            .context("Prepare statement")?;
        let row = stmt
            .query_row(params![key], |row| -> Result<String, rusqlite::Error> {
                row.get(0)
            })
            .context("Query database")?;
        Ok(FromStr::from_str(&row)
            .map_err(|err: <T as FromStr>::Err| anyhow!(format!("{:?}", err)))?)
    }
}

impl Deref for Db {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn create_tables(conn: &Connection) {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS things(
                  id TEXT PRIMARY KEY,
                  description TEXT
                  )",
        [],
    )
    .expect("Create table things");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings(
                  key TEXT PRIMARY KEY,
                  value TEXT
                  )",
        [],
    )
    .expect("Create table settings");
}

#[cfg(test)]
mod tests {
    extern crate rusty_fork;
    extern crate serial_test;
    use crate::macros::call;

    use super::*;
    use rusty_fork::rusty_fork_test;
    use serial_test::serial;
    use std::{env, fs};
    use tokio::runtime::Runtime;

    #[allow(unused_must_use)]
    fn setup() {
        let dir = env::temp_dir().join(".webthingsio");
        fs::remove_dir_all(&dir); // We really don't want to handle this result, since we don't care if the directory never existed
        env::set_var("WEBTHINGS_HOME", dir);
    }

    rusty_fork_test! {
        #[test]
        #[serial]
        fn test_create_things() {
            Runtime::new().unwrap().block_on(async {
                setup();
                call!(Db.CreateThing("test1".to_owned(), serde_json::json!({}))).unwrap();
                call!(Db.CreateThing("test2".to_owned(), serde_json::json!({}))).unwrap();
                let things = call!(Db.GetThings).unwrap();
                assert_eq!(things.len(), 2);
                assert_eq!(
                    things[0],
                    Thing {
                        id: "test1".to_owned()
                    }
                );
                assert_eq!(
                    things[1],
                    Thing {
                        id: "test2".to_owned()
                    }
                );
            });
        }

        #[test]
        #[serial]
        fn test_get_thing() {
            Runtime::new().unwrap().block_on(async {
                setup();
                call!(Db.CreateThing("test".to_owned(), serde_json::json!({}))).unwrap();
                let thing = call!(Db.GetThing("test".to_owned())).unwrap();
                assert_eq!(
                    thing,
                    Some(Thing {
                        id: "test".to_owned()
                    })
                );
            });
        }
    }
}
