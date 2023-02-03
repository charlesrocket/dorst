use clap::Parser;
use git2::RemoteCallbacks;

use std::path::Path;

fn main() {
    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    struct Args {
        #[arg(short, long)]
        url: String,
        #[arg(short, long)]
        path: String,
    }

    let args = Args::parse();
    let callbacks = RemoteCallbacks::new();
    let mut options = git2::FetchOptions::new();
    let mut repo = git2::build::RepoBuilder::new();
    let builder = repo
        .bare(true)
        .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

    options.remote_callbacks(callbacks);
    builder.fetch_options(options);
    builder.clone(&args.url, Path::new(&args.path)).unwrap();
}
