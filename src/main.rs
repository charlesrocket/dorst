#![forbid(unsafe_code)]

use anyhow::Result;
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};

use std::{
    env::{self, var},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

mod config;
mod git;
mod util;

use crate::config::Config;

const BANNER: &str = "\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\n\
                      \u{2588} \u{2584}\u{2580}\u{2588}\u{2580}\u{2584}\u{2584}\u{2580}\u{2588} \u{2584}\u{2584}\u{2580}\u{2588} \u{2584}\u{2584}\u{2588}\u{2584} \u{2584}\u{2588}\n\
                      \u{2588} \u{2588} \u{2588} \u{2580}\u{2584} \u{2588} \u{2580}\u{2580}\u{2584}\u{2588}\u{2584}\u{2584}\u{2580}\u{2588}\u{2588} \u{2588}\u{2588}\n\
                      \u{2588}\u{2584}\u{2584}\u{2588}\u{2588}\u{2588}\u{2584}\u{2584}\u{2588}\u{2588}\u{2584}\u{2588}\u{2584}\u{2584}\u{2588}\u{2584}\u{2584}\u{2584}\u{2588}\u{2588}\u{2584}\u{2588}\u{2588}";

const SPINNER: [&str; 2] = ["\u{2591}", "\u{2592}"];

const BAR_1: [&str; 3] = ["\u{25a0}", "\u{25a0}", "\u{25a1}"];
const BAR_2: [&str; 3] = ["+", "+", "-"];

fn get_name(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

fn get_dir() -> String {
    let current_dir = env::current_dir().unwrap();
    current_dir.to_str().unwrap().to_owned()
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
            Arg::new("silent")
                .short('s')
                .long("silent")
                .help("Do not output status")
                .action(ArgAction::SetTrue),
        );

    matches.get_matches()
}

fn text_prompt(message: &str) -> Result<String> {
    let mut line = String::new();
    print!("{message}");

    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_owned())
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

fn main() -> Result<()> {
    println!("{BANNER}");

    let matches = args();
    let path = matches.get_one::<PathBuf>("path").unwrap();
    let silent = matches.get_flag("silent");
    let mut config = Config::default();
    let cache_dir = format!(
        "{}/dorst",
        var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_owned())
    );

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        config.open(config_path)?;
    } else {
        let xdg_config_home =
            var("XDG_CONFIG_HOME").unwrap_or(format!("{}/.config", std::env::var("HOME")?));

        let config_path = format!("{xdg_config_home}/dorst");
        let file_path = format!("{config_path}/config.yaml");
        if !Path::new(&config_path).exists() {
            fs::create_dir_all(config_path)?;
        }

        config.open(&PathBuf::from(file_path))?;
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
        let mut callbacks = git::callbacks();
        let destination = format!("{}/{}.dorst", &path.display(), get_name(&target));
        let target_name = get_name(&target);

        if !silent {
            spinner.tick();
            spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));
            spinner.set_message(format!(
                "\x1b[96mstarting\x1b[0m \x1b[93m{target_name}\x1b[0m"
            ));
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
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\x1b[0m received {}/{} | indexed {} in {}",
                        stats.received_objects(),
                        stats.total_objects(),
                        stats.indexed_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap())
                    ));
                }

                true
            });
        }

        match git::clone(&destination, &target, &cache_dir, callbacks) {
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
                        println!("{}", &err);
                    }

                    spinner.finish_with_message(err);
                }
            }
        };

        progress_bar.inc(1);
    }

    progress_bar.finish();

    let report = format!(
        "┗╸\x1b[1mCOMPLETED\x1b[0m \x1b[1;92m{compl_count}\x1b[0m \x1b[37m/\x1b[0m \x1b[1;91m{err_count}\x1b[0m"
    );

    println!("{report}");

    if Path::new(&cache_dir).exists() {
        fs::remove_dir_all(&cache_dir)?;
    }

    Ok(())
}
