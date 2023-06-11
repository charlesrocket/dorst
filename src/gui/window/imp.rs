use adw::{prelude::*, subclass::prelude::*};
use glib::signal::Inhibit;
use glib::subclass::InitializingObject;
use gtk::{gio, glib, CompositeTemplate, Entry, ListBox};
use serde_yaml::{Mapping, Sequence, Value};

use std::{cell::RefCell, fs::File, io::Write, path::PathBuf};

use crate::gui::window::RepoObject;
use crate::gui::RepoData;
use crate::util;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/org/hellbyte/dorst/window.ui")]
pub struct Window {
    #[template_child]
    pub repo_entry: TemplateChild<Entry>,
    #[template_child]
    pub repos_list: TemplateChild<ListBox>,
    pub repos: RefCell<Option<gio::ListStore>>,
    pub directory_output: RefCell<PathBuf>,
    pub directory_dialog: gtk::FileDialog,
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "DorstWindow";
    type Type = super::Window;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
        klass.install_action_async(
            "win.select-directory",
            None,
            |win, _action_name, _action_target| async move {
                let dialog = &win.imp().directory_dialog;
                if let Ok(folder) = dialog.select_folder_future(Some(&win)).await {
                    win.set_directory(&folder.path().unwrap());
                }
            },
        );
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();

        obj.setup_repos();
        obj.restore_data();
        obj.setup_callbacks();
        obj.setup_actions();
        #[cfg(debug_assertions)]
        obj.setup_debug();
    }
}

#[gtk::template_callbacks]
impl Window {}

impl WidgetImpl for Window {}

impl WindowImpl for Window {
    fn close_request(&self) -> Inhibit {
        let backup_data: Vec<RepoData> = self
            .obj()
            .repos()
            .snapshot()
            .iter()
            .filter_map(Cast::downcast_ref::<RepoObject>)
            .map(RepoObject::repo_data)
            .collect();

        let mut target_sequence = Sequence::new();
        for repo_data in backup_data {
            target_sequence.push(Value::String(repo_data.link));
        }

        let mut yaml_mapping = Mapping::new();
        yaml_mapping.insert(
            Value::String("targets".to_owned()),
            Value::Sequence(target_sequence),
        );

        let yaml_data = serde_yaml::to_string(&yaml_mapping).unwrap();
        let mut file = File::create(util::xdg_path().unwrap()).unwrap();
        file.write_all(yaml_data.as_bytes()).unwrap();
        self.parent_close_request()
    }
}

impl ApplicationWindowImpl for Window {}

impl AdwApplicationWindowImpl for Window {}