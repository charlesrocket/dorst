use anyhow::{anyhow, Result};
use git2::{AutotagOption, Cred, FetchOptions, RemoteCallbacks, Repository};
use indicatif::{HumanBytes, ProgressBar};

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use crate::util::get_name;

fn clone(
    destination: &str,
    target: &str,
    spinner: &ProgressBar,
    git_config: &git2::Config,
    silent: bool,
) -> Result<Repository, git2::Error> {
    let mut callbacks = RemoteCallbacks::new();
    let target_name = get_name(target);

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

    let mut fetch_options = git2::FetchOptions::new();
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

    Ok(mirror)
}

fn fetch(
    target: &str,
    path: &str,
    spinner: &ProgressBar,
    silent: bool,
) -> Result<Repository, git2::Error> {
    let repo = Repository::open(path)?;
    let target_name = get_name(target);

    {
        let mut cb = RemoteCallbacks::new();
        let mut remote = repo
            .find_remote("origin")
            .or_else(|_| repo.remote_anonymous(target))?;

        if !silent {
            cb.sideband_progress(|data| {
                spinner.set_message(format!("remote: {}", std::str::from_utf8(data).unwrap()));
                io::stdout().flush().unwrap();

                true
            });

            cb.update_tips(|refname, a, b| {
                if a.is_zero() {
                    spinner.set_message(format!("[new]     {:20} {}", b, refname));
                } else {
                    spinner.set_message(format!("[updated] {:10}..{:10} {}", a, b, refname));
                }

                true
            });

            cb.transfer_progress(|stats| {
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

                io::stdout().flush().unwrap();

                true
            });
        }

        let mut fo = FetchOptions::new();

        fo.remote_callbacks(cb);
        remote.download(&[] as &[&str], Some(&mut fo))?;

        {
            if !silent {
                let stats = remote.stats();

                if stats.local_objects() > 0 {
                    spinner.set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\x1b[0m received {}/{} in {} (used {} local objects)",
                        stats.indexed_objects(),
                        stats.total_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap()),
                        stats.local_objects()
                    ));
                } else {
                    spinner.set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\x1b[0m received {}/{} in {}",
                        stats.indexed_objects(),
                        stats.total_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap())
                    ));
                }
            }
        }

        remote.disconnect()?;
        remote.update_tips(None, true, AutotagOption::Unspecified, None)?;
    }

    Ok(repo)
}

fn update_refs(mirror: &Repository) -> Result<()> {
    let mut string = String::new();

    for reference in mirror.references()? {
        let reference = reference?;

        if let Some(target) = reference.target() {
            string.push_str(&format!("{}\t{}\n", target, reference.name().unwrap()));
        }
    }

    let destination = mirror.path().join("info");

    if !destination.exists() {
        fs::create_dir_all(&destination)?;
    }

    let info = destination.join("refs");

    fs::write(info, string)?;

    Ok(())
}

fn set_head(mirror: &Repository) -> Result<()> {
    let repo = mirror;
    let remote = repo.find_remote("origin")?;
    let remote_branch = remote.name().unwrap();
    let remote_branch_ref = repo.resolve_reference_from_short_name(remote_branch)?;
    let remote_branch_name = remote_branch_ref
        .name()
        .ok_or_else(|| anyhow!("No default branch"));

    let head = remote_branch_name?.to_owned();

    repo.set_head(&head)?;

    Ok(())
}

pub fn mirror(destination: &str, target: &str, spinner: &ProgressBar, silent: bool) -> Result<()> {
    let git_config = git2::Config::open_default().unwrap();
    let repo = if Path::new(&destination).exists() {
        fetch(target, destination, spinner, silent)?
    } else {
        clone(destination, target, spinner, &git_config, silent)?
    };

    update_refs(&repo)?;
    set_head(&repo)?;

    Ok(())
}
