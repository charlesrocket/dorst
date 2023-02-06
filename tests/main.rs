use std::{error::Error, fs, path::Path, process::Command};

use assert_cmd::prelude::*;

#[test]
fn main() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
        .unwrap_or(format!("{}/.config", std::env::var("HOME").unwrap()));
    let file_path = format!("{xdg_config_home}/dorst/config.yaml");
    let conf_path = format!("{xdg_config_home}/dorst");

    if !Path::new(&file_path).exists() {
        fs::create_dir(conf_path)?;
        fs::copy("tests/test.yml", file_path)?;
    }

    fs::create_dir_all("punk.dorst")?;
    cmd.assert().success();

    Ok(())
}
