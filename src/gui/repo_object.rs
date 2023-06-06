use glib::Object;
use gtk::glib;
use gtk::subclass::prelude::*;
use serde::{Deserialize, Serialize};

mod imp;

glib::wrapper! {
    pub struct RepoObject(ObjectSubclass<imp::RepoObject>);
}

impl RepoObject {
    pub fn new(link: String) -> Self {
        Object::builder().property("link", link).build()
    }

    pub fn repo_data(&self) -> RepoData {
        self.imp().data.borrow().clone()
    }

    pub fn from_repo_data(repo_data: RepoData) -> Self {
        Self::new(repo_data.link)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RepoData {
    pub link: String,
}
