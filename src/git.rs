use git2::{AutotagOption, Cred, RemoteCallbacks};

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{error::Error, util::copy_dir};

fn get_name(target: &str) -> &str {
    target.rsplit('/').next().unwrap_or(target)
}

pub fn callbacks(
    ssh_key: Option<PathBuf>,
    needs_password: bool,
    password: Option<String>,
) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        if allowed_types.is_ssh_key() {
            ssh_key.as_ref().map_or_else(Cred::default, |ssh_key| {
                let key = shellexpand::tilde(ssh_key.to_str().unwrap()).into_owned();
                let key_path = PathBuf::from(&key);

                if needs_password {
                    password.as_ref().map_or_else(Cred::default, |pwd| {
                        Cred::ssh_key(username_from_url.unwrap(), None, &key_path, Some(pwd))
                    })
                } else {
                    Cred::ssh_key(username_from_url.unwrap(), None, &key_path, None)
                }
            })
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
