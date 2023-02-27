use git2::{AutotagOption, Cred, RemoteCallbacks};

use std::{fs, path::Path};

use crate::{error::Error, util::copy_dir};

fn get_name(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

pub fn callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    let git_config = git2::Config::open_default().unwrap();

    callbacks.credentials(move |url, username_from_url, allowed_types| {
        if allowed_types.is_user_pass_plaintext() {
            Cred::credential_helper(&git_config, url, username_from_url)
        } else if allowed_types.is_ssh_key() {
            match username_from_url {
                Some(username) => Cred::ssh_key_from_agent(username),
                None => Err(git2::Error::from_str("Could not extract username from URL")),
            }
        } else {
            Cred::default()
        }
    });

    callbacks
}

pub fn clone(
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
