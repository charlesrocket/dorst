use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use git2::{AutotagOption, Cred, RemoteCallbacks};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use serde_yaml::{self};

use std::{
    env, fs,
    path::{Path, PathBuf},
};

const BANNER: &str = "▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄\n\
                      █ ▄▀█▀▄▄▀█ ▄▄▀█ ▄▄█▄ ▄█\n\
                      █ █ █ ▀▄ █ ▀▀▄█▄▄▀██ ██\n\
                      █▄▄███▄▄██▄█▄▄█▄▄▄██▄██";

const SPINNER: [&str; 7] = ["░", "▒", "▓", "░", "▒", "▓", "░"];

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    targets: Vec<String>,
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
            .short('p')
            .long("path")
            .action(ArgAction::Set)
            .value_name("PATH")
            .help("Backup destination")
            .value_parser(value_parser!(PathBuf))
            .hide_default_value(true)
            .default_value(get_dir()))
        .arg(Arg::new("config")
            .value_name("TARGETS")
            .help("Backup targets")
            .value_parser(value_parser!(PathBuf))
            .required(true));

    matches.get_matches()
}

fn main() {
    let matches = args();
    let config = matches.get_one::<PathBuf>("config").unwrap();
    let targets = matches.get_one::<PathBuf>("path").unwrap();
    let spinner = ProgressBar::new_spinner();
    let config = std::fs::File::open(config).expect("Could not open config file.");
    let scrape_config: Config =
        serde_yaml::from_reader(config).expect("Could not read config values.");

    spinner.enable_steady_tick(std::time::Duration::from_millis(90));
    spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));

    for target in scrape_config.targets.iter() {
        let dest = format!("{0}/{1}", &targets.display(), get_name(target));
        if Path::new(&dest).exists() {
            fs::remove_dir_all(&dest).unwrap();
        }

        let message = format!("\x1b[36mpulling\x1b[0m \x1b[33m{}\x1b[0m", get_name(target));
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            Cred::ssh_key(
                username_from_url.unwrap(),
                None,
                Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                None,
            )
        });

        let mut options = git2::FetchOptions::new();
        let mut repo = git2::build::RepoBuilder::new();
        let builder = repo
            .bare(true)
            .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

        spinner.set_message(message);
        options.remote_callbacks(callbacks);
        options.download_tags(AutotagOption::All);
        builder.fetch_options(options);
        builder.clone(target, Path::new(&dest)).unwrap();
    }

    spinner.finish_with_message("\x1b[1;32mDONE\x1b[0m");
}
