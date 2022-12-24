use librhodos::settings::Settings;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::{env, io};
use tempfile::{tempdir, TempDir};

pub fn make_config(base_name: &str) -> Settings {
    Settings::new(Some("./tests/config_files"), Some(base_name))
        .map_err(|e| {
            eprintln!("Failed to get settings: {}", e);
        })
        .unwrap()
}

pub fn make_config_with_dotenv_override(
    base_name: &str,
    env_str: &str,
) -> (Settings, TempDir, PathBuf) {
    let current_dir = env::current_dir().expect("unable to determine current directory");
    let conf = make_config(base_name);
    let dir = make_test_dotenv(env_str).expect("couldn't create temp dir");

    (conf, dir, current_dir)
}
// Copied from https://github.com/allan2/dotenvy/blob/master/dotenv/tests/common/mod.rs
// with slight modification to have make_test_dotenv() take an argument
//
fn tempdir_with_dotenv(dotenv_text: &str) -> io::Result<TempDir> {
    let dir = tempdir()?;
    env::set_current_dir(dir.path())?;
    let dotenv_path = dir.path().join(".env");
    let mut dotenv_file = File::create(dotenv_path)?;
    dotenv_file.write_all(dotenv_text.as_bytes())?;
    dotenv_file.sync_all()?;
    Ok(dir)
}

#[allow(dead_code)]
fn make_test_dotenv(env_vars: &str) -> io::Result<TempDir> {
    tempdir_with_dotenv(env_vars)
}
