use anyhow::Result;
use git2::{AutotagOption, Cred, FetchOptions, RemoteCallbacks, Repository};
#[cfg(feature = "gui")]
use glib::Sender;
#[cfg(feature = "cli")]
use indicatif::{HumanBytes, ProgressBar};

#[cfg(feature = "gui")]
use crate::gui::window::Message;
use crate::util::get_name;

use std::{
    io::{self, Write},
    path::Path,
};

pub fn set_callbacks(git_config: &git2::Config) -> RemoteCallbacks {
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

pub fn set_default_branch(mirror: &Repository) -> Result<(), git2::Error> {
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

pub fn clone_repo(
    target: &str,
    destination: &str,
    bare: bool,
    #[cfg(feature = "cli")] spinner: Option<&ProgressBar>,
    #[cfg(feature = "gui")] tx: &Option<Sender<Message>>,
    git_config: &git2::Config,
    #[cfg(feature = "cli")] silent: Option<bool>,
) -> Result<Repository, git2::Error> {
    let mut callbacks = set_callbacks(git_config);
    let target_name = get_name(target);

    #[cfg(feature = "cli")]
    if silent == Some(false) {
        callbacks.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                spinner.unwrap().set_message(format!(
                    "\x1b[35mpulling\x1b[0m \x1b[93m{target_name}\
                     \x1b[0m resolving deltas {}/{}",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                ));
            } else if stats.total_objects() > 0 {
                spinner.unwrap().set_message(format!(
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

    #[cfg(feature = "gui")]
    if tx.is_some() {
        let _ = tx.clone().unwrap().send(Message::Clone);

        callbacks.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                let _ = tx.clone().unwrap().send(Message::Deltas);
                let indexed = stats.indexed_deltas() as f64;
                let total = stats.total_deltas() as f64;
                let progress = indexed / total;
                let _ = tx.clone().unwrap().send(Message::Progress(progress));
            } else if stats.total_objects() > 0 {
                let received = stats.received_objects() as f64;
                let total = stats.total_objects() as f64;
                let progress = received / total;
                let _ = tx.clone().unwrap().send(Message::Progress(progress));
            }

            true
        });
    }

    if bare {
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
    } else {
        let mut fetch_options = FetchOptions::new();
        let checkout_options = git2::build::CheckoutBuilder::new();

        fetch_options.remote_callbacks(callbacks);

        let repo = git2::build::RepoBuilder::new()
            .fetch_options(fetch_options)
            .with_checkout(checkout_options)
            .clone(target, Path::new(&destination))?;

        Ok(repo)
    }
}

pub fn fetch_repo(
    target: &str,
    path: &str,
    #[cfg(feature = "cli")] spinner: Option<&ProgressBar>,
    #[cfg(feature = "gui")] tx: &Option<Sender<Message>>,
    git_config: &git2::Config,
    #[cfg(feature = "cli")] silent: Option<bool>,
) -> Result<Repository, git2::Error> {
    let mirror = Repository::open(path)?;
    let target_name = get_name(target);

    {
        let mut callbacks = set_callbacks(git_config);
        let mut fetch_options = FetchOptions::new();
        let mut remote = mirror
            .find_remote("origin")
            .or_else(|_| mirror.remote_anonymous(target))?;

        #[cfg(feature = "cli")]
        if silent == Some(false) {
            callbacks.sideband_progress(|data| {
                spinner.unwrap().set_message(format!(
                    "\x1b[35mpulling\x1b[0m \x1b[93m{target_name}\
                     \x1b[0m remote: {}",
                    std::str::from_utf8(data).unwrap()
                ));

                io::stdout().flush().unwrap();

                true
            });

            callbacks.update_tips(|refname, a, b| {
                if a.is_zero() {
                    spinner
                        .unwrap()
                        .set_message(format!("[new]     {b:20} {refname}"));
                } else {
                    spinner
                        .unwrap()
                        .set_message(format!("[updated] {a:10}..{b:10} {refname}"));
                }

                true
            });

            callbacks.transfer_progress(|stats| {
                if stats.received_objects() == stats.total_objects() {
                    spinner.unwrap().set_message(format!(
                        "\x1b[35mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m resolving deltas {}/{}",
                        stats.indexed_deltas(),
                        stats.total_deltas()
                    ));
                } else if stats.total_objects() > 0 {
                    spinner.unwrap().set_message(format!(
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

        #[cfg(feature = "gui")]
        if tx.is_some() {
            let _ = tx.clone().unwrap().send(Message::Fetch);

            callbacks.transfer_progress(|stats| {
                if stats.received_objects() == stats.total_objects() {
                    let _ = tx.clone().unwrap().send(Message::Deltas);
                    let indexed = stats.indexed_deltas() as f64;
                    let total = stats.total_deltas() as f64;
                    let progress = indexed / total;
                    let _ = tx.clone().unwrap().send(Message::Progress(progress));
                } else if stats.total_objects() > 0 {
                    let received = stats.received_objects() as f64;
                    let total = stats.total_objects() as f64;
                    let progress = received / total;
                    let _ = tx.clone().unwrap().send(Message::Progress(progress));
                }

                true
            });
        }

        fetch_options.remote_callbacks(callbacks);
        remote.download(&[] as &[&str], Some(&mut fetch_options))?;

        {
            #[cfg(feature = "cli")]
            if silent == Some(false) {
                let stats = remote.stats();

                if stats.local_objects() > 0 {
                    spinner.unwrap().set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m received {}/{} in {} (used {} local objects)",
                        stats.indexed_objects(),
                        stats.total_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap()),
                        stats.local_objects()
                    ));
                } else {
                    spinner.unwrap().set_message(format!(
                        "\x1b[94mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m received {}/{} in {}",
                        stats.indexed_objects(),
                        stats.total_objects(),
                        HumanBytes(stats.received_bytes().try_into().unwrap())
                    ));
                }
            }

            #[cfg(feature = "gui")]
            if tx.is_some() {
                let stats = remote.stats();
                let indexed = stats.indexed_objects() as f64;
                let total = stats.total_objects() as f64;
                let progress = indexed / total;
                let _ = tx.clone().unwrap().send(Message::Progress(progress));
            }
        }

        let default_branch = remote.default_branch()?;

        mirror.set_head(default_branch.as_str().unwrap())?;
        remote.disconnect()?;
        remote.update_tips(None, true, AutotagOption::Unspecified, None)?;
    }

    Ok(mirror)
}

pub fn clone_target(
    destination: &str,
    target: &str,
    mirror: bool,
    #[cfg(feature = "cli")] spinner: Option<&ProgressBar>,
    #[cfg(feature = "gui")] tx: &Option<Sender<Message>>,
    #[cfg(feature = "cli")] silent: Option<bool>,
) -> Result<()> {
    let git_config = git2::Config::open_default()?;

    if Path::new(&destination).exists() {
        fetch_repo(
            target,
            destination,
            #[cfg(feature = "cli")]
            spinner,
            #[cfg(feature = "gui")]
            tx,
            &git_config,
            #[cfg(feature = "cli")]
            silent,
        )?
    } else {
        clone_repo(
            target,
            destination,
            mirror,
            #[cfg(feature = "cli")]
            spinner,
            #[cfg(feature = "gui")]
            tx,
            &git_config,
            #[cfg(feature = "cli")]
            silent,
        )?
    };

    Ok(())
}
