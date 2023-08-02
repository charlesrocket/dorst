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
    pub data: RefCell<RepoData>,
}

#[glib::object_subclass]
impl ObjectSubclass for RepoObject {
    const NAME: &'static str = "DorstRepoObject";
    type Type = super::RepoObject;
}

#[glib::derived_properties]
impl ObjectImpl for RepoObject {}
