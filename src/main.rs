use clap::Parser;
use git2::{AutotagOption, Cred, RemoteCallbacks, RemoteRedirect};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use serde_yaml::{self};

use std::{env, fs, path::Path};

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

fn main() {
    #[derive(Parser, Debug)]
    #[command(author, version, about = BANNER, long_about = None)]
    struct Args {
        #[arg(short, long, default_value_t = get_dir(), hide_default_value = true)]
        path: String,
        #[arg()]
        targets: String,
    }

    let spinner = ProgressBar::new_spinner();
    let args = Args::parse();
    let config = std::fs::File::open(args.targets).expect("Could not open config file.");
    let scrape_config: Config =
        serde_yaml::from_reader(config).expect("Could not read config values.");

    spinner.enable_steady_tick(std::time::Duration::from_millis(90));
    spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));

    for target in scrape_config.targets.iter() {
        let dest = format!("{0}/{1}", &args.path, get_name(target));
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
