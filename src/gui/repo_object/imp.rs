use std::cell::RefCell;

use glib::{ParamSpec, Properties, Value};
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

impl ObjectImpl for RepoObject {
    fn properties() -> &'static [ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, id: usize, value: &Value, pspec: &ParamSpec) {
        self.derived_set_property(id, value, pspec)
    }

    fn property(&self, id: usize, pspec: &ParamSpec) -> Value {
        self.derived_property(id, pspec)
    }
}
