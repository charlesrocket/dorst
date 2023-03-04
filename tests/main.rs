use std::{
    error::Error,
    fs::{create_dir_all, File},
    io::Write,
    process::Command,
};

use assert_cmd::prelude::*;
use predicates::str::contains;
use tempfile::NamedTempFile;

mod files;

// TODO
// Simulate responses

#[test]
fn default() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;

    create_dir_all("punk.dorst")?;

    let mut file = File::create("punk.dorst/test.txt")?;
    file.write_all(b"test")?;

    cmd.arg("--config")
        .arg("tests/default.yml")
        .assert()
        .success();

    Ok(())
}

#[test]
fn config_empty() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;
    config.write_all(files::EMPTY)?;

    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains("Error: Failed to read the configuration file"));

    Ok(())
}
