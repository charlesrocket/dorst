use anyhow::{Context, Result};
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use git2::{AutotagOption, FetchOptions, Repository};
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    git,
    util::{get_dir, get_name, text_prompt, xdg_path},
};

const BANNER: &str = "\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\
                      \u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\
                      \u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\
                      \u{2584}\u{2584}\u{2584}\n\u{2588} \u{2584}\u{2580}\
                      \u{2588}\u{2580}\u{2584}\u{2584}\u{2580}\u{2588} \
                      \u{2584}\u{2584}\u{2580}\u{2588} \u{2584}\u{2584}\
                      \u{2588}\u{2584} \u{2584}\u{2588}\n\u{2588} \u{2588} \
                      \u{2588} \u{2580}\u{2584} \u{2588} \u{2580}\u{2580}\
                      \u{2584}\u{2588}\u{2584}\u{2584}\u{2580}\u{2588}\u{2588} \
                      \u{2588}\u{2588}\n\u{2588}\u{2584}\u{2584}\u{2588}\
                      \u{2588}\u{2588}\u{2584}\u{2584}\u{2588}\u{2588}\u{2584}\
                      \u{2588}\u{2584}\u{2584}\u{2588}\u{2584}\u{2584}\u{2584}\
                      \u{2588}\u{2588}\u{2584}\u{2588}\u{2588}";

const SPINNER: [&str; 2] = ["\u{2591}", "\u{2592}"];

const BAR_1: [&str; 3] = ["\u{25a0}", "\u{25a0}", "\u{25a1}"];
const BAR_2: [&str; 3] = ["+", "+", "-"];

#[derive(Default, Serialize, Deserialize)]
struct Config {
    targets: Vec<String>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    count: u64,
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

    fn open(&mut self, file_path: &PathBuf) -> Result<()> {
        if !Path::new(&file_path).exists() {
            println!("\x1b[7m DORST: Initialization \x1b[0m");

            let dir = file_path.parent().unwrap();
            let prompt =
                text_prompt("\x1b[7m Enter backup targets  \n separated by a comma: \x1b[0m ");

            let target: Vec<String> = prompt?.split(',').map(ToString::to_string).collect();
            let config = Self {
                targets: target,
                count: 0,
            };

            std::fs::create_dir_all(dir).unwrap();

            let new_config = serde_yaml::to_string(&config)?;
            let mut file = fs::File::create(file_path)?;

            file.write_all(new_config.as_bytes())?;
        }

        let path: PathBuf = file_path.into();
        self.load_config(&path)?;

        Ok(())
    }

    fn load_config(&mut self, path: &PathBuf) -> Result<()> {
        let config = Self::read(path).context("Failed to read the configuration file")?;

        self.targets = config.targets;
        self.count = config.count;

        Ok(())
    }
}

fn args() -> ArgMatches {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .about(BANNER)
        .version(env!("CARGO_PKG_VERSION"))
        .help_template(
            "{name} v{version} CLI\n{about-with-newline}\
             Codebase backup utility\n\n{usage-heading} \
             {usage}\n\n{all-args}{after-help}",
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
            Arg::new("purge")
                .short('p')
                .long("purge")
                .help("Purge current backups")
                .action(ArgAction::SetTrue),
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

fn bar_chars() -> [&'static str; 3] {
    if cfg!(unix) {
        if env::var_os("DISPLAY").is_some() | cfg!(target_os = "macos") {
            BAR_1
        } else {
            BAR_2
        }
    } else {
        BAR_2
    }
}

fn clone(
    target: &str,
    destination: &str,
    spinner: &ProgressBar,
    git_config: &git2::Config,
    silent: bool,
) -> Result<Repository, git2::Error> {
    let mut callbacks = git::set_callbacks(git_config);
    let target_name = get_name(target);

    if !silent {
        callbacks.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                spinner.set_message(format!(
                    "\x1b[35mpulling\x1b[0m \x1b[93m{target_name}\
                     \x1b[0m resolving deltas {}/{}",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                ));
            } else if stats.total_objects() > 0 {
                spinner.set_message(format!(
                    "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\
                     \x1b[0m received {}/{} | indexed {} in {}",
                    stats.received_objects(),
                    stats.total_objects(),
                    stats.indexed_objects(),
                    HumanBytes(stats.received_bytes().try_into().unwrap())
                ));
            }

            true
        });
    }

    let mut fetch_options = FetchOptions::new();
    let mut repo_builder = git2::build::RepoBuilder::new();
    let builder = repo_builder
        .bare(true)
        .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

    fetch_options
        .remote_callbacks(callbacks)
        .download_tags(AutotagOption::All);

    let mirror = builder
        .fetch_options(fetch_options)
        .clone(target, Path::new(&destination))?;

    mirror.config()?.set_bool("remote.origin.mirror", true)?;
    git::set_default_branch(&mirror)?;

    Ok(mirror)
}

