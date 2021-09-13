use std::env;

use tempdir::TempDir;

#[allow(unused_must_use)]
pub fn setup() -> TempDir {
    let dir = TempDir::new(".webthingsio").unwrap();
    env::set_var("WEBTHINGS_HOME", dir.path());
    dir
}
