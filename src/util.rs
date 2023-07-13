use anyhow::Result;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn version_string() -> String {
    let dirty = built_info::GIT_DIRTY
        .and_then(|x| x.then_some("-dirty"))
        .unwrap_or("");

    let commit_hash =
        built_info::GIT_COMMIT_HASH_SHORT.map_or_else(String::new, |hash| format!("-{hash}"));

    let version = env!("CARGO_PKG_VERSION");

    format!("{version}{commit_hash}{dirty}")
}

pub fn expand_path(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        let home_dir = env::var_os("HOME").unwrap();
        let home_dir_str = home_dir.to_str().unwrap();
        let mut expanded_path = home_dir_str.to_owned();

        expanded_path.push_str(&path[1..]);
        return expanded_path;
    }

    path.to_owned()
}

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

#[test]
fn test_path() {
    let path_string = "~/";
    let path_expanded = expand_path(path_string);
    assert!(path_expanded.starts_with("/"));
}
