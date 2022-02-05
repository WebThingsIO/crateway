use crate::user_config;
use diesel::{sqlite::SqliteConnection, Connection};
use diesel_migrations::embed_migrations;
use tokio::sync::Mutex;

embed_migrations!();

lazy_static! {
    pub static ref CONNECTION: Mutex<SqliteConnection> = {
        let database_url = user_config::CONFIG_DIR.join("db.sqlite3");
        let connection = SqliteConnection::establish(database_url.to_str().unwrap_or(""))
            .expect("Open database file");
        embedded_migrations::run_with_output(&connection, &mut std::io::stdout())
            .expect("Run migrations");
        Mutex::new(connection)
    };
}
