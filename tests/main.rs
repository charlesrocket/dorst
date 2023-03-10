use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

use std::{
    env,
    error::Error,
    fs::{remove_dir_all, remove_file},
    io::Write,
    path::Path,
};

use crate::{
    files::{EMPTY, TEST_REPO, TEST_REPO_INVALID},
    helper::test_setup,
};

mod files;
mod helper;

// TODO
// Simulate responses

#[test]
fn init() -> Result<(), Box<dyn Error>> {
    env::set_var("XDG_CONFIG_HOME", "init_test");

    if Path::new("init_test").exists() {
        remove_dir_all("init_test")?;
    }

    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.assert().success();

    if Path::new("init_test").exists() {
        remove_dir_all("init_test")?;
    }

    Ok(())
}

#[test]
fn local() -> Result<(), Box<dyn Error>> {
    test_setup(TEST_REPO, "test-local/testrepo", "test-local");

    let mut clone = Command::cargo_bin("dorst")?;
    let mut fetch = Command::cargo_bin("dorst")?;

    clone
        .arg("--config")
        .arg("test-local/config.yaml")
        .arg("test-local/local")
        .assert()
        .success()
        .stdout(contains(
            "\u{1b}[1;92m1\u{1b}[0m \u{1b}[37m/\u{1b}[0m \u{1b}[1;91m0\u{1b}[0m",
        ));

    fetch
        .arg("--config")
        .arg("test-local/config.yaml")
        .arg("test-local/local")
        .assert()
        .success()
        .stdout(contains(
            "\u{1b}[1;92m1\u{1b}[0m \u{1b}[37m/\u{1b}[0m \u{1b}[1;91m0\u{1b}[0m",
        ));

    remove_dir_all("test-local")?;

    Ok(())
}

#[test]
fn bad_refs() -> Result<(), Box<dyn Error>> {
    test_setup(TEST_REPO_INVALID, "test-bad_refs/badrefs", "test-bad_refs");

    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.arg("--config")
        .arg("test-bad_refs/config.yaml")
        .arg("test-bad_refs/bad_refs")
        .assert()
        .success()
        .stdout(contains("badrefs: corrupted loose reference file"));

    remove_dir_all("test-bad_refs")?;

    Ok(())
}

#[test]
fn config_new() -> Result<(), Box<dyn Error>> {
    if Path::new("prompt_test").is_file() {
        remove_file("prompt_test")?;
    }

    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.arg("-c");
    cmd.arg("prompt_test");
    cmd.write_stdin("foo?\n");
    cmd.assert().stdout(contains("unsupported URL protocol"));

    if Path::new("prompt_test").is_file() {
        remove_file("prompt_test")?;
    }

    Ok(())
}

#[test]
fn config_empty() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;

    config.write_all(EMPTY)?;
    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains("Error: Failed to read the configuration file"));

    Ok(())
}
