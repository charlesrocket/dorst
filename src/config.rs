use serde::{Deserialize, Serialize};
use serde_yaml::{self};

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{error::Error, text_prompt};

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing)]
    pub ssh_key: Option<PathBuf>,
    #[serde(skip_serializing)]
    pub ssh_pass_protected: Option<bool>,
    pub targets: Vec<String>,
}

impl Config {
    fn read(path: &PathBuf) -> Result<Self, Error> {
        let mut file = fs::File::open(path)?;
        let mut config_data = String::new();

        file.read_to_string(&mut config_data)?;

        let config: Self = serde_yaml::from_str(&config_data)?;

        Ok(Self {
            ssh_key: config.ssh_key,
            ssh_pass_protected: config.ssh_pass_protected,
            targets: config.targets,
        })
    }

    pub fn open(&mut self) -> Result<(), Error> {
        let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or(format!("{}/.config/dorst", std::env::var("HOME")?));

        let file_path = format!("{xdg_config_home}/config.yaml");
        if Path::new(&file_path).exists() {
            let path: PathBuf = file_path.into();
            self.load_config(&path)?;
        } else {
            let prompt = text_prompt("Enter backup target: ");
            let target: Vec<String> = prompt?.split(',').map(|x| x.to_string()).collect();
            let config = Self {
                ssh_key: None,
                ssh_pass_protected: None,
                targets: target,
            };

            let new_config = serde_yaml::to_string(&config)?;
            if !Path::new(&xdg_config_home).exists() {
                fs::create_dir(xdg_config_home)?;
            }

            let mut file = fs::File::create(&file_path)?;
            file.write_all(new_config.as_bytes())?;
        }

        Ok(())
    }

    pub fn load_config(&mut self, path: &PathBuf) -> Result<(), Error> {
        let config = Self::read(path)?;

        self.ssh_key = config.ssh_key;
        self.ssh_pass_protected = config.ssh_pass_protected;
        self.targets = config.targets;

        Ok(())
    }

    pub fn check(&self) -> Result<(), Error> {
        if self.ssh_key.is_some() && self.ssh_pass_protected.is_none() {
            Err(Error::Config(
                "Invalid configuration: Password status is missing".to_string(),
            ))
        } else if self.ssh_key.is_none() && self.ssh_pass_protected.is_some() {
            Err(Error::Config(
                "Invalid configuration: SSH key is missing".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}
