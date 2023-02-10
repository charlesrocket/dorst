use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use git2::{AutotagOption, Cred, RemoteCallbacks};
use indicatif::{ProgressBar, ProgressStyle};

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
            .help("Config path")
            .value_parser(value_parser!(PathBuf)));

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

// TODO
// `is_user_pass_plaintext`?
fn callbacks(
    ssh_key: Option<PathBuf>,
    needs_password: bool,
    password: Option<String>,
) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        if allowed_types.is_ssh_key() {
            if let Some(ssh_key) = &ssh_key {
                let key = shellexpand::tilde(ssh_key.to_str().unwrap()).into_owned();
                let key_path = PathBuf::from(&key);
                if needs_password {
                    if let Some(pwd) = &password {
                        Cred::ssh_key(username_from_url.unwrap(), None, &key_path, Some(pwd))
                    } else {
                        Cred::default()
                    }
                } else {
                    Cred::ssh_key(username_from_url.unwrap(), None, &key_path, None)
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
) -> Result<(), Error> {
    let ssh_key = Path::new(&key);
    let callbacks = callbacks(Some(ssh_key.into()), needs_password, password);
    clone(destination, target, callbacks)?;

    Ok(())
}

fn clone_with_defaults(destination: &str, target: &str) -> Result<(), Error> {
    let callbacks = callbacks(None, false, None);
    clone(destination, target, callbacks)?;

    Ok(())
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
    println!("{BANNER}");

    let matches = args();
    let mut config = Config::default();
    let mut needs_password = false;
    let mut credentials = Credentials::default();
    let path = matches.get_one::<PathBuf>("path").unwrap();
    let spinner = ProgressBar::new_spinner();

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        config.load_config(config_path)?;
    } else {
        config.open()?;
    }

    match config.check() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("\x1b[1;31mError:\x1b[0m {error}");
            std::process::exit(1)
        }
    }

    if let Some(pwd) = config.ssh_pass_protected {
        if config.ssh_pass_protected == Some(true) {
            credentials.ssh_password = pass_prompt("Enter \x1b[1mSSH\x1b[0m key password:");
            needs_password = pwd;
        }
    }

    spinner.enable_steady_tick(std::time::Duration::from_millis(90));
    spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));

    for target in config.targets.iter() {
        let destination = format!("{0}/{1}.dorst", &path.display(), get_name(target));
        if Path::new(&destination).exists() {
            fs::remove_dir_all(&destination)?;
        }

        let message = format!("\x1b[36mpulling\x1b[0m \x1b[33m{}\x1b[0m", get_name(target));

        spinner.set_message(message);
        if let Some(ref ssh_key) = config.ssh_key {
            clone_with_key(
                ssh_key,
                &destination,
                target,
                needs_password,
                credentials.ssh_password.clone(),
            )?;
        } else {
            clone_with_defaults(&destination, target)?;
        }
    }

    spinner.finish_with_message("\x1b[1;32mDONE\x1b[0m");

    Ok(())
}
