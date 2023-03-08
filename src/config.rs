use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use std::{
    env::var,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::util::text_prompt;

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub targets: Vec<String>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub count: u64,
}

impl Config {
    fn read(path: &PathBuf) -> Result<Self> {
        let config_data = fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&config_data)?;
        let config_count = config.targets.len().try_into().unwrap();

        Ok(Self {
            targets: config.targets,
            count: config_count,
        })
    }

    pub fn open(&mut self, file_path: &PathBuf) -> Result<()> {
        if !Path::new(&file_path).exists() {
            println!("\x1b[7m DORST: Initialization \x1b[0m");

            let prompt =
                text_prompt("\x1b[7m Enter backup targets  \n separated by a comma: \x1b[0m ");

            let target: Vec<String> = prompt?.split(',').map(ToString::to_string).collect();
            let config = Self {
                targets: target,
                count: 0,
            };

            let new_config = serde_yaml::to_string(&config)?;
            let mut file = fs::File::create(file_path)?;

            file.write_all(new_config.as_bytes())?;
        }

        let path: PathBuf = file_path.into();
        self.load_config(&path)?;

        Ok(())
    }

    pub fn load_config(&mut self, path: &PathBuf) -> Result<()> {
        let config = Self::read(path).context("Failed to read the configuration file")?;

        self.targets = config.targets;
        self.count = config.count;

        Ok(())
    }
}

pub fn xdg_path() -> Result<PathBuf> {
    let xdg_config_home =
        var("XDG_CONFIG_HOME").unwrap_or(format!("{}/.config", std::env::var("HOME")?));

    let config_path = format!("{xdg_config_home}/dorst");
    let file_path = format!("{config_path}/config.yaml");

    if !Path::new(&config_path).exists() {
        fs::create_dir_all(config_path)?;
    }

    Ok(PathBuf::from(file_path))
}
