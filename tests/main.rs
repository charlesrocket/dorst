use std::{
    error::Error,
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::Write,
    process::Command,
};

use assert_cmd::prelude::*;
use predicates::str::contains;
use tempfile::NamedTempFile;

use crate::helper::{test_config, test_repo};

mod files;
mod helper;

// TODO
// Simulate responses

#[test]
fn local() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;

    create_dir_all("testdir/testrepo.dorst")?;
    test_config();
    test_repo();

    let mut file = File::create("testdir/testrepo.dorst/test.txt")?;
    file.write_all(b"test")?;

    cmd.arg("--config")
        .arg("local.yaml")
        .arg("testdir")
        .assert()
        .success();

    remove_dir_all("testrepo")?;
    remove_dir_all("testdir")?;
    remove_file("local.yaml")?;

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

#[test]
fn bad_url() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;
    config.write_all(files::BAD_URL)?;

    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .success()
        .stdout(contains(
            "\u{1b}[0m \u{1b}[1;92m0\u{1b}[0m \u{1b}[37m/\u{1b}[0m \u{1b}[1;91m1\u{1b}[0m",
        ));

    Ok(())
}
