use std::{error::Error, fs, process::Command};

use assert_cmd::prelude::*;
use predicates::str::contains;

// TODO
// Simulate responses

#[test]
fn default() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;

    fs::create_dir_all("punk.dorst")?;

    cmd.arg("--config")
        .arg("tests/default.yml")
        .assert()
        .success();

    Ok(())
}
