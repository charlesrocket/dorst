use gtk::glib::{self, Object};
use gtk::subclass::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "logs")]
use tracing::{error, info};

use std::sync::{Arc, Mutex};

use crate::{git, gui::window::RowMessage};

mod imp;

glib::wrapper! {
    pub struct RepoObject(ObjectSubclass<imp::RepoObject>);
}

enum RepoMessage {
    Ok,
    Error(String),
    Reset,
    Finish(bool),
}

impl RepoObject {
    pub fn new(
        name: String,
        link: String,
        branch: String,
        progress: f64,
        status: String,
        error: String,
        completed: bool,
    ) -> Self {
        Object::builder()
            .property("name", name)
            .property("link", link)
            .property("branch", branch)
            .property("progress", progress)
            .property("status", status)
            .property("error", error)
            .property("completed", completed)
            .build()
    }

    pub fn repo_data(&self) -> RepoData {
        self.imp().data.borrow().clone()
    }

    pub fn from_repo_data(repo_data: RepoData) -> Self {
        Self::new(
            repo_data.name,
            repo_data.link,
            repo_data.branch,
            repo_data.progress,
            repo_data.status,
            repo_data.error,
            repo_data.completed,
        )
    }

    pub fn process_repo(
        &self,
        destination_clone: &str,
        destination_backup: &str,
        mirror: bool,
        #[cfg(feature = "gui")] tx: Option<async_channel::Sender<RowMessage>>,
        #[cfg(feature = "logs")] logs: bool,
        active_threads: Arc<Mutex<u64>>,
    ) {
        let repo = self.clone();
        let repo_link = self.link();
        #[cfg(feature = "logs")]
        let repo_name = self.name();
        let dest_clone = String::from(destination_clone);
        let dest_backup = String::from(destination_backup);

        let (tx_repo, rx_repo) = async_channel::unbounded();

        let event_handler = async move {
            while let Ok(event) = rx_repo.recv().await {
                match event {
                    RepoMessage::Ok => {
                        repo.set_status("ok");
                    }
                    RepoMessage::Error(value) => {
                        repo.set_error(value);
                        repo.set_status("err");
                    }
                    RepoMessage::Reset => {
                        repo.set_status("started");
                    }
                    RepoMessage::Finish(value) => {
                        repo.set_completed(value);
                        repo.set_status("finished");
                    }
                }
            }
        };

        glib::MainContext::default().spawn_local(event_handler);
        gtk::gio::spawn_blocking(move || {
            let mut err_string = String::new();

            match git::process_target(
                &dest_clone,
                &repo_link,
                false,
                #[cfg(feature = "cli")]
                None,
                #[cfg(feature = "gui")]
                &tx,
                #[cfg(feature = "cli")]
                None,
            ) {
                Ok(()) => {
                    #[cfg(feature = "logs")]
                    if logs {
                        info!("Completed: {repo_name}");
                    }

                    let _ = tx_repo.send_blocking(RepoMessage::Ok);
                }
                Err(error) => {
                    #[cfg(feature = "logs")]
                    if logs {
                        error!("Failed: {repo_name} - {error}");
                    }

                    err_string.push_str(&format!("{error}"));
                }
            }

            let _ = tx.clone().unwrap().send_blocking(RowMessage::Finish);

            if mirror {
                let _ = tx.clone().unwrap().send_blocking(RowMessage::Reset);
                let _ = tx_repo.send_blocking(RepoMessage::Reset);

                match git::process_target(
                    &dest_backup,
                    &repo_link,
                    true,
                    #[cfg(feature = "cli")]
                    None,
                    #[cfg(feature = "gui")]
                    &tx,
                    #[cfg(feature = "cli")]
                    None,
                ) {
                    Ok(()) => {
                        #[cfg(feature = "logs")]
                        if logs {
                            info!("Completed (backup): {repo_name}");
                        }

                        let _ = tx_repo.send_blocking(RepoMessage::Ok);
                    }
                    Err(error) => {
                        #[cfg(feature = "logs")]
                        if logs {
                            error!("Failed (backup): {repo_name} - {error}");
                        }

                        err_string.push_str(&format!(" backup: {error}"));
                    }
                }

                let _ = tx.clone().unwrap().send_blocking(RowMessage::Finish);
            }

            if !err_string.is_empty() {
                let _ = tx_repo.send_blocking(RepoMessage::Error(err_string));
            }

            let _ = tx_repo.send_blocking(RepoMessage::Finish(true));
            *active_threads.lock().unwrap() -= 1;
        });
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RepoData {
    pub name: String,
    pub link: String,
    pub branch: String,
    pub progress: f64,
    pub status: String,
    pub error: String,
    pub completed: bool,
}
