use std::{error::Error, process::Command};

use assert_cmd::prelude::*;

#[test]
fn main() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.arg("example.toml")
        .assert()
        .success();

    Ok(())
}
