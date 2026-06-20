use anyhow::Result;
#[cfg(feature = "gui")]
use async_channel::Sender;
use git2::{AutotagOption, Cred, FetchOptions, RemoteCallbacks, RemoteUpdateFlags, Repository};
#[cfg(feature = "cli")]
use indicatif::{HumanBytes, ProgressBar};

#[cfg(feature = "gui")]
use crate::gui::window::{RowMessage, Status};
#[cfg(feature = "cli")]
use libdorst::get_name;

#[cfg(feature = "cli")]
use std::io::{self, Write};
use std::path::Path;

pub fn set_callbacks(git_config: &git2::Config) -> RemoteCallbacks<'_> {
    let mut callbacks = RemoteCallbacks::new();
    let mut tried_ssh_agent = false;
    let mut tried_cred_helper = false;

    callbacks.credentials(move |url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::USERNAME) {
            return Cred::username(username_from_url.unwrap_or("git"));
        }

        if allowed_types.contains(git2::CredentialType::SSH_KEY) && !tried_ssh_agent {
            tried_ssh_agent = true;
            return Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"));
        }

        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) && !tried_cred_helper {
            tried_cred_helper = true;
            return Cred::credential_helper(git_config, url, username_from_url);
        }

        if allowed_types.contains(git2::CredentialType::DEFAULT) {
            return Cred::default();
        }

        Err(git2::Error::from_str("No valid auth method found"))
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

pub fn current_branch(destination: std::path::PathBuf) -> Result<String, git2::Error> {
    let repo = Repository::open(destination)?;
    let head = repo.head()?;

    if head.is_branch() {
        match head.shorthand() {
            Some(branch) => Ok(String::from(branch)),
            _ => Ok(String::from("*INVALID")),
        }
    } else {
        Ok(String::from("*DETACHED"))
    }
}

