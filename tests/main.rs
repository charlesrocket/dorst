use std::{error::Error, fs, process::Command};

use assert_cmd::prelude::*;

#[test]
fn main() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;

    fs::create_dir_all("punk.dorst")?;
    cmd.arg("tests/test.yml").assert().success();

    Ok(())
}
