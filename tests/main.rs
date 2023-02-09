use std::{error::Error, fs::create_dir_all, io::Write, process::Command};

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

    cmd.arg("--config")
        .arg("tests/default.yml")
        .assert()
        .success();

    Ok(())
}

#[test]
fn config_ssh() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;
    config.write_all(files::SSH_INVALID)?;

    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains(
            "Invalid configuration: Password status is missing",
        ));

    Ok(())
}

#[test]
fn config_ssh_pwd() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;
    config.write_all(files::SSH_INVALID_PWD)?;

    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains("Invalid configuration: SSH key is missing"));

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
        .stderr(contains("Error: Config"));

    Ok(())
}

#[test]
fn no_conf() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.arg("--config")
        .arg("no-conf")
        .assert()
        .failure()
        .stderr(contains("No such file or directory"));

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
        .failure()
        .stderr(contains("unsupported URL protocol"));

    Ok(())
}
