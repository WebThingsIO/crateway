/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    model::{Thing, User},
    user_config,
};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
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

#[message(result = "Result<User>")]
pub struct CreateUser(pub String, pub String, pub String);

#[async_trait]
impl Handler<CreateUser> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        CreateUser(email, password, name): CreateUser,
    ) -> Result<User> {
        let mut user = User::new(0, email, password, name)?;
        self.execute(
            "INSERT INTO users (email, password, name) VALUES (?, ?, ?)",
            params![user.email, user.password, user.name],
        )
        .context("Create user")?;
        user.id = self.last_insert_rowid();
        Ok(user)
    }
}

#[message(result = "Result<()>")]
pub struct EditUser(pub User);

#[async_trait]
impl Handler<EditUser> for Db {
    async fn handle(&mut self, _ctx: &mut Context<Self>, EditUser(user): EditUser) -> Result<()> {
        self.execute(
            "UPDATE users SET email=?, password=?, name=? WHERE id=?",
            params![user.email, user.password, user.name, user.id],
        )
        .context("Edit user")?;
        Ok(())
    }
}

#[message(result = "Result<()>")]
pub struct DeleteUser(pub i64);

#[async_trait]
impl Handler<DeleteUser> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        DeleteUser(user_id): DeleteUser,
    ) -> Result<()> {
        self.execute("DELETE FROM users WHERE id = ?", params![user_id])
            .context("Delete user")?;
        Ok(())
    }
}

#[message(result = "Result<Option<User>>")]
pub enum GetUser {
    ByEmail(String),
    ById(i64),
}

#[async_trait]
impl Handler<GetUser> for Db {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: GetUser) -> Result<Option<User>> {
        let f = |row: &Row<'_>| {
            let id: i64 = row.get(0)?;
            let email: String = row.get(1)?;
            let password: String = row.get(2)?;
            let name: String = row.get(3)?;
            Ok(User {
                id,
                email,
                password,
                name,
            })
        };
        match msg {
            GetUser::ByEmail(email) => {
                let mut stmt = self
                    .prepare("SELECT * FROM users WHERE email = ?")
                    .context("Prepare statement")?;
                stmt.query_row(params![email], f)
            }
            GetUser::ById(id) => {
                let mut stmt = self
                    .prepare("SELECT * FROM users WHERE id = ?")
                    .context("Prepare statement")?;
                stmt.query_row(params![id], f)
            }
        }
        .optional()
        .context("Query database")
    }
}

#[message(result = "Result<Vec<User>>")]
pub struct GetUsers;

#[async_trait]
impl Handler<GetUsers> for Db {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: GetUsers) -> Result<Vec<User>> {
        let mut stmt = self
            .prepare("SELECT * FROM users")
            .context("Prepare statement")?;
        let mut rows = stmt.query([]).context("Execute query")?;
        let mut users = Vec::new();
        while let Some(row) = rows.next().context("Next row")? {
            let id: i64 = row.get(0).context("Get parameter")?;
            let email: String = row.get(1).context("Get parameter")?;
            let password: String = row.get(2).context("Get parameter")?;
            let name: String = row.get(3).context("Get parameter")?;
            users.push(User {
                id,
                email,
                password,
                name,
            });
        }
        Ok(users)
    }
}

#[message(result = "Result<i64>")]
pub struct GetUserCount;

#[async_trait]
impl Handler<GetUserCount> for Db {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: GetUserCount) -> Result<i64> {
        let mut stmt = self
            .prepare("SELECT COUNT(*) AS count FROM users")
            .context("Prepare statement")?;
        stmt.query_row(params![], |row| {
            let count: i64 = row.get(0)?;
            Ok(count)
        })
        .context("Query database")
    }
}

#[message(result = "Result<()>")]
pub struct CreateJwt(pub String, pub i64, pub String);

#[async_trait]
impl Handler<CreateJwt> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        CreateJwt(key_id, user, public_key): CreateJwt,
    ) -> Result<()> {
        self.execute(
            "INSERT INTO jsonwebtokens (keyId, user, publicKey) VALUES (?, ?, ?)",
            params![key_id, user, public_key],
        )
        .context("Insert into jsonwebtokens")?;
        Ok(())
    }
}

#[message(result = "Result<String>")]
pub struct GetJwtPublicKeyByKeyId(pub String);

