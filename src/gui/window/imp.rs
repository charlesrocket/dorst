use adw::{prelude::*, subclass::prelude::*, Banner, StyleManager, ToastOverlay};
use gtk::{
    gio, glib::subclass::InitializingObject, Button, CompositeTemplate, Entry, FileDialog,
    FilterListModel, ListBox, ProgressBar, Revealer, Stack, ToggleButton,
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

#[cfg(feature = "logs")]
use tracing::info;

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
    pub repos_list_count: Cell<u32>,
    pub repos: RefCell<Option<gio::ListStore>>,
    pub repos_filtered: RefCell<FilterListModel>,
    pub source_directory: RefCell<String>,
    pub backup_directory: RefCell<PathBuf>,
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
    #[template_child]
    pub stack_list: TemplateChild<Stack>,
    pub filter_option: RefCell<String>,
    pub backups_enabled: Cell<bool>,
    pub color_scheme: Arc<Mutex<String>>,
    pub style_manager: StyleManager,
    pub updated_list: Arc<Mutex<Vec<String>>>,
    pub errors_list: Arc<Mutex<Vec<String>>>,
    pub success_list: Arc<Mutex<Vec<String>>>,
    #[property(get, set)]
    pub completed: Cell<u32>,
    #[cfg(feature = "logs")]
    #[property(get, set)]
    pub logs: Cell<bool>,
    #[property(get, set)]
    pub task_limiter: Cell<bool>,
    pub thread_pool: Arc<Mutex<u64>>,
    pub active_threads: Arc<Mutex<u64>>,
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "DorstWindow";
    type Type = super::Window;
    type ParentType = adw::ApplicationWindow;

    fn new() -> Self {
        Self {
            button_start: TemplateChild::default(),
            button_source_dest: TemplateChild::default(),
            button_backup_dest: TemplateChild::default(),
            button_backup_state: TemplateChild::default(),
            repo_entry: TemplateChild::default(),
            repo_entry_empty: TemplateChild::default(),
            repos_list: TemplateChild::default(),
            repos_list_count: Cell::default(),
            repos: RefCell::default(),
            repos_filtered: RefCell::default(),
            source_directory: RefCell::new(String::new()),
            backup_directory: RefCell::new(PathBuf::new()),
            progress_bar: TemplateChild::default(),
            toast_overlay: TemplateChild::default(),
            banner: TemplateChild::default(),
            revealer_banner: TemplateChild::default(),
            revealer: TemplateChild::default(),
            stack: TemplateChild::default(),
            stack_list: TemplateChild::default(),
            filter_option: RefCell::new(String::from("All")),
            backups_enabled: Cell::new(false),
            color_scheme: Arc::default(),
            style_manager: StyleManager::default(),
            updated_list: Arc::default(),
            errors_list: Arc::default(),
            success_list: Arc::default(),
            completed: Cell::default(),
            #[cfg(feature = "logs")]
            logs: Cell::new(true),
            task_limiter: Cell::default(),
            thread_pool: Arc::new(Mutex::new(7)),
            active_threads: Arc::default(),
        }
    }

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
        klass.bind_template_callbacks();
        klass.install_action_async(
            "win.select-source-directory",
            None,
            |window, _action_name, _action_target| async move {
                let dialog = FileDialog::builder()
                    .title("Source directory")
                    .modal(true)
                    .build();

                if let Ok(folder) = dialog.select_folder_future(Some(&window)).await {
                    window.select_source_directory(&folder.path().unwrap());
                }
            },
        );

        klass.install_action_async(
            "win.select-backup-directory",
            None,
            |window, _action_name, _action_target| async move {
                let dialog = FileDialog::builder()
                    .title("Backup directory")
                    .modal(true)
                    .build();

                if let Ok(folder) = dialog.select_folder_future(Some(&window)).await {
                    window.select_backup_directory(&folder.path().unwrap());
                }
            },
        );
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
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

        obj.connect_completed_notify(|window| {
            let error_margin = f64::EPSILON;
            let total_repos = window.get_repo_data().len();
            let completed = f64::from(window.completed());
            let progress = completed / total_repos as f64;

            if (completed - total_repos as f64).abs() < error_margin {
                let updated_list_locked = window.imp().updated_list.lock().unwrap();
                let errors_list_locked = window.imp().errors_list.lock().unwrap();
                let errors_locked = errors_list_locked
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n");

                if !errors_locked.is_empty() {
                    window.imp().banner.set_title(&errors_locked);
                    window.imp().revealer_banner.set_reveal_child(true);
                    window.imp().banner.set_revealed(true);
                    window.show_message(&format!("Failures: {}", errors_list_locked.len()), 1);
                }

                if !updated_list_locked.is_empty() {
                    window.show_message(
                        &format!("Repositories with updates: {}", updated_list_locked.len()),
                        4,
                    );
                }

                window.imp().progress_bar.set_fraction(1.0);
                window.imp().revealer.set_reveal_child(false);
                window.imp().button_source_dest.remove_css_class("with_bar");
                window
                    .imp()
                    .button_backup_state
                    .remove_css_class("with_bar");
                window.controls_disabled(false);

                #[cfg(feature = "logs")]
                if window.logs() {
                    info!("Finished");
                }
            } else {
                window.imp().progress_bar.set_fraction(progress);
            }
        });
    }
}

#[gtk::template_callbacks]
impl Window {
    #[template_callback]
    fn toggle_backups(&self, button_backup_state: &ToggleButton) {
        if button_backup_state.is_active() {
            self.button_backup_dest.set_visible(true);
            self.backups_enabled.set(true);
        } else {
            self.button_backup_dest.set_visible(false);
            self.backups_enabled.set(false);
        };
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::window::tests::window;

    #[gtk::test]
    fn toggle_backups() {
        let window = window();

        window.imp().button_backup_state.set_active(true);
        assert!(window.imp().backups_enabled.get());

        window.imp().button_backup_state.set_active(false);
        assert!(!window.imp().backups_enabled.get());
    }
}
