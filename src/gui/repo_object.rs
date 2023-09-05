use gtk::glib::{self, ControlFlow, MainContext, Object, Priority, Sender};
use gtk::subclass::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "logs")]
use {
    crate::util,
    tracing::{error, info},
};

use crate::{git, gui::window::Message};

mod imp;

glib::wrapper! {
    pub struct RepoObject(ObjectSubclass<imp::RepoObject>);
}

enum RepoMessage {
    Success(bool),
    Error(String),
    Completed(bool),
}

impl RepoObject {
    pub fn new(
        name: String,
        link: String,
        branch: String,
        progress: f64,
        status: String,
        success: bool,
        error: String,
        completed: bool,
    ) -> Self {
        Object::builder()
            .property("name", name)
            .property("link", link)
            .property("branch", branch)
            .property("progress", progress)
            .property("status", status)
            .property("success", success)
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
            repo_data.success,
            repo_data.error,
            repo_data.completed,
        )
    }

    pub fn process_repo(
        &self,
        destination_clone: &str,
        destination_backup: &str,
        mirror: bool,
        #[cfg(feature = "gui")] tx: Option<Sender<Message>>,
        #[cfg(feature = "logs")] logs: bool,
    ) {
        let repo = self.clone();
        let repo_link = self.repo_data().link;
        let dest_clone = String::from(destination_clone);
        let dest_backup = String::from(destination_backup);

        let (tx_repo, rx_repo) = MainContext::channel(Priority::default());

        rx_repo.attach(None, move |x| match x {
            RepoMessage::Success(value) => {
                repo.set_success(value);
                ControlFlow::Continue
            }
            RepoMessage::Error(value) => {
                repo.set_error(value);
                ControlFlow::Continue
            }
            RepoMessage::Completed(value) => {
                repo.set_completed(value);
                ControlFlow::Continue
            }
        });

        gtk::gio::spawn_blocking(move || {
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
                        info!("Completed: {}", util::get_name(&repo_link));
                    }

                    tx_repo.send(RepoMessage::Success(true)).unwrap();
                }
                Err(error) => {
                    #[cfg(feature = "logs")]
                    if logs {
                        error!("Failed: {} - {error}", util::get_name(&repo_link));
                    }

                    tx_repo
                        .send(RepoMessage::Error(format!("{repo_link}: {error}")))
                        .unwrap();
                }
            }

            tx.clone().unwrap().send(Message::Finish).unwrap();

            if mirror {
                tx.clone().unwrap().send(Message::Reset).unwrap();

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
                            info!("Completed (backup): {}", util::get_name(&repo_link));
                        }

                        tx_repo.send(RepoMessage::Success(true)).unwrap();
                    }
                    Err(error) => {
                        #[cfg(feature = "logs")]
                        if logs {
                            error!("Failed (backup): {} - {error}", util::get_name(&repo_link));
                        }

                        tx_repo
                            .send(RepoMessage::Error(format!("{repo_link} (backup): {error}")))
                            .unwrap();
                    }
                }

                tx.clone().unwrap().send(Message::Finish).unwrap();
            }

            tx_repo.send(RepoMessage::Completed(true)).unwrap();
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
    pub success: bool,
    pub error: String,
    pub completed: bool,
}