#[async_trait]
impl Handler<GetJwtPublicKeyByKeyId> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        GetJwtPublicKeyByKeyId(kid): GetJwtPublicKeyByKeyId,
    ) -> Result<String> {
        let mut stmt = self
            .prepare("SELECT publicKey FROM jsonwebtokens WHERE keyId = ?")
            .context("Prepare statement")?;
        stmt.query_row(params![kid], |row| {
            let public_key: String = row.get(0)?;
            Ok(public_key)
        })
        .context("Query database")
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

    conn.execute(
        "CREATE TABLE IF NOT EXISTS users(
                  id INTEGER PRIMARY KEY ASC,
                  email TEXT UNIQUE,
                  password TEXT,
                  name TEXT
                  )",
        [],
    )
    .expect("Create table users");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS jsonwebtokens(
                  id INTEGER PRIMARY KEY ASC,
                  keyId TEXT UNIQUE,
                  user INTEGER,
                  publicKey TEXT
                  )",
        [],
    )
    .expect("Create table jsonwebtokens");
}

#[cfg(test)]
mod tests {
    extern crate rusty_fork;
    extern crate serial_test;
    use super::*;
    use crate::macros::call;
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

        #[test]
        #[serial]
        fn test_get_user_by_id() {
            Runtime::new().unwrap().block_on(async {
                setup();
                let created_user = call!(Db.CreateUser("test@test".to_owned(), "password".to_owned(), "Tester".to_owned())).unwrap();
                let user = call!(Db.GetUser::ById(created_user.id)).unwrap().unwrap();
                assert_eq!(user.id, created_user.id);
                assert_eq!(user.email, "test@test");
                assert!(user.verify_password("password").unwrap());
                assert_eq!(user.name, "Tester");
            });
        }

        #[test]
        #[serial]
        fn test_get_user_by_email() {
            Runtime::new().unwrap().block_on(async {
                setup();
                let created_user = call!(Db.CreateUser("test@test".to_owned(), "password".to_owned(), "Tester".to_owned())).unwrap();
                let user = call!(Db.GetUser::ByEmail(created_user.email)).unwrap().unwrap();
                assert_eq!(user.id, created_user.id);
                assert_eq!(user.email, "test@test");
                assert!(user.verify_password("password").unwrap());
                assert_eq!(user.name, "Tester");
            });
        }

        #[test]
        #[serial]
        fn test_edit_user() {
            Runtime::new().unwrap().block_on(async {
                setup();
                let created_user = call!(Db.CreateUser("test@test".to_owned(), "password".to_owned(), "Tester".to_owned())).unwrap();
                call!(Db.EditUser(User::new(created_user.id, "foo@bar".to_owned(), "test1234".to_owned(), "Peter".to_owned()).unwrap())).unwrap();
                let edited_user = call!(Db.GetUser::ById(created_user.id)).unwrap().unwrap();
                assert_eq!(edited_user.id, created_user.id);
                assert_eq!(edited_user.email, "foo@bar");
                assert!(edited_user.verify_password("test1234").unwrap());
                assert_eq!(edited_user.name, "Peter");
            });
        }

        #[test]
        #[serial]
        fn test_delete_user() {
            Runtime::new().unwrap().block_on(async {
                setup();
                let created_user = call!(Db.CreateUser("test@test".to_owned(), "password".to_owned(), "Tester".to_owned())).unwrap();
                assert!(call!(Db.DeleteUser(created_user.id)).is_ok());
            });
        }

        #[test]
        #[serial]
        fn test_get_users() {
            Runtime::new().unwrap().block_on(async {
                setup();
                call!(Db.CreateUser("test@test".to_owned(), "password".to_owned(), "Tester".to_owned())).unwrap();
                call!(Db.CreateUser("foo@bar".to_owned(), "test1234".to_owned(), "Peter".to_owned())).unwrap();
                let users = call!(Db.GetUsers).unwrap();
                assert_eq!(users.len(), 2);
                assert!(users.iter().any(|user| {
                    user.email == "test@test" &&
                    user.verify_password("password").unwrap() &&
                    user.name == "Tester"
                }));
                assert!(users.iter().any(|user| {
                    user.email == "foo@bar" &&
                    user.verify_password("test1234").unwrap() &&
                    user.name == "Peter"
                }));
            });
        }

        #[test]
        #[serial]
        fn test_get_user_count() {
            Runtime::new().unwrap().block_on(async {
                setup();
                call!(Db.CreateUser("test@test".to_owned(), "password".to_owned(), "Tester".to_owned())).unwrap();
                call!(Db.CreateUser("foo@bar".to_owned(), "test1234".to_owned(), "Peter".to_owned())).unwrap();
                assert_eq!(call!(Db.GetUserCount).unwrap(), 2);
            });
        }
        #[test]
        #[serial]
        fn test_get_jwt_public_key() {
            Runtime::new().unwrap().block_on(async {
                setup();
                let user = call!(Db.CreateUser("test@test".to_owned(), "password".to_owned(), "Tester".to_owned())).unwrap();
                call!(Db.CreateJwt("1234".to_owned(), user.id, "key".to_owned())).unwrap();
                assert_eq!(call!(Db.GetJwtPublicKeyByKeyId("1234".to_owned())).unwrap(), "key");
            });
        }
    }
}
