use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

use std::{
    env,
    error::Error,
    fs::remove_dir_all,
    io::Write,
    path::Path,
};

use crate::files::CONFIG_EMPTY;

mod files;

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
fn config_empty() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;

    config.write_all(CONFIG_EMPTY)?;
    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains("Error: Failed to read the configuration file"));

    Ok(())
}
