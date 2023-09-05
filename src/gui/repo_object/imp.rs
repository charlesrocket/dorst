use std::cell::RefCell;

use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::gui::RepoData;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::RepoObject)]
pub struct RepoObject {
    #[property(name = "name", get, set, type = String, member = name)]
    #[property(name = "link", get, set, type = String, member = link)]
    #[property(name = "branch", get, set, type = String, member = branch)]
    #[property(name = "progress", get, set, type = f64, member = progress)]
    #[property(name = "status", get, set, type = String, member = status)]
    #[property(name = "success", get, set, type = bool, member = success)]
    #[property(name = "error", get, set, type = String, member = error)]
    #[property(name = "completed", get, set, type = bool, member = completed)]
    pub data: RefCell<RepoData>,
}

#[glib::object_subclass]
impl ObjectSubclass for RepoObject {
    const NAME: &'static str = "DorstRepoObject";
    type Type = super::RepoObject;
}

#[glib::derived_properties]
impl ObjectImpl for RepoObject {}
