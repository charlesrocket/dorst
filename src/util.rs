use anyhow::Result;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub fn get_name(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

pub fn get_dir() -> String {
    let current_dir = env::current_dir().unwrap();
    current_dir.to_str().unwrap().to_owned()
}

pub fn xdg_path() -> Result<PathBuf> {
    let xdg_config_home =
        env::var("XDG_CONFIG_HOME").unwrap_or(format!("{}/.config", std::env::var("HOME")?));

    let config_path = format!("{xdg_config_home}/dorst");
    let file_path = format!("{config_path}/config.yaml");

    if !Path::new(&config_path).exists() {
        fs::create_dir_all(config_path)?;
    }

    Ok(PathBuf::from(file_path))
}