pub fn clone_repo(
    target: &str,
    destination: &str,
    bare: bool,
    #[cfg(feature = "cli")] spinner: Option<&ProgressBar>,
    #[cfg(feature = "gui")] tx: Option<&Sender<RowMessage>>,
    git_config: &git2::Config,
    #[cfg(feature = "cli")] silent: Option<bool>,
) -> Result<(), git2::Error> {
    let mut callbacks = set_callbacks(git_config);
    #[cfg(feature = "cli")]
    let target_name = get_name(target);

    #[cfg(feature = "cli")]
    if silent == Some(false) {
        callbacks.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                spinner.unwrap().set_message(format!(
                    "\x1b[1;35mpulling\x1b[0m \x1b[93m{target_name}\
                     \x1b[0m resolving deltas {}/{}",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                ));
            } else if stats.total_objects() > 0 {
                spinner.unwrap().set_message(format!(
                    "\x1b[1;94mpulling\x1b[0m \x1b[93m{target_name}\
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
    if let Some(tx) = tx {
        let _ = tx.send_blocking(RowMessage::Clone);

        callbacks.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                let _ = tx.send_blocking(RowMessage::Deltas);
                let indexed = stats.indexed_deltas() as f64;
                let total = stats.total_deltas() as f64;
                let progress = indexed / total;
                let _ = tx.send_blocking(RowMessage::Progress(progress, Status::Deltas));
            } else if stats.total_objects() > 0 {
                let received = stats.received_objects() as f64;
                let total = stats.total_objects() as f64;
                let progress = received / total;
                let _ = tx.send_blocking(RowMessage::Progress(progress, Status::Data));
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

        Ok(())
    } else {
        let mut fetch_options = FetchOptions::new();
        let checkout_options = git2::build::CheckoutBuilder::new();

        fetch_options.remote_callbacks(callbacks);

        git2::build::RepoBuilder::new()
            .fetch_options(fetch_options)
            .with_checkout(checkout_options)
            .clone(target, Path::new(&destination))?;

        Ok(())
    }
}

pub fn fetch_repo(
    target: &str,
    repo: &Repository,
    mirror: bool,
    #[cfg(feature = "cli")] spinner: Option<&ProgressBar>,
    #[cfg(feature = "gui")] tx: Option<&Sender<RowMessage>>,
    git_config: &git2::Config,
    #[cfg(feature = "cli")] silent: Option<bool>,
) -> Result<(), git2::Error> {
    #[cfg(feature = "cli")]
    let target_name = get_name(target);

    {
        let mut callbacks = set_callbacks(git_config);
        let mut fetch_options = FetchOptions::new();
        let mut remote = repo
            .find_remote("origin")
            .or_else(|_| repo.remote_anonymous(target))?;

        #[cfg(feature = "cli")]
        if silent == Some(false) {
            callbacks.sideband_progress(|data| {
                spinner.unwrap().set_message(format!(
                    "\x1b[1;35mpulling\x1b[0m \x1b[93m{target_name}\
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
                        "\x1b[1;35mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m resolving deltas {}/{}",
                        stats.indexed_deltas(),
                        stats.total_deltas()
                    ));
                } else if stats.total_objects() > 0 {
                    spinner.unwrap().set_message(format!(
                        "\x1b[1;94mpulling\x1b[0m \x1b[93m{target_name}\
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
        if let Some(tx) = tx {
            let _ = tx.send_blocking(RowMessage::Fetch);

            callbacks.transfer_progress(|stats| {
                if stats.received_objects() == stats.total_objects() {
                    let _ = tx.send_blocking(RowMessage::Deltas);
                    let indexed = stats.indexed_deltas() as f64;
                    let total = stats.total_deltas() as f64;
                    let progress = indexed / total;
                    let _ = tx.send_blocking(RowMessage::Progress(progress, Status::Deltas));
                } else if stats.total_objects() > 0 {
                    let received = stats.received_objects() as f64;
                    let total = stats.total_objects() as f64;
                    let progress = received / total;
                    let _ = tx.send_blocking(RowMessage::Progress(progress, Status::Data));
                }

                true
            });
        }

        fetch_options.remote_callbacks(callbacks);
        remote.download(&[] as &[&str], Some(&mut fetch_options))?;

        #[cfg(feature = "cli")]
        if silent == Some(false) {
            let stats = remote.stats();

            if stats.local_objects() > 0 {
                spinner.unwrap().set_message(format!(
                    "\x1b[1;94mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m received {}/{} in {} (used {} local objects)",
                    stats.indexed_objects(),
                    stats.total_objects(),
                    HumanBytes(stats.received_bytes().try_into().unwrap()),
                    stats.local_objects()
                ));
            } else {
                spinner.unwrap().set_message(format!(
                    "\x1b[1;94mpulling\x1b[0m \x1b[93m{target_name}\
                         \x1b[0m received {}/{} in {}",
                    stats.indexed_objects(),
                    stats.total_objects(),
                    HumanBytes(stats.received_bytes().try_into().unwrap())
                ));
            }
        }

        #[cfg(feature = "gui")]
        if let Some(tx) = tx {
            let stats = remote.stats();
            let indexed = stats.indexed_objects() as f64;
            let total = stats.total_objects() as f64;
            let progress = indexed / total;
            let _ = tx.send_blocking(RowMessage::Progress(progress, Status::Normal));
        }

        remote.disconnect()?;
        remote.update_tips(
            None,
            RemoteUpdateFlags::UPDATE_FETCHHEAD,
            AutotagOption::Unspecified,
            None,
        )?;

        let local_oid = repo.refname_to_id("HEAD")?;
        let remote_oid = repo.refname_to_id("FETCH_HEAD")?;

        if local_oid != remote_oid && !mirror {
            #[cfg(feature = "cli")]
            if silent == Some(false) {
                spinner.unwrap().set_prefix(" ø");
            }
            #[cfg(feature = "gui")]
            if let Some(tx) = tx {
                let _ = tx.send_blocking(RowMessage::Updated(String::from(target)));
            }
        }
    }

    Ok(())
}

pub fn process_target(
    destination: &str,
    target: &str,
    mirror: bool,
    #[cfg(feature = "cli")] spinner: Option<&ProgressBar>,
    #[cfg(feature = "gui")] tx: Option<&Sender<RowMessage>>,
    #[cfg(feature = "cli")] silent: Option<bool>,
) -> Result<()> {
    let git_config = git2::Config::open_default()?;

    if Path::new(&destination).exists() {
        let repo = Repository::open(destination)?;

        fetch_repo(
            target,
            &repo,
            mirror,
            #[cfg(feature = "cli")]
            spinner,
            #[cfg(feature = "gui")]
            tx,
            &git_config,
            #[cfg(feature = "cli")]
            silent,
        )?;
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
        )?;
    }

    Ok(())
}
