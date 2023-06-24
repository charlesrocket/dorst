use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

use std::{env, error::Error, fs::remove_dir_all, io::Write, path::Path, thread};

use crate::{
    files::{CONFIG_EMPTY, CONFIG_LOCAL},
    helper::{commit, serve, test_repo},
};

mod files;
mod helper;

#[test]
fn init() -> Result<(), Box<dyn Error>> {
    env::set_var("XDG_CONFIG_HOME", "test-init");

    if Path::new("test-init").exists() {
        remove_dir_all("test-init")?;
    }

    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.write_stdin("init-test-target\n")
        .assert()
        .failure()
        .stderr(contains("init-test-target: unsupported URL protocol;"));

    if Path::new("test-init").exists() {
        remove_dir_all("test-init")?;
    }

    Ok(())
}

#[test]
fn mirror() -> Result<(), Box<dyn Error>> {
    if Path::new("test-mirror").exists() {
        remove_dir_all("test-mirror")?;
    }

    let repo = test_repo();
    let repo_dir = String::from(repo.path().to_str().unwrap());
    let mut clone = Command::cargo_bin("dorst")?;
    let mut fetch = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()?;

    config.write_all(CONFIG_LOCAL)?;
    runtime.spawn(async move {
        serve(repo);
    });

    thread::sleep(std::time::Duration::from_millis(300));

    clone
        .arg("--config")
        .arg(config.path())
        .arg("test-mirror")
        .assert()
        .success()
        .stdout(contains(
            "COMPLETED\u{1b}[0m \
             \u{1b}[37m(\u{1b}[0m\u{1b}[1;92m1\u{1b}[0m\u{1b}[37m)\u{1b}[0m",
        ));

    commit(repo_dir);
    fetch
        .arg("--config")
        .arg(config.path())
        .arg("test-mirror")
        .assert()
        .success()
        .stdout(contains(
            "COMPLETED\u{1b}[0m \
             \u{1b}[37m(\u{1b}[0m\u{1b}[1;92m1\u{1b}[0m\u{1b}[37m)\u{1b}[0m",
        ));

    if Path::new("test-mirror").exists() {
        remove_dir_all("test-mirror")?;
    }

    Ok(())
}

#[test]
fn config_empty() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;

    config.write_all(CONFIG_EMPTY)?;
    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains("missing field"));

    Ok(())
}
