use gtk::glib::{self, Object};

mod imp;

glib::wrapper! {
    pub struct RepoBox(ObjectSubclass<imp::RepoBox>)
        @extends gtk::Box, gtk::Widget;
}

impl RepoBox {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

impl Default for RepoBox {
    fn default() -> Self {
        Self::new()
    }
}
