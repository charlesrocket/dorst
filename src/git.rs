use anyhow::Result;
use git2::{AutotagOption, Cred, FetchOptions, RemoteCallbacks, Repository};
use indicatif::{HumanBytes, ProgressBar};

use std::{
    io::{self, Write},
    path::Path,
};

use crate::util::get_name;

fn set_callbacks(git_config: &git2::Config) -> RemoteCallbacks {
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(move |url, username_from_url, allowed_types| {
        if allowed_types.is_user_pass_plaintext() {
            Cred::credential_helper(git_config, url, username_from_url)
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

fn set_default_branch(mirror: &Repository) -> Result<(), git2::Error> {
    let remote = mirror.find_remote("origin")?;
    let remote_branch = remote.name().unwrap();
    let remote_branch_ref = mirror.resolve_reference_from_short_name(remote_branch)?;
    let remote_branch_name = remote_branch_ref
        .name()
        .ok_or_else(|| git2::Error::from_str("No default branch"));

    let branch = remote_branch_name?.to_owned();

    mirror.set_head(&branch)?;

    Ok(())
}

fn clone(
    target: &str,
    destination: &str,
    spinner: &ProgressBar,
    git_config: &git2::Config,
    silent: bool,
) -> Result<Repository, git2::Error> {
    let mut callbacks = set_callbacks(git_config);
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
    set_default_branch(&mirror)?;

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
        let mut callbacks = set_callbacks(git_config);
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

pub fn mirror(destination: &str, target: &str, spinner: &ProgressBar, silent: bool) -> Result<()> {
    let git_config = git2::Config::open_default()?;

    if Path::new(&destination).exists() {
        fetch(target, destination, spinner, &git_config, silent)?
    } else {
        clone(target, destination, spinner, &git_config, silent)?
    };

    Ok(())
}
