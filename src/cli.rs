use anyhow::{anyhow, Result};
use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

#[cfg(feature = "logs")]
use tracing::{error, info};

use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    git,
    util::{expand_path, get_dir, get_name, version_string, xdg_path},
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
    source_directory: String,
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

        for target in &config.targets {
            if target.ends_with('/') {
                return Err(anyhow!("Invalid URL {} (trailing slash)", &target));
            }
        }

        Ok(Self {
            source_directory: config.source_directory,
            targets: config.targets,
            count: config_count,
        })
    }

    fn open(&mut self, file_path: &PathBuf) -> Result<()> {
        if !Path::new(&file_path).exists() {
            println!("\x1b[7m DORST: Initialization \x1b[0m");

            let dir = file_path.parent().unwrap();

            let source_prompt =
                text_prompt("\x1b[7m Enter source storage  \n directory (~/src):    \x1b[0m ");

            let target_prompt =
                text_prompt("\x1b[7m Enter backup targets  \n separated by a comma: \x1b[0m ");

            let source = source_prompt?;

            let target: Vec<String> = target_prompt?
                .split(',')
                .map(|target| {
                    let mut target_string = String::from(target);
                    if target_string.ends_with('/') {
                        target_string.pop();
                    }

                    target_string
                })
                .collect();

            let config = Self {
                source_directory: source,
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
        let config = Self::read(path)?;
        self.source_directory = config.source_directory;
        self.targets = config.targets;
        self.count = config.count;

        Ok(())
    }
}

fn text_prompt(message: &str) -> Result<String> {
    let mut line = String::new();
    print!("{message}");

    std::io::stdout().flush()?;
    std::io::stdin().read_line(&mut line)?;

    Ok(line.trim().to_owned())
}

fn args() -> ArgMatches {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .about(format!("{}\n{}", BANNER, env!("CARGO_PKG_DESCRIPTION")))
        .version(version_string())
        .help_template(
            "{name} v{version} CLI\n{about-with-newline}\
             \n{usage-heading} \
             {usage}\n\n{all-args}{after-help}",
        )
        .args([
            Arg::new("path")
                .action(ArgAction::Set)
                .value_name("PATH")
                .help("Backup destination")
                .value_parser(value_parser!(PathBuf))
                .hide_default_value(true)
                .default_value(get_dir()),
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Use alternative config file")
                .value_parser(value_parser!(PathBuf)),
            Arg::new("backups")
                .short('b')
                .long("backups")
                .help("Enable backups")
                .action(ArgAction::SetTrue),
            Arg::new("purge")
                .short('p')
                .long("purge")
                .help("Purge current data")
                .action(ArgAction::SetTrue),
            Arg::new("silent")
                .short('s')
                .long("silent")
                .help("Do not output status")
                .action(ArgAction::SetTrue),
            #[cfg(feature = "logs")]
            Arg::new("logs")
                .long("no-log")
                .help("Disable logging")
                .action(ArgAction::SetFalse),
        ]);

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

fn cli(matches: &ArgMatches) -> Result<()> {
    #[cfg(feature = "logs")]
    let _logger = crate::util::init_logs();

    println!("{BANNER}");

    let path = matches.get_one::<PathBuf>("path").unwrap();
    let purge = matches.get_flag("purge");
    let repo_mirror = matches.get_flag("backups");
    let silent = matches.get_flag("silent");
    #[cfg(feature = "logs")]
    let logs = matches.get_flag("logs");
    let mut config = Config::default();

    if let Some(config_path) = matches.get_one::<PathBuf>("config") {
        config.open(config_path)?;
    } else {
        config.open(&xdg_path()?)?;
    }

    #[cfg(feature = "logs")]
    if logs {
        info!("Started");
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
        let destination_clone = format!(
            "{}/{}",
            PathBuf::from(expand_path(&config.source_directory)).display(),
            get_name(&target)
        );

        let destination_backup = format!("{}/{}.dorst", &path.display(), get_name(&target));
        let target_name = get_name(&target);

        if !silent {
            spinner.tick();
            spinner.set_style(ProgressStyle::default_spinner().tick_strings(&SPINNER));
            spinner.set_message(format!(
                "\x1b[1;96mstarting\x1b[0m \x1b[93m{target_name}\x1b[0m"
            ));
        }

        if purge && Path::new(&destination_clone).exists() {
            fs::remove_dir_all(&destination_clone)?;
        }

        if purge && Path::new(&destination_backup).exists() {
            fs::remove_dir_all(&destination_backup)?;
        }

        match process_repo(
            &destination_clone,
            &destination_backup,
            &target,
            repo_mirror,
            Some(&spinner),
            Some(silent),
        ) {
            Ok(_) => {
                #[cfg(feature = "logs")]
                if logs {
                    info!("Completed: {target_name}");
                }

                compl_count += 1;
                if !silent {
                    let status = spinner.prefix();
                    let branch = git::current_branch(destination_clone.into())?;
                    spinner.finish_with_message(format!(
                        "\x1b[1;96mdone\x1b[0m \x1b[0;93m{target_name}\x1b[0m \x1b[4;36m{branch}\x1b[0m{status}"
                    ));
                }
            }

            Err(error) => {
                #[cfg(feature = "logs")]
                if logs {
                    error!("Failed: {target_name} - {error}");
                }

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

    #[cfg(feature = "logs")]
    if logs {
        info!("Finished");
    }

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

fn process_repo(
    destination_clone: &str,
    destination_backup: &str,
    target: &str,
    mirror: bool,
    #[cfg(feature = "cli")] spinner: Option<&ProgressBar>,
    #[cfg(feature = "cli")] silent: Option<bool>,
) -> Result<()> {
    git::process_target(
        destination_clone,
        target,
        false,
        spinner,
        #[cfg(feature = "gui")]
        &None,
        silent,
    )?;

    if mirror {
        if silent == Some(false) {
            spinner.unwrap().set_message(format!(
                "\x1b[1;96mbackup \x1b[0;93m{}\x1b[0m",
                get_name(target)
            ));
        }

        git::process_target(
            destination_backup,
            target,
            true,
            spinner,
            #[cfg(feature = "gui")]
            &None,
            silent,
        )?;
    }

    Ok(())
}

pub fn start() {
    let args = args();
    if let Err(error) = cli(&args) {
        eprintln!("\x1b[1;31mError:\x1b[0m {error}");
        std::process::exit(1);
    }
}
