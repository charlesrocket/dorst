#![forbid(unsafe_code)]

use anyhow::Result;
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use std::{env, fs::remove_dir_all, path::Path, path::PathBuf, sync::Arc};

mod config;
mod git;
mod util;

use crate::{
    config::{xdg_path, Config},
    util::{get_dir, get_name},
};

const BANNER: &str = "\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\
                       \u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\
                        \u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\u{2584}\
                         \u{2584}\u{2584}\u{2584}\n\
                      \u{2588} \u{2584}\u{2580}\u{2588}\u{2580}\u{2584}\u{2584}\
                       \u{2580}\u{2588} \u{2584}\u{2584}\u{2580}\u{2588} \
                        \u{2584}\u{2584}\u{2588}\u{2584} \u{2584}\u{2588}\n\
                      \u{2588} \u{2588} \u{2588} \u{2580}\u{2584} \u{2588} \
                       \u{2580}\u{2580}\u{2584}\u{2588}\u{2584}\u{2584}\
                        \u{2580}\u{2588}\u{2588} \u{2588}\u{2588}\n\
                      \u{2588}\u{2584}\u{2584}\u{2588}\u{2588}\u{2588}\
                       \u{2584}\u{2584}\u{2588}\u{2588}\u{2584}\u{2588}\
                        \u{2584}\u{2584}\u{2588}\u{2584}\u{2584}\u{2584}\
                         \u{2588}\u{2588}\u{2584}\u{2588}\u{2588}";

const SPINNER: [&str; 2] = ["\u{2591}", "\u{2592}"];

const BAR_1: [&str; 3] = ["\u{25a0}", "\u{25a0}", "\u{25a1}"];
const BAR_2: [&str; 3] = ["+", "+", "-"];

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

fn main() -> Result<()> {
    println!("{BANNER}");

    let matches = args();
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
            remove_dir_all(&destination)?;
        }

        match git::mirror(&destination, &target, &spinner, silent) {
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

    if err_count > 0 {
        eprintln!(
            "┗╸\x1b[1mCOMPLETED\x1b[0m \x1b[37m(\x1b[0m\x1b[1;92m{compl_count}\
             \x1b[0m\x1b[37m/\x1b[0m\x1b[1;91m{err_count}\x1b[0m\x1b[37m)\x1b[0m"
        );

        std::process::exit(1);
    } else {
        println!(
            "┗╸\x1b[1mCOMPLETED\x1b[0m \x1b[37m(\x1b[0m\x1b[1;92m{compl_count}\
             \x1b[0m\x1b[37m)\x1b[0m"
        );
    }

    Ok(())
}
