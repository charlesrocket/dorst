use serde::{Deserialize, Serialize};

use std::{
    fs,
    io::Write,
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
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub count: u64,
}

impl Config {
    fn read(path: &PathBuf) -> Result<Self, Error> {
        let config_data = fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&config_data)?;
        let config_count = config.targets.len().try_into().unwrap();

        Ok(Self {
            ssh_key: config.ssh_key,
            ssh_pass_protected: config.ssh_pass_protected,
            targets: config.targets,
            count: config_count,
        })
    }

    pub fn open(&mut self) -> Result<(), Error> {
        let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or(format!("{}/.config", std::env::var("HOME")?));

        let config_path = format!("{xdg_config_home}/dorst");
        let file_path = format!("{config_path}/config.yaml");
        if !Path::new(&file_path).exists() {
            println!("\x1b[7m DORST: Initialization \x1b[0m");

            let prompt =
                text_prompt("\x1b[7m Enter backup targets  \n separated by a comma: \x1b[0m ");

            let target: Vec<String> = prompt?.split(',').map(ToString::to_string).collect();
            let config = Self {
                ssh_key: None,
                ssh_pass_protected: None,
                targets: target,
                count: 0,
            };

            let new_config = serde_yaml::to_string(&config)?;
            if !Path::new(&config_path).exists() {
                fs::create_dir_all(config_path)?;
            }

            let mut file = fs::File::create(&file_path)?;
            file.write_all(new_config.as_bytes())?;
        }

        let path: PathBuf = file_path.into();
        self.load_config(&path)?;

        Ok(())
    }

    pub fn load_config(&mut self, path: &PathBuf) -> Result<(), Error> {
        let config = Self::read(path)?;

        self.ssh_key = config.ssh_key;
        self.ssh_pass_protected = config.ssh_pass_protected;
        self.targets = config.targets;
        self.count = config.count;

        Ok(())
    }

    pub fn check(&self) -> Result<(), Error> {
        match (&self.ssh_key, &self.ssh_pass_protected) {
            (Some(_), None) => Err(Error::Config(
                "Invalid configuration: Password status is missing".to_string(),
            )),
            (None, Some(_)) => Err(Error::Config(
                "Invalid configuration: SSH key is missing".to_string(),
            )),
            _ => Ok(()),
        }
    }
}
