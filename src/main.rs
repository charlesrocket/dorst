use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use git2::{AutotagOption, Cred, RemoteCallbacks};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use serde_yaml::{self};

use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const BANNER: &str = "▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄\n\
                      █ ▄▀█▀▄▄▀█ ▄▄▀█ ▄▄█▄ ▄█\n\
                      █ █ █ ▀▄ █ ▀▀▄█▄▄▀██ ██\n\
                      █▄▄███▄▄██▄█▄▄█▄▄▄██▄██";

const SPINNER: [&str; 7] = ["░", "▒", "▓", "░", "▒", "▓", "░"];

#[derive(Default)]
struct Credentials {
    ssh_password: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(skip_serializing)]
    ssh_key: Option<PathBuf>,
    #[serde(skip_serializing)]
    ssh_pass_protected: Option<bool>,
    targets: Vec<String>,
}

impl Config {
    fn new() -> Self {
        let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
            .unwrap_or(format!("{}/.config/dorst", std::env::var("HOME").unwrap()));

        let file_path = format!("{xdg_config_home}/config.yaml");
        if Path::new(&file_path).exists() {
            let path: PathBuf = file_path.into();
            Self::read(&path)
        } else {
            let prompt = text_prompt("Enter backup target: ");
            let target: Vec<String> = prompt.split(',').map(|x| x.to_string()).collect();
            let config = Self {
                ssh_key: None,
                ssh_pass_protected: None,
                targets: target,
            };

            let new_config = serde_yaml::to_string(&config).unwrap();
            if !Path::new(&xdg_config_home).exists() {
                fs::create_dir(xdg_config_home).unwrap();
            }

            let mut file = fs::File::create(&file_path).unwrap();
            file.write_all(new_config.as_bytes()).unwrap();

            config
        }
    }

    fn read(path: &PathBuf) -> Self {
        let mut file = fs::File::open(path).unwrap();
        let mut config_data = String::new();

        file.read_to_string(&mut config_data).unwrap();

        let config: Self = serde_yaml::from_str(&config_data).unwrap();

        Self {
            ssh_key: config.ssh_key,
            ssh_pass_protected: config.ssh_pass_protected,
            targets: config.targets,
        }
    }
}

fn get_name(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

fn get_dir() -> String {
    let current_dir = env::current_dir().unwrap();
    current_dir.to_str().unwrap().to_string()
}

fn args() -> ArgMatches {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(BANNER)
        .version(env!("CARGO_PKG_VERSION"))
        .help_template("{about-with-newline}Codebase backup utility\n\n{usage-heading} {usage}\n\n{all-args}{after-help}")
        .arg(Arg::new("path")
            .action(ArgAction::Set)
            .value_name("PATH")
            .help("Backup destination")
            .value_parser(value_parser!(PathBuf))
            .hide_default_value(true)
            .default_value(get_dir()))
        .arg(Arg::new("config")
            .short('c')
            .long("config")
            .value_name("CONFIG")
            .help("Custom config path")
            .value_parser(value_parser!(PathBuf)));

    matches.get_matches()
}

fn text_prompt(message: &str) -> String {
    let mut line = String::new();
    print!("{message}");

    std::io::stdout().flush().unwrap();
    std::io::stdin()
        .read_line(&mut line)
        .expect("Error: Could not read a line");

    line.trim().to_string()
}

fn pass_prompt(message: &str) -> Option<String> {
    let pass = rpassword::prompt_password(message);
    match pass {
        Ok(pass) => Some(pass),
        _ => None,
    }
}

fn callbacks(
    ssh_key: Option<PathBuf>,
    needs_password: bool,
    password: Option<String>,
) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        if allowed_types.is_ssh_key() {
            if let Some(ssh_key) = &ssh_key {
                if needs_password {
                    if let Some(pwd) = &password {
                        Cred::ssh_key(username_from_url.unwrap(), None, ssh_key, Some(pwd))
                    } else {
                        Cred::default()
                    }
                } else {
                    Cred::ssh_key(username_from_url.unwrap(), None, ssh_key, None)
                }
            } else {
                Cred::default()
            }
        } else {
            Cred::default()
        }
    });

    callbacks
}

fn clone_with_key(
    key: &Path,
    destination: &str,
    target: &str,
    needs_password: bool,
    password: Option<String>,
) {
    let ssh_key = Path::new(&key);
    let callbacks = callbacks(Some(ssh_key.into()), needs_password, password);
    clone(destination, target, callbacks);
}

fn clone_with_defaults(destination: &str, target: &str) {
    let callbacks = callbacks(None, false, None);
    clone(destination, target, callbacks);
}

fn clone(destination: &str, target: &str, callbacks: RemoteCallbacks) {
    let mut options = git2::FetchOptions::new();
    let mut repo = git2::build::RepoBuilder::new();
    let builder = repo
        .bare(true)
        .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

    options
        .remote_callbacks(callbacks)
        .download_tags(AutotagOption::All);

    builder
        .fetch_options(options)
        .clone(target, Path::new(&destination))
        .unwrap();
}

fn main() {
    println!("{BANNER}");

    let matches = args();
    let config: Config;
    let mut needs_pwd = false;
    let mut creds = Credentials::default();
    let path = matches.get_one::<PathBuf>("path").unwrap();
    let spinner = ProgressBar::new_spinner();

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        config = Config::read(config_path)
    } else {
        config = Config::new()
    }

    if let Some(pwd) = config.ssh_pass_protected {
        creds.ssh_password = pass_prompt("Enter \x1b[1mSSH\x1b[0m key password:");
        needs_pwd = pwd;
    }

    spinner.enable_steady_tick(std::time::Duration::from_millis(90));
    spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));

    for target in config.targets.iter() {
        let dest = format!("{0}/{1}.dorst", &path.display(), get_name(target));
        if Path::new(&dest).exists() {
            fs::remove_dir_all(&dest).unwrap();
        }

        let message = format!("\x1b[36mpulling\x1b[0m \x1b[33m{}\x1b[0m", get_name(target));

        spinner.set_message(message);
        if let Some(ref ssh_key) = config.ssh_key {
            clone_with_key(
                ssh_key,
                &dest,
                target,
                needs_pwd,
                creds.ssh_password.clone(),
            );
        } else {
            clone_with_defaults(&dest, target);
        }
    }

    spinner.finish_with_message("\x1b[1;32mDONE\x1b[0m");
}
