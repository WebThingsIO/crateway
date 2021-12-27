/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use crate::{
    model::{IntoThing, Thing, ThingWithoutId, User},
    user_config,
};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::{collections::HashMap, fmt::Debug, marker::PhantomData, ops::Deref, str::FromStr};
use webthings_gateway_ipc_types::Device;
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
            let description: ThingWithoutId =
                serde_json::from_str(&description).context("Parse JSON description")?;
            things.push(description.into_thing(id));
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
                let description: ThingWithoutId =
                    serde_json::from_str(&entry.1).context("Parse JSON description")?;
                Ok(Some(description.into_thing(id)))
            }
        }
    }
}

#[message(result = "Result<Thing>")]
pub struct CreateThing(pub Device);

#[async_trait]
impl Handler<CreateThing> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        CreateThing(description): CreateThing,
    ) -> Result<Thing> {
        let thing = Thing {
            device: description,
            connected: true,
        };
        let description = serde_json::to_string(&thing).context("Stringify thing")?;
        self.execute(
            "INSERT INTO things (id, description) VALUES (?, ?)",
            params![thing.id, description],
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

#[message(result = "Result<HashMap<String, String>>")]
pub struct GetJwtsByUser(pub i64);

#[async_trait]
impl Handler<GetJwtsByUser> for Db {
    async fn handle(
        &mut self,
        _ctx: &mut Context<Self>,
        GetJwtsByUser(user_id): GetJwtsByUser,
    ) -> Result<HashMap<String, String>> {
        let mut stmt = self
            .prepare("SELECT keyId, publicKey FROM jsonwebtokens WHERE user = ?")
            .context("Prepare statement")?;
        let mut rows = stmt.query([user_id]).context("Execute query")?;
        let mut jwts = HashMap::new();
        while let Some(row) = rows.next().context("Next row")? {
            let key_id: String = row.get(0).context("Get parameter")?;
            let public_key: String = row.get(1).context("Get parameter")?;
            jwts.insert(key_id, public_key);
        }
        Ok(jwts)
    }
}

impl Deref for Db {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn create_tables(conn: &Connection) {
    conn.execute("PRAGMA foreign_keys = ON", params![])
        .expect("Enable foreign key support");

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
                  publicKey TEXT,
                  FOREIGN KEY (user) REFERENCES users(id) 
                      ON DELETE CASCADE
                  )",
        [],
    )
    .expect("Create table jsonwebtokens");
}

#[cfg(test)]
mod tests {
    extern crate two_rusty_forks;
    use super::*;
    use crate::{macros::call, model::IntoDevice, tests_common::setup};
    use serde_json::json;
    use two_rusty_forks::test_fork;
    use webthings_gateway_ipc_types::DeviceWithoutId;

    #[async_test]
    #[test_fork]
    async fn test_create_things() {
        let _ = setup();
        let description = DeviceWithoutId {
            at_context: None,
            at_type: None,
            actions: None,
            base_href: None,
            credentials_required: None,
            description: None,
            events: None,
            links: None,
            pin: None,
            properties: None,
            title: None,
        };
        call!(Db.CreateThing(description.clone().into_device("test1".to_owned()))).unwrap();
        call!(Db.CreateThing(description.clone().into_device("test2".to_owned()))).unwrap();
        let things = call!(Db.GetThings).unwrap();
        assert_eq!(things.len(), 2);
        assert_eq!(
            things[0],
            ThingWithoutId {
                device: description.clone(),
                connected: true
            }
            .into_thing("test1".to_owned())
        );
        assert_eq!(
            things[1],
            ThingWithoutId {
                device: description,
                connected: true
            }
            .into_thing("test2".to_owned())
        );
    }

    #[async_test]
    #[test_fork]
    async fn test_get_thing() {
        let _ = setup();
        let description = DeviceWithoutId {
            at_context: None,
            at_type: None,
            actions: None,
            base_href: None,
            credentials_required: None,
            description: None,
            events: None,
            links: None,
            pin: None,
            properties: None,
            title: None,
        };
        call!(Db.CreateThing(description.clone().into_device("test".to_owned()))).unwrap();
        let thing = call!(Db.GetThing("test".to_owned())).unwrap();
        assert_eq!(
            thing,
            Some(
                ThingWithoutId {
                    device: description,
                    connected: true,
                }
                .into_thing("test".to_owned())
            )
        );
    }

    #[async_test]
    #[test_fork]
    async fn test_get_user_by_id() {
        let _ = setup();
        let created_user = call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        let user = call!(Db.GetUser::ById(created_user.id)).unwrap().unwrap();
        assert_eq!(user.id, created_user.id);
        assert_eq!(user.email, "test@test");
        assert!(user.verify_password("password").unwrap());
        assert_eq!(user.name, "Tester");
    }

    #[async_test]
    #[test_fork]
    async fn test_get_user_by_email() {
        let _ = setup();
        let created_user = call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        let user = call!(Db.GetUser::ByEmail(created_user.email))
            .unwrap()
            .unwrap();
        assert_eq!(user.id, created_user.id);
        assert_eq!(user.email, "test@test");
        assert!(user.verify_password("password").unwrap());
        assert_eq!(user.name, "Tester");
    }

    #[async_test]
    #[test_fork]
    async fn test_edit_user() {
        let _ = setup();
        let created_user = call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        call!(Db.EditUser(
            User::new(
                created_user.id,
                "foo@bar".to_owned(),
                "test1234".to_owned(),
                "Peter".to_owned()
            )
            .unwrap()
        ))
        .unwrap();
        let edited_user = call!(Db.GetUser::ById(created_user.id)).unwrap().unwrap();
        assert_eq!(edited_user.id, created_user.id);
        assert_eq!(edited_user.email, "foo@bar");
        assert!(edited_user.verify_password("test1234").unwrap());
        assert_eq!(edited_user.name, "Peter");
    }

    #[async_test]
    #[test_fork]
    async fn test_delete_user() {
        let _ = setup();
        let created_user = call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        assert!(call!(Db.DeleteUser(created_user.id)).is_ok());
    }

    #[async_test]
    #[test_fork]
    async fn test_get_users() {
        let _ = setup();
        call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        call!(Db.CreateUser(
            "foo@bar".to_owned(),
            "test1234".to_owned(),
            "Peter".to_owned()
        ))
        .unwrap();
        let users = call!(Db.GetUsers).unwrap();
        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|user| {
            user.email == "test@test"
                && user.verify_password("password").unwrap()
                && user.name == "Tester"
        }));
        assert!(users.iter().any(|user| {
            user.email == "foo@bar"
                && user.verify_password("test1234").unwrap()
                && user.name == "Peter"
        }));
    }

    #[async_test]
    #[test_fork]
    async fn test_get_user_count() {
        let _ = setup();
        call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        call!(Db.CreateUser(
            "foo@bar".to_owned(),
            "test1234".to_owned(),
            "Peter".to_owned()
        ))
        .unwrap();
        assert_eq!(call!(Db.GetUserCount).unwrap(), 2);
    }

    #[async_test]
    #[test_fork]
    async fn test_get_jwt_public_key() {
        let _ = setup();
        let user = call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        call!(Db.CreateJwt("1234".to_owned(), user.id, "key".to_owned())).unwrap();
        assert_eq!(
            call!(Db.GetJwtPublicKeyByKeyId("1234".to_owned())).unwrap(),
            "key"
        );
    }

    #[async_test]
    #[test_fork]
    async fn test_get_jwts_by_user() {
        let _ = setup();
        let user1 = call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        let user2 = call!(Db.CreateUser(
            "foo@bar".to_owned(),
            "test1234".to_owned(),
            "Peter".to_owned()
        ))
        .unwrap();
        call!(Db.CreateJwt("1234".to_owned(), user1.id, "key1".to_owned())).unwrap();
        call!(Db.CreateJwt("2345".to_owned(), user1.id, "key2".to_owned())).unwrap();
        call!(Db.CreateJwt("3456".to_owned(), user2.id, "key3".to_owned())).unwrap();
        assert_eq!(call!(Db.GetJwtsByUser(user1.id)).unwrap().len(), 2);
        assert_eq!(call!(Db.GetJwtsByUser(user2.id)).unwrap().len(), 1);
    }

    #[async_test]
    #[test_fork]
    async fn test_delete_user_deletes_jwts() {
        let _ = setup();
        let user1 = call!(Db.CreateUser(
            "test@test".to_owned(),
            "password".to_owned(),
            "Tester".to_owned()
        ))
        .unwrap();
        let user2 = call!(Db.CreateUser(
            "foo@bar".to_owned(),
            "test1234".to_owned(),
            "Peter".to_owned()
        ))
        .unwrap();
        call!(Db.CreateJwt("1234".to_owned(), user1.id, "key1".to_owned())).unwrap();
        call!(Db.CreateJwt("2345".to_owned(), user1.id, "key2".to_owned())).unwrap();
        call!(Db.CreateJwt("3456".to_owned(), user2.id, "key3".to_owned())).unwrap();
        call!(Db.DeleteUser(user1.id)).unwrap();
        assert_eq!(call!(Db.GetJwtsByUser(user1.id)).unwrap().len(), 0);
        assert_eq!(call!(Db.GetJwtsByUser(user2.id)).unwrap().len(), 1);
    }

    #[async_test]
    #[test_fork]
    async fn test_set_setting() {
        let _ = setup();
        call!(Db.SetSetting("foo".to_owned(), "bar")).unwrap();
        call!(Db.SetSetting("more".to_owned(), 42)).unwrap();
        call!(Db.SetSetting("another".to_owned(), true)).unwrap();
        call!(Db.SetSetting("last".to_owned(), json!({"foo": "bar"}))).unwrap();
        assert_eq!(
            call!(Db.GetSetting("foo".to_owned(), PhantomData::<String>)).unwrap(),
            "bar"
        );
        assert_eq!(
            call!(Db.GetSetting("more".to_owned(), PhantomData::<i64>)).unwrap(),
            42
        );
        assert!(call!(Db.GetSetting("another".to_owned(), PhantomData::<bool>)).unwrap());
        assert_eq!(
            call!(Db.GetSetting("last".to_owned(), PhantomData::<serde_json::Value>)).unwrap(),
            json!({"foo": "bar"})
        );
    }

    #[async_test]
    #[test_fork]
    async fn test_set_setting_overwrite() {
        let _ = setup();
        call!(Db.SetSetting("foo".to_owned(), "bar")).unwrap();
        call!(Db.SetSetting("foo".to_owned(), "stuff")).unwrap();
        assert_eq!(
            call!(Db.GetSetting("foo".to_owned(), PhantomData::<String>)).unwrap(),
            "stuff"
        );
    }

    #[async_test]
    #[test_fork]
    async fn test_get_setting_wrong_datatype() {
        let _ = setup();
        call!(Db.SetSetting("foo".to_owned(), "bar")).unwrap();
        assert!(call!(Db.GetSetting("foo".to_owned(), PhantomData::<i64>)).is_err());
    }

    #[async_test]
    #[test_fork]
    async fn test_set_setting_if_not_exists() {
        let _ = setup();
        call!(Db.SetSettingIfNotExists("foo".to_owned(), "bar")).unwrap();
        assert_eq!(
            call!(Db.GetSetting("foo".to_owned(), PhantomData::<String>)).unwrap(),
            "bar"
        );
        call!(Db.SetSettingIfNotExists("foo".to_owned(), "buzz")).unwrap();
        assert_eq!(
            call!(Db.GetSetting("foo".to_owned(), PhantomData::<String>)).unwrap(),
            "bar"
        );
    }
}
