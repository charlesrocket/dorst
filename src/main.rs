use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use git2::{AutotagOption, Cred, RemoteCallbacks};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

mod config;
mod error;

use crate::{config::Config, error::Error};

const BANNER: &str = "▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄\n\
                      █ ▄▀█▀▄▄▀█ ▄▄▀█ ▄▄█▄ ▄█\n\
                      █ █ █ ▀▄ █ ▀▀▄█▄▄▀██ ██\n\
                      █▄▄███▄▄██▄█▄▄█▄▄▄██▄██";

const SPINNER: [&str; 7] = [
    "\u{2591}", "\u{2592}", "\u{2593}", "\u{2591}", "\u{2592}", "\u{2593}", "\u{2591}",
];

#[derive(Default)]
struct Credentials {
    ssh_password: Option<String>,
}

fn get_name(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

fn get_dir() -> String {
    let current_dir = env::current_dir().unwrap();
    current_dir.to_str().unwrap().to_string()
}

fn set_threads(threads: u8) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.into())
        .build_global()
        .unwrap();
}

fn args() -> ArgMatches {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
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
            .help("Use alternative config file")
            .value_parser(value_parser!(PathBuf)))
        .arg(Arg::new("threads")
            .short('t')
            .long("threads")
            .value_name("THREADS")
            .help("Concurrency limit")
            .value_parser(value_parser!(u8))
            .hide_default_value(true)
            .default_value("0"));

    matches.get_matches()
}

fn text_prompt(message: &str) -> Result<String, Error> {
    let mut line = String::new();
    print!("{message}");

    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_string())
}

fn pass_prompt(message: &str) -> Option<String> {
    let pass = rpassword::prompt_password(message);
    match pass {
        Ok(pass) => Some(pass),
        _ => None,
    }
}

fn clone(destination: &str, target: &str, callbacks: RemoteCallbacks) -> Result<(), Error> {
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
        .clone(target, Path::new(&destination))?;

    Ok(())
}

fn main() -> Result<(), Error> {
    let matches = args();
    let path = matches.get_one::<PathBuf>("path").unwrap();
    let threads = *matches.get_one::<u8>("threads").unwrap();
    let spinner = ProgressBar::new_spinner();
    let mut config = Config::default();
    let mut credentials = Credentials::default();
    let mut needs_password = false;

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        config.load_config(config_path)?;
    } else {
        config.open()?;
    }

    println!("{BANNER}");
    set_threads(threads);
    config.check()?;

    if let Some(pwd) = config.ssh_pass_protected {
        if config.ssh_pass_protected == Some(true) {
            credentials.ssh_password = pass_prompt("Enter \x1b[1mSSH\x1b[0m key password:");
            needs_password = pwd;
        }
    }

    spinner.enable_steady_tick(std::time::Duration::from_millis(90));
    spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));

    config.targets.par_iter().for_each(|target| {
        let mut callbacks = RemoteCallbacks::new();
        let destination = format!("{}/{}.dorst", &path.display(), get_name(target));
        let target_name = get_name(target);

        if Path::new(&destination).exists() {
            match fs::remove_dir_all(&destination) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("\x1b[1;31mError:\x1b[0m {error}");
                    std::process::exit(1)
                }
            }
        }

        spinner.set_message(format!(
            "\x1b[36mpulling\x1b[0m \x1b[33m{target_name}\x1b[0m"
        ));

        if let Some(ref ssh_key) = config.ssh_key {
            callbacks.credentials(|_url, username_from_url, allowed_types| {
                // TODO
                // `is_user_pass_plaintext`?
                if allowed_types.is_ssh_key() {
                    let key = shellexpand::tilde(ssh_key.to_str().unwrap()).into_owned();
                    let key_path = PathBuf::from(&key);
                    if needs_password {
                        credentials.ssh_password.clone().map_or_else(
                            || Cred::default(),
                            |pwd| {
                                Cred::ssh_key(
                                    username_from_url.unwrap(),
                                    None,
                                    &key_path,
                                    Some(&pwd),
                                )
                            },
                        )
                    } else {
                        Cred::ssh_key(username_from_url.unwrap(), None, &key_path, None)
                    }
                } else {
                    Cred::default()
                }
            });
        }

        match clone(&destination, target, callbacks) {
            Ok(_) => {}
            Err(error) => {
                eprintln!("\x1b[1;31mError:\x1b[0m {target_name}: {error}");
            }
        };
    });

    spinner.finish_with_message("\x1b[1;32mDONE\x1b[0m");

    Ok(())
}