fn fetch(
    target: &str,
    path: &str,
    spinner: &ProgressBar,
    git_config: &git2::Config,
    silent: bool,
) -> Result<Repository, git2::Error> {
    let mirror = Repository::open(path)?;
    let target_name = get_name(target);

    {
        let mut callbacks = git::set_callbacks(git_config);
        let mut fetch_options = FetchOptions::new();
        let mut remote = mirror
            .find_remote("origin")
            .or_else(|_| mirror.remote_anonymous(target))?;

        if !silent {
            callbacks.sideband_progress(|data| {
                spinner.set_message(format!(
                    "\x1b[35mpulling\x1b[0m \x1b[93m{target_name}\
                     \x1b[0m remote: {}",
                    std::str::from_utf8(data).unwrap()
                ));

                io::stdout().flush().unwrap();

                true
            });

            callbacks.update_tips(|refname, a, b| {
                if a.is_zero() {
                    spinner.set_message(format!("[new]     {b:20} {refname}"));
                } else {
                    spinner.set_message(format!("[updated] {a:10}..{b:10} {refname}"));
                }

                true
            });

            callbacks.transfer_progress(|stats| {
                if stats.received_objects() == stats.total_objects() {
                    spinner.set_message(format!(
                        "\x1b[35mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m resolving deltas {}/{}",
                        stats.indexed_deltas(),
                        stats.total_deltas()
                    ));
                } else if stats.total_objects() > 0 {
                    spinner.set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m received {}/{} | indexed {} in {}",
                        stats.received_objects(),
                        stats.total_objects(),
                        stats.indexed_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap())
                    ));
                }

                io::stdout().flush().unwrap();

                true
            });
        }

        fetch_options.remote_callbacks(callbacks);
        remote.download(&[] as &[&str], Some(&mut fetch_options))?;

        {
            if !silent {
                let stats = remote.stats();

                if stats.local_objects() > 0 {
                    spinner.set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m received {}/{} in {} (used {} local objects)",
                        stats.indexed_objects(),
                        stats.total_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap()),
                        stats.local_objects()
                    ));
                } else {
                    spinner.set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m received {}/{} in {}",
                        stats.indexed_objects(),
                        stats.total_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap())
                    ));
                }
            }
        }

        let default_branch = remote.default_branch()?;

        mirror.set_head(default_branch.as_str().unwrap())?;
        remote.disconnect()?;
        remote.update_tips(None, true, AutotagOption::Unspecified, None)?;
    }

    Ok(mirror)
}

fn mirror(destination: &str, target: &str, spinner: &ProgressBar, silent: bool) -> Result<()> {
    let git_config = git2::Config::open_default()?;

    if Path::new(&destination).exists() {
        fetch(target, destination, spinner, &git_config, silent)?
    } else {
        clone(target, destination, spinner, &git_config, silent)?
    };

    Ok(())
}

fn cli(matches: ArgMatches) -> Result<()> {
    println!("{BANNER}");

    let path = matches.get_one::<PathBuf>("path").unwrap();
    let purge = matches.get_flag("purge");
    let silent = matches.get_flag("silent");
    let mut config = Config::default();

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        config.open(config_path)?;
    } else {
        config.open(&xdg_path()?)?;
    }

    let indicat = Arc::new(MultiProgress::new());
    let indicat_template = ProgressStyle::with_template("{bar:23}\n{msg}")
        .unwrap()
        .progress_chars(&bar_chars().join(""));

    let mut err_count = 0;
    let mut compl_count = 0;
    let progress_bar = indicat.add(ProgressBar::new(config.count));

    progress_bar.set_style(indicat_template);
    progress_bar.set_position(0);

    for target in config.targets {
        let spinner = indicat.insert_before(&progress_bar, ProgressBar::new_spinner());
        let destination = format!("{}/{}.dorst", &path.display(), get_name(&target));
        let target_name = get_name(&target);

        if !silent {
            spinner.tick();
            spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));
            spinner.set_message(format!(
                "\x1b[96mstarting\x1b[0m \x1b[93m{target_name}\x1b[0m"
            ));
        }

        if purge && Path::new(&destination).exists() {
            fs::remove_dir_all(&destination)?;
        }

        match mirror(&destination, &target, &spinner, silent) {
            Ok(_) => {
                compl_count += 1;
                if !silent {
                    spinner.finish_with_message(format!(
                        "\x1b[96mdone\x1b[0m \x1b[93m{target_name}\x1b[0m"
                    ));
                }
            }

            Err(error) => {
                let err = format!("\x1b[1;31mError:\x1b[0m {target_name}: {error}");
                err_count += 1;

                if !silent {
                    if spinner.is_hidden() {
                        eprintln!("{}", &err);
                    }

                    spinner.finish_with_message(err);
                }
            }
        };

        progress_bar.inc(1);
    }

    progress_bar.finish();

    if err_count > 0 {
        eprintln!(
            "\u{2517}\u{2578}\x1b[1mCOMPLETED\x1b[0m \
             \x1b[37m(\x1b[0m\x1b[1;92m{compl_count}\
             \x1b[0m\x1b[37m/\x1b[0m\x1b[1;91m{err_count}\x1b[0m\x1b[37m)\x1b[0m"
        );

        std::process::exit(1);
    } else {
        println!(
            "\u{2517}\u{2578}\x1b[1mCOMPLETED\x1b[0m \
             \x1b[37m(\x1b[0m\x1b[1;92m{compl_count}\
             \x1b[0m\x1b[37m)\x1b[0m"
        );
    }

    Ok(())
}

pub fn start() {
    let args = args();
    if let Err(error) = cli(args) {
        eprintln!("\x1b[1;31mError:\x1b[0m {error}");
        std::process::exit(1);
    }
}
