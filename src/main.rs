#![forbid(unsafe_code)]

use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use git2::{AutotagOption, Cred, RemoteCallbacks};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

mod config;
mod error;

use crate::{config::Config, error::Error};

const BANNER: &str = "\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\n\
                      \u{2588} \u{2584}\u{2580}\u{2588}\u{2580}\u{2584}\u{2584}\u{2580}\u{2588} \u{2584}\u{2584}\u{2580}\u{2588} \u{2584}\u{2584}\u{2588}\u{2584} \u{2584}\u{2588}\n\
                      \u{2588} \u{2588} \u{2588} \u{2580}\u{2584} \u{2588} \u{2580}\u{2580}\u{2584}\u{2588}\u{2584}\u{2584}\u{2580}\u{2588}\u{2588} \u{2588}\u{2588}\n\
                      \u{2588}\u{2584}\u{2584}\u{2588}\u{2588}\u{2588}\u{2584}\u{2584}\u{2588}\u{2588}\u{2584}\u{2588}\u{2584}\u{2584}\u{2588}\u{2584}\u{2584}\u{2584}\u{2588}\u{2588}\u{2584}\u{2588}\u{2588}";

const SPINNER: [&str; 7] = [
    "\u{2591}", "\u{2592}", "\u{2593}", "\u{2591}", "\u{2592}", "\u{2593}", "\u{2591}",
];

const BAR_1: [&str; 3] = ["\u{25a0}", "\u{25a0}", "\u{25a1}"];
const BAR_2: [&str; 3] = ["+", "+", "-"];

#[derive(Default)]
struct Credentials {
    ssh_password: Option<String>,
}

fn get_name(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

fn get_dir() -> String {
    let current_dir = env::current_dir().unwrap();
    current_dir.to_str().unwrap().to_owned()
}

fn set_threads(threads: u8) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads.into())
        .build_global()
        .unwrap();
}

fn args() -> ArgMatches {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .help_template(
            "Codebase backup utility\n\n{usage-heading} {usage}\n\n{all-args}{after-help}",
        )
        .arg(
            Arg::new("path")
                .action(ArgAction::Set)
                .value_name("PATH")
                .help("Backup destination")
                .value_parser(value_parser!(PathBuf))
                .hide_default_value(true)
                .default_value(get_dir()),
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("CONFIG")
                .help("Use alternative config file")
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("threads")
                .short('t')
                .long("threads")
                .value_name("THREADS")
                .help("Concurrency limit")
                .value_parser(value_parser!(u8))
                .hide_default_value(true)
                .default_value("0"),
        )
        .arg(
            Arg::new("silent")
                .short('s')
                .long("silent")
                .help("Do not output status")
                .action(ArgAction::SetTrue),
        );

    matches.get_matches()
}

fn text_prompt(message: &str) -> Result<String, Error> {
    let mut line = String::new();
    print!("{message}");

    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_owned())
}

fn pass_prompt(message: &str) -> Option<String> {
    match rpassword::prompt_password(message) {
        Ok(pass) => Some(pass),
        _ => None,
    }
}

fn bar_chars() -> [&'static str; 3] {
    if cfg!(unix) {
        if env::var_os("DISPLAY").is_some() {
            BAR_1
        } else {
            BAR_2
        }
    } else {
        BAR_2
    }
}

fn clone(
    destination: &str,
    target: &str,
    cache_dir: &str,
    callbacks: RemoteCallbacks,
) -> Result<(), Error> {
    let cache = format!("{cache_dir}/{}-cache", get_name(target));
    let mut fetch_options = git2::FetchOptions::new();
    let mut repo_builder = git2::build::RepoBuilder::new();
    let builder = repo_builder
        .bare(true)
        .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

    if Path::new(&destination).exists() {
        if Path::new(&cache).exists() {
            fs::remove_dir_all(&cache)?;
        }

        match copy_dir(destination, &cache) {
            Ok(_) => {
                fs::remove_dir_all(destination)?;
            }

            Err(error) => {
                error.to_string();
                std::process::exit(1)
            }
        }
    }

    fetch_options
        .remote_callbacks(callbacks)
        .download_tags(AutotagOption::All);

    let mirror = match builder
        .fetch_options(fetch_options)
        .clone(target, Path::new(&destination))
    {
        Ok(repo) => {
            if Path::new(&cache).exists() {
                fs::remove_dir_all(&cache)?;
            }

            Ok(repo)
        }

        Err(error) => Err({
            if Path::new(&cache).exists() {
                copy_dir(&cache, destination)?;
            }

            Error::CloneFailed(error.to_string())
        }),
    };

    let repo = mirror?;
    let remote = repo.find_remote("origin")?;
    let remote_branch = remote.name().unwrap();
    let remote_branch_ref = repo.resolve_reference_from_short_name(remote_branch)?;
    let remote_branch_name = remote_branch_ref
        .name()
        .ok_or_else(|| Error::CloneFailed("No default branch".to_owned()));

    let head = remote_branch_name?.to_owned();

    repo.set_head(&head)?;

    Ok(())
}

