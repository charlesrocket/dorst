use anyhow::Result;
use git2::{Cred, RemoteCallbacks, Repository};

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
