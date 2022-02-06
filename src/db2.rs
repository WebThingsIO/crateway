use crate::user_config;
use diesel::{sqlite::SqliteConnection, Connection};
use diesel_migrations::embed_migrations;
use ref_thread_local::{Ref, RefThreadLocal};

embed_migrations!();

ref_thread_local! {
    static managed CONNECTION: SqliteConnection = {
        let database_url = user_config::CONFIG_DIR.join("db.sqlite3");
        let connection = SqliteConnection::establish(database_url.to_str().unwrap_or(""))
            .expect("Open database file");
        embedded_migrations::run_with_output(&connection, &mut std::io::stdout())
            .expect("Run migrations");
        connection
    };
}

pub fn connection<'a>() -> Ref<'a, SqliteConnection> {
    CONNECTION.borrow()
}