fn copy_dir(src: &str, dst: &str) -> Result<(), Error> {
    fs::create_dir_all(dst)?;

    for file in fs::read_dir(src)? {
        let src_file = file?;
        let path = src_file.path();

        if path.is_dir() {
            let sub_dst = format!("{}/{}", dst, path.file_name().unwrap().to_str().unwrap());
            copy_dir(path.to_str().unwrap(), &sub_dst)?;
        } else {
            let dst_file = format!("{}/{}", dst, path.file_name().unwrap().to_str().unwrap());
            fs::copy(&path, &dst_file)?;
        }
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    println!("{BANNER}");

    let matches = args();
    let path = matches.get_one::<PathBuf>("path").unwrap();
    let threads = *matches.get_one::<u8>("threads").unwrap();
    let silent = matches.get_flag("silent");
    let mut config = Config::default();
    let mut credentials = Credentials::default();
    let mut needs_password = false;
    let cache_dir = format!(
        "{}/dorst",
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned())
    );

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        config.load_config(config_path)?;
    } else {
        config.open()?;
    }

    set_threads(threads);
    config.check()?;
    config.check_targets();

    if let Some(pwd) = config.ssh_pass_protected {
        if config.ssh_pass_protected == Some(true) {
            credentials.ssh_password = pass_prompt("\x1b[7mEnter SSH key password:\x1b[0m");
            needs_password = pwd;
        }
    }

    let indicat = Arc::new(MultiProgress::new());
    let indicat_template = ProgressStyle::with_template("{bar:23}")
        .unwrap()
        .progress_chars(&bar_chars().join(""));

    let progress_bar = indicat.add(ProgressBar::new(config.count));

    progress_bar.set_style(indicat_template);
    progress_bar.set_position(0);

    config.targets.par_iter().for_each(|target| {
        let spinner = indicat.add(ProgressBar::new_spinner());
        let mut callbacks = RemoteCallbacks::new();
        let destination = format!("{}/{}.dorst", &path.display(), get_name(target));
        let target_name = get_name(target);

        if !silent {
            spinner.enable_steady_tick(std::time::Duration::from_millis(90));
            spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));
            spinner.set_message(format!(
                "\x1b[96mstarting\x1b[0m \x1b[93m{target_name}\x1b[0m"
            ));
        }

        if let Some(ref ssh_key) = config.ssh_key {
            callbacks.credentials(|_url, username_from_url, allowed_types| {
                // TODO
                // `is_user_pass_plaintext`?
                if allowed_types.is_ssh_key() {
                    let key = shellexpand::tilde(ssh_key.to_str().unwrap()).into_owned();
                    let key_path = PathBuf::from(&key);
                    if needs_password {
                        credentials
                            .ssh_password
                            .clone()
                            .map_or_else(Cred::default, |passphrase| {
                                Cred::ssh_key(
                                    username_from_url.unwrap(),
                                    None,
                                    &key_path,
                                    Some(&passphrase),
                                )
                            })
                    } else {
                        Cred::ssh_key(username_from_url.unwrap(), None, &key_path, None)
                    }
                } else {
                    Cred::default()
                }
            });
        }

        if !silent {
            callbacks.transfer_progress(|stats| {
                if stats.received_objects() == stats.total_objects() {
                    spinner.set_message(format!(
                        "\x1b[35mpulling\x1b[0m \x1b[93m{target_name}\x1b[0m resolving deltas {}/{}",
                        stats.indexed_deltas(),
                        stats.total_deltas()
                    ));

                } else if stats.total_objects() > 0 {
                    spinner.set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\x1b[0m received {}/{} objects ({}) in {} bytes",
                        stats.received_objects(),
                        stats.total_objects(),
                        stats.indexed_objects(),
                        stats.received_bytes()
                    ));
                }

                true
            });
        }

        match clone(&destination, target, &cache_dir, callbacks) {
            Ok(_) => {
                if !silent {
                    spinner.finish_with_message(format!(
                        "\x1b[96mdone\x1b[0m \x1b[93m{target_name}\x1b[0m"
                    ));
                }
            }

            Err(error) => {
                let err = format!("\x1b[1;31mError:\x1b[0m {target_name}: {error}");
                spinner.finish_with_message(err);
            }
        };

        progress_bar.inc(1);
    });

    progress_bar.finish();

    if Path::new(&cache_dir).exists() {
        fs::remove_dir_all(&cache_dir)?;
    }

    Ok(())
}
