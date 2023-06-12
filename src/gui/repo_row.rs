use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::gui::repo_object::RepoObject;

mod imp;

glib::wrapper! {
    pub struct RepoRow(ObjectSubclass<imp::RepoRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for RepoRow {
    fn default() -> Self {
        Self::new()
    }
}

impl RepoRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, repo_object: &RepoObject) {
        let name_label = self.imp().name_label.get();
        let link_label = self.imp().link_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let name_label_binding = repo_object
            .bind_property("name", &name_label, "label")
            .sync_create()
            .build();

        bindings.push(name_label_binding);

        let link_label_binding = repo_object
            .bind_property("link", &link_label, "label")
            .sync_create()
            .build();

        bindings.push(link_label_binding);
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
