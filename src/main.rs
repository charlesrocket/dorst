use clap::Parser;
use git2::RemoteCallbacks;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};

use std::{env, fs, path::Path};

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
    #[command(author, version, about, long_about = None)]
    struct Args {
        #[arg(short, long, default_value_t = get_dir(), hide_default_value = true)]
        path: String,
        #[arg()]
        targets: String,
    }

    let args = Args::parse();
    let config = std::fs::File::open(args.targets).expect("Could not open config file.");
    let scrape_config: Config =
        serde_yaml::from_reader(config).expect("Could not read config values.");

    for target in scrape_config.targets.iter() {
        let dest = format!["{0}/{1}", &args.path, get_name(target)];
        if Path::new(&dest).exists() {
            fs::remove_dir_all(&dest).unwrap();
        }

        let callbacks = RemoteCallbacks::new();
        let mut options = git2::FetchOptions::new();
        let mut repo = git2::build::RepoBuilder::new();
        let builder = repo
            .bare(true)
            .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

        options.remote_callbacks(callbacks);
        builder.fetch_options(options);
        builder.clone(target, Path::new(&dest)).unwrap();
    }
}
