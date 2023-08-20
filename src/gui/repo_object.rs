use glib::Object;
use gtk::glib;
use gtk::subclass::prelude::*;
use serde::{Deserialize, Serialize};

mod imp;

glib::wrapper! {
    pub struct RepoObject(ObjectSubclass<imp::RepoObject>);
}

impl RepoObject {
    pub fn new(name: String, link: String, branch: String, status: String) -> Self {
        Object::builder()
            .property("name", name)
            .property("link", link)
            .property("branch", branch)
            .property("status", status)
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
            repo_data.status,
        )
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RepoData {
    pub name: String,
    pub link: String,
    pub branch: String,
    pub status: String,
}
