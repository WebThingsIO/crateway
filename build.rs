use std::env;
use std::process::Command;

fn main() {
    println!("Building mock addon");
    let mock_addon_source_dir = env::current_dir().unwrap().join("mock-addon");
    Command::new("cargo")
        .args(&["build"])
        .current_dir(mock_addon_source_dir)
        .output()
        .unwrap();
}
