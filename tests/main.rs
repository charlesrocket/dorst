use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

use std::{
    env::{self, var},
    error::Error,
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::Write,
    path::Path,
};

use crate::{
    files::{BAD_URL, EMPTY, TEST_REPO},
    helper::{test_config, test_repo},
};

mod files;
mod helper;

// TODO
// Simulate responses

#[test]
fn local() -> Result<(), Box<dyn Error>> {
    create_dir_all("testdir/testrepo.dorst/test")?;
    test_config();
    test_repo(TEST_REPO);

    let mut file = File::create("testdir/testrepo.dorst/test/test.txt")?;
    let cache = format!(
        "{}/dorst/testrepo-cache",
        var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned())
    );

    create_dir_all(cache)?;
    file.write_all(b"test")?;

    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.arg("--config")
        .arg("local.yaml")
        .arg("testdir")
        .assert()
        .success()
        .stdout(contains(
            "\u{1b}[1;92m1\u{1b}[0m \u{1b}[37m/\u{1b}[0m \u{1b}[1;91m0\u{1b}[0m",
        ));

    remove_dir_all("testrepo")?;
    //remove_dir_all("testdir")?;
    remove_file("local.yaml")?;

    Ok(())
}

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

#[test]
fn bad_url() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;

    config.write_all(BAD_URL)?;
    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .success()
        .stdout(contains("unsupported URL protocol"));

    Ok(())
}
