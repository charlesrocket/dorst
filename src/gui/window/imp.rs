use adw::{prelude::*, subclass::prelude::*, Banner, StyleManager, ToastOverlay};
use gtk::{
    gio,
    glib::{subclass::InitializingObject, ParamSpec},
    Button, CompositeTemplate, Entry, ListBox, ProgressBar, Revealer, Stack, ToggleButton,
};

use glib::Properties;
use serde_yaml::{Mapping, Sequence, Value};
use std::{
    cell::{Cell, RefCell},
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::gui::window::RepoObject;
use crate::gui::RepoData;
use crate::util;

#[derive(CompositeTemplate, Properties)]
#[properties(wrapper_type = super::Window)]
#[template(resource = "/org/hellbyte/dorst/window.ui")]
pub struct Window {
    #[template_child]
    pub button_start: TemplateChild<Button>,
    #[template_child]
    pub button_source_dest: TemplateChild<Button>,
    #[template_child]
    pub button_backup_dest: TemplateChild<Button>,
    #[template_child]
    pub button_backup_state: TemplateChild<ToggleButton>,
    #[template_child]
    pub repo_entry: TemplateChild<Entry>,
    #[template_child]
    pub repo_entry_empty: TemplateChild<Entry>,
    #[template_child]
    pub repos_list: TemplateChild<ListBox>,
    pub repos: RefCell<Option<gio::ListStore>>,
    pub source_directory: RefCell<String>,
    pub backup_directory: RefCell<PathBuf>,
    pub directory_dialog: gtk::FileDialog,
    #[template_child]
    pub progress_bar: TemplateChild<ProgressBar>,
    #[template_child]
    pub toast_overlay: TemplateChild<ToastOverlay>,
    #[template_child]
    pub banner: TemplateChild<Banner>,
    #[template_child]
    pub revealer_banner: TemplateChild<Revealer>,
    #[template_child]
    pub revealer: TemplateChild<Revealer>,
    #[template_child]
    pub stack: TemplateChild<Stack>,
    pub filter_option: RefCell<String>,
    pub backups_enabled: RefCell<bool>,
    pub color_scheme: Arc<Mutex<String>>,
    pub style_manager: StyleManager,
    pub errors_list: Arc<Mutex<Vec<String>>>,
    pub success_list: Arc<Mutex<Vec<String>>>,
    #[property(name = "limiter", get, set, type = bool)]
    pub task_limiter: Cell<bool>,
    pub thread_pool: Arc<Mutex<u64>>,
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "DorstWindow";
    type Type = super::Window;
    type ParentType = adw::ApplicationWindow;

    fn new() -> Self {
        let directory_dialog = gtk::FileDialog::builder()
            .title("Select destination")
            .modal(true)
            .build();

        Self {
            button_start: TemplateChild::default(),
            button_source_dest: TemplateChild::default(),
            button_backup_dest: TemplateChild::default(),
            button_backup_state: TemplateChild::default(),
            repo_entry: TemplateChild::default(),
            repo_entry_empty: TemplateChild::default(),
            repos_list: TemplateChild::default(),
            repos: RefCell::default(),
            source_directory: RefCell::new(String::new()),
            backup_directory: RefCell::new(PathBuf::new()),
            directory_dialog,
            progress_bar: TemplateChild::default(),
            toast_overlay: TemplateChild::default(),
            banner: TemplateChild::default(),
            revealer_banner: TemplateChild::default(),
            revealer: TemplateChild::default(),
            stack: TemplateChild::default(),
            filter_option: RefCell::new(String::from("All")),
            backups_enabled: RefCell::new(false),
            color_scheme: Arc::default(),
            style_manager: StyleManager::default(),
            errors_list: Arc::default(),
            success_list: Arc::default(),
            task_limiter: Cell::default(),
            thread_pool: Arc::new(Mutex::new(7)),
        }
    }

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
        klass.install_action_async(
            "win.select-source-directory",
            None,
            |win, _action_name, _action_target| async move {
                let dialog = &win.imp().directory_dialog;

                if let Ok(folder) = dialog.select_folder_future(Some(&win)).await {
                    win.set_source_directory(&folder.path().unwrap());
                    win.show_message(
                        &format!(
                            "Source directory: {}",
                            folder.path().unwrap().to_str().unwrap()
                        ),
                        3,
                    );

                    win.imp()
                        .button_source_dest
                        .remove_css_class("suggested-action");
                }
            },
        );

        klass.install_action_async(
            "win.select-backup-directory",
            None,
            |win, _action_name, _action_target| async move {
                let dialog = &win.imp().directory_dialog;
                if let Ok(folder) = dialog.select_folder_future(Some(&win)).await {
                    win.set_backup_directory(&folder.path().unwrap());
                    win.show_message(
                        &format!(
                            "Backup directory: {}",
                            folder.path().unwrap().to_str().unwrap()
                        ),
                        3,
                    );

                    win.imp()
                        .button_backup_dest
                        .remove_css_class("suggested-action");
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

        obj.setup_actions();
        obj.setup_repos();
        obj.load_settings();
        obj.setup_theme();
        obj.setup_callbacks();
        obj.restore_data();
    }

    fn properties() -> &'static [ParamSpec] {
        Self::derived_properties()
    }
    fn set_property(&self, id: usize, value: &glib::Value, pspec: &ParamSpec) {
        self.derived_set_property(id, value, pspec);
    }
    fn property(&self, id: usize, pspec: &ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }
}

#[gtk::template_callbacks]
impl Window {}

impl WidgetImpl for Window {}

impl WindowImpl for Window {
    fn close_request(&self) -> glib::Propagation {
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
            Value::String("source_directory".to_owned()),
            Value::String(self.source_directory.borrow().to_string()),
        );

        yaml_mapping.insert(
            Value::String("targets".to_owned()),
            Value::Sequence(target_sequence),
        );

        let yaml_data = serde_yaml::to_string(&yaml_mapping).unwrap();
        let mut file = File::create(util::xdg_path().unwrap()).unwrap();
        file.write_all(yaml_data.as_bytes()).unwrap();
        self.obj().save_settings();
        self.parent_close_request()
    }
}

impl ApplicationWindowImpl for Window {}

impl AdwApplicationWindowImpl for Window {}
