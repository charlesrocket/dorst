use adw::{prelude::*, subclass::prelude::*, AboutWindow, ColorScheme};
use anyhow::Result;
use glib::{clone, ControlFlow, KeyFile, MainContext, Object, Priority, Sender};
use gtk::{
    gio::{self, ListStore, SimpleAction},
    glib,
    pango::EllipsizeMode,
    Align, Box, Button, CustomFilter, EventSequenceState, FilterListModel, GestureClick, Label,
    License, ListBoxRow, NoSelection, Orientation, Popover, ProgressBar, Revealer,
    RevealerTransitionType,
};

use std::{
    cell::RefMut,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time,
};

mod imp;

use crate::{
    git,
    gui::{repo_object::RepoObject, window, RepoData},
    util,
};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

pub enum Message {
    Reset,
    Progress(f64),
    Clone,
    Fetch,
    Deltas,
    Finish,
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        Object::builder::<Window>()
            .property("application", app)
            .build()
    }

    fn setup_theme(&self) {
        #[cfg(debug_assertions)]
        self.add_css_class("devel");
    }

    fn setup_actions(&self) {
        let action_about = SimpleAction::new("about", None);
        action_about.connect_activate(clone!(@weak self as window => move |_, _| {
            window.show_about_dialog();
        }));

        let action_process_targets = SimpleAction::new("process-targets", None);
        action_process_targets.connect_activate(clone!(@weak self as window => move |_, _| {
            window.process_targets();
        }));

        let action_close = SimpleAction::new("close", None);
        action_close.connect_activate(clone!(@weak self as window => move |_, _| {
            window.close();
        }));

        let action_color_scheme = SimpleAction::new_stateful(
            "color-scheme",
            Some(&String::static_variant_type()),
            &"Default".to_variant(),
        );

        action_color_scheme.connect_activate(
            clone!(@weak self as window => move |action, parameter| {
                let parameter = parameter
                    .unwrap()
                    .get::<String>()
                    .unwrap();

                let value = match parameter.as_str() {
                    "Force Light" => {
                        window
                            .imp()
                            .style_manager
                            .set_color_scheme(ColorScheme::ForceLight);
                        "Force Light"
                    }
                    "Force Dark" => {
                        window
                            .imp()
                            .style_manager
                            .set_color_scheme(ColorScheme::ForceDark);
                        "Force Dark"
                    }
                    "Prefer Light" => {
                        window
                            .imp()
                            .style_manager
                            .set_color_scheme(ColorScheme::ForceLight);
                        "Prefer Light"
                    }
                    "Prefer Dark" => {
                        window
                            .imp()
                            .style_manager
                            .set_color_scheme(ColorScheme::ForceDark);
                        "Prefer Dark"
                    }
                    _ => {
                        window
                            .imp()
                            .style_manager
                            .set_color_scheme(ColorScheme::Default);
                        "Default"
                    }
                };

                *window.imp().color_scheme.lock().unwrap() = String::from(value);
                action.set_state(&value.to_variant());
            }),
        );

        let action_task_limiter = gio::PropertyAction::new("task-limiter", self, "task_limiter");

        self.add_action(&action_about);
        self.add_action(&action_process_targets);
        self.add_action(&action_close);
        self.add_action(&action_color_scheme);
        self.add_action(&action_task_limiter);
    }

    fn setup_callbacks(&self) {
        self.imp()
            .repo_entry_empty
            .connect_activate(clone!(@weak self as window => move |_| {
                window.new_repo(false);
            }));

        self.imp().repo_entry_empty.connect_icon_release(
            clone!(@weak self as window => move |_,_| {
                window.new_repo(false);
            }),
        );

        self.imp()
            .repo_entry
            .connect_activate(clone!(@weak self as window => move |_| {
                window.new_repo(true);
            }));

        self.imp()
            .repo_entry
            .connect_icon_release(clone!(@weak self as window => move |_,_| {
                window.new_repo(true);
            }));

        self.imp()
            .button_backup_state
            .connect_toggled(clone!(@weak self as window => move |_| {
                let mut state = window.imp().backups_enabled.borrow_mut();
                *state = window.imp().button_backup_state.is_active();
                window.imp().button_backup_dest.set_visible(*state);
            }));
    }

    fn setup_repos(&self) {
        let model = gio::ListStore::new::<RepoObject>();
        self.imp().repos.replace(Some(model));

        let filter_model = FilterListModel::new(Some(self.repos()), self.filter());
        let selection_model = NoSelection::new(Some(filter_model.clone()));

        self.imp().repos_list.bind_model(
            Some(&selection_model),
            clone!(@weak self as window => @default-panic, move |obj| {
                let repo_object = obj.downcast_ref().expect("The object should be of type `RepoObject`.");
                let row = window.create_repo_row(repo_object);
                row.upcast()
            }),
        );

        self.set_repo_list_visible(&self.repos());
        self.repos()
            .connect_items_changed(clone!(@weak self as window => move |repos, _, _, _| {
                window.set_repo_list_visible(repos);
                window.set_repo_list_stack();
            }));

        let action_filter = SimpleAction::new_stateful(
            "filter",
            Some(&String::static_variant_type()),
            &"All".to_variant(),
        );

        action_filter.connect_activate(
            clone!(@weak self as window => move |action, parameter| {
                let parameter = parameter
                    .unwrap()
                    .get::<String>()
                    .unwrap();

                *window.imp().filter_option.borrow_mut() = String::from(&parameter);

                filter_model.set_filter(window.filter().as_ref());

                if window.imp().errors_list.lock().unwrap().len() > 0 || window.imp().success_list.lock().unwrap().len() > 0 {
                    window.update_rows();
                }

                window.set_repo_list_stack();
                action.set_state(&parameter.to_variant());
            }),
        );

        self.add_action(&action_filter);
    }

    fn controls_disabled(&self, state: bool) {
        if state {
            self.imp().button_start.set_sensitive(false);
            self.imp().button_source_dest.set_sensitive(false);
            self.imp().button_backup_dest.set_sensitive(false);
            self.imp().button_backup_state.set_sensitive(false);
            self.imp().repo_entry.set_sensitive(false);
        } else {
            self.imp().button_start.set_sensitive(true);
            self.imp().button_source_dest.set_sensitive(true);
            self.imp().button_backup_dest.set_sensitive(true);
            self.imp().button_backup_state.set_sensitive(true);
            self.imp().repo_entry.set_sensitive(true);
        }
    }

    fn process_targets(&self) {
        self.controls_disabled(true);
        self.imp().banner.set_revealed(false);
        self.imp().revealer_banner.set_reveal_child(false);
        self.imp().errors_list.lock().unwrap().clear();
        self.imp().success_list.lock().unwrap().clear();
        self.imp().button_source_dest.add_css_class("with_bar");
        self.imp().button_backup_state.add_css_class("with_bar");
        self.imp().progress_bar.set_fraction(0.0);
        self.imp().revealer.set_reveal_child(true);

        let mut active_task = false;
        let repos = self.repos();
        let total_repos = self.get_repo_data().len();
        let completed_repos = Arc::new(Mutex::new(0));
        let completed_repos_clone = completed_repos.clone();
        let dest_clone = self.get_dest_clone();
        let dest_backup = self.get_dest_backup();
        let backups_enabled = *self.imp().backups_enabled.borrow();

        let thread_pool = Arc::new(Mutex::new(0));

        glib::idle_add_local(
            clone!(@weak self as window => @default-return ControlFlow::Continue, move || {
                let completed = *completed_repos_clone.lock().unwrap() as f64;
                let progress = completed / total_repos as f64;

                window.update_rows();

                if completed == total_repos as f64 {
                    let errors_list_locked = window.imp().errors_list.lock().unwrap();
                    let errors_locked = errors_list_locked.iter()
                                                          .map(std::string::ToString::to_string)
                                                          .collect::<Vec<_>>()
                        .join("\n");

                    if !errors_locked.is_empty() {
                        window.imp().banner.set_title(&errors_locked);
                        window.imp().revealer_banner.set_reveal_child(true);
                        window.imp().banner.set_revealed(true);
                        window.show_message(&format!("Failures: {}", errors_list_locked.len()), 1);
                    }

                    window.imp().progress_bar.set_fraction(1.0);
                    window.imp().revealer.set_reveal_child(false);
                    window.imp().button_source_dest.remove_css_class("with_bar");
                    window.imp().button_backup_state.remove_css_class("with_bar");
                    window.controls_disabled(false);
                    ControlFlow::Break
                } else {
                    window.imp().progress_bar.set_fraction(progress);
                    ControlFlow::Continue
                }
            }),
        );

        for i in 0..repos.n_items() {
            let completed_repos_clone = completed_repos.clone();
            let obj = repos.item(i).unwrap();
            let repo = obj.downcast_ref::<RepoObject>().unwrap();
            if let Some(row) = self.imp().repos_list.row_at_index(i as i32) {
                active_task = true;
                let repo_link = repo.link();
                let thread_pool_clone = thread_pool.clone();
                let tx = window::Window::set_row_channel(&row);
                let destination_clone = format!(
                    "{}/{}",
                    &dest_clone.clone().display().to_string(),
                    util::get_name(&repo_link)
                );
                let destination_backup = format!(
                    "{}/{}.dorst",
                    &dest_backup.clone().display().to_string(),
                    util::get_name(&repo_link)
                );

                let errors_clone = self.imp().errors_list.clone();
                let success_clone = self.imp().success_list.clone();
                let revealer = window::Window::get_row_revealer(&row);
                let progress_bar = revealer.child().unwrap().downcast::<ProgressBar>().unwrap();

                progress_bar.set_fraction(0.0);
                revealer.set_reveal_child(true);

                if self.task_limiter() {
                    while *thread_pool_clone.lock().unwrap()
                        > *self.imp().thread_pool.lock().unwrap()
                    {
                        self.update_rows();
                        let wait_loop = glib::MainLoop::new(None, false);

                        glib::timeout_add(
                            time::Duration::from_millis(50),
                            glib::clone!(@strong wait_loop => move || {
                                wait_loop.quit();
                                ControlFlow::Break
                            }),
                        );

                        wait_loop.run();
                    }
                }

                *thread_pool_clone.lock().unwrap() += 1;

                gio::spawn_blocking(move || {
                    match window::Window::process_repo(
                        &destination_clone,
                        &destination_backup,
                        &repo_link,
                        backups_enabled,
                        #[cfg(feature = "gui")]
                        &Some(tx.clone()),
                    ) {
                        Ok(()) => {
                            success_clone.lock().unwrap().push(repo_link);
                        }
                        Err(error) => errors_clone
                            .lock()
                            .unwrap()
                            .push(format!("{repo_link}: {error}")),
                    }

                    *completed_repos_clone.lock().unwrap() += 1;
                    *thread_pool_clone.lock().unwrap() -= 1;
                });
            } else {
                *completed_repos.lock().unwrap() += 1;
            }
        }

        if !active_task {
            self.controls_disabled(false);
        }
    }

    fn process_repo(
        destination_clone: &str,
        destination_backup: &str,
        repo_link: &str,
        mirror: bool,
        #[cfg(feature = "gui")] tx: &Option<Sender<Message>>,
    ) -> Result<()> {
        git::process_target(
            destination_clone,
            repo_link,
            false,
            #[cfg(feature = "cli")]
            None,
            #[cfg(feature = "gui")]
            tx,
            #[cfg(feature = "cli")]
            None,
        )?;

        let _ = tx.clone().unwrap().send(Message::Finish);

        if mirror {
            let _ = tx.clone().unwrap().send(Message::Reset);

            git::process_target(
                destination_backup,
                repo_link,
                true,
                #[cfg(feature = "cli")]
                None,
                #[cfg(feature = "gui")]
                tx,
                #[cfg(feature = "cli")]
                None,
            )?;

            let _ = tx.clone().unwrap().send(Message::Finish);
        }

        Ok(())
    }

    fn update_rows(&self) {
        let repos = self.repos();

        for i in 0..repos.n_items() {
            if let Some(obj) = repos.item(i) {
                if let Some(repo_object) = obj.downcast_ref::<RepoObject>() {
                    let link = repo_object.repo_data().link.clone();
                    if self.imp().success_list.lock().unwrap().contains(&link) {
                        if let Some(row) = self.imp().repos_list.row_at_index(i as i32) {
                            row.remove_css_class("error");
                            row.add_css_class("success");
                        }
                    } else if self
                        .imp()
                        .errors_list
                        .lock()
                        .unwrap()
                        .iter()
                        .any(|x| x.contains(&link))
                    {
                        if let Some(row) = self.imp().repos_list.row_at_index(i as i32) {
                            row.remove_css_class("success");
                            row.add_css_class("error");
                        }
                    } else if let Some(row) = self.imp().repos_list.row_at_index(i as i32) {
                        row.remove_css_class("success");
                        row.remove_css_class("error");
                    }
                }
            }
        }
    }

    fn repos(&self) -> ListStore {
        self.imp()
            .repos
            .borrow()
            .clone()
            .expect("Could not get current repositories.")
    }

    fn get_dest_clone(&self) -> PathBuf {
        let dest = self.imp().source_directory.borrow();
        let path = dest.to_string();

        PathBuf::from(util::expand_path(&path))
    }

    fn get_dest_backup(&self) -> RefMut<PathBuf> {
        self.imp().backup_directory.borrow_mut()
    }

    fn get_repo_data(&self) -> Vec<RepoData> {
        self.repos()
            .snapshot()
            .iter()
            .filter_map(Cast::downcast_ref::<RepoObject>)
            .map(RepoObject::repo_data)
            .collect()
    }

    fn set_repo_list_visible(&self, repos: &ListStore) {
        let repos_list_activated = repos.n_items() > 0;
        self.imp().repos_list.set_visible(repos_list_activated);

        if repos_list_activated {
            self.imp().stack.set_visible_child_name("main");
        } else {
            self.imp().stack.set_visible_child_name("empty");
        }
    }

    fn set_repo_list_stack(&self) {
        if self.imp().repos_list.row_at_index(0).is_none() {
            self.imp().stack_list.set_visible_child_name("empty");
        } else {
            self.imp().stack_list.set_visible_child_name("main");
        }
    }

    fn set_row_channel(row: &ListBoxRow) -> glib::Sender<Message> {
        let (tx, rx) = MainContext::channel(Priority::DEFAULT);
        let revealer = window::Window::get_row_revealer(row);
        let progress_bar = revealer.child().unwrap().downcast::<ProgressBar>().unwrap();

        rx.attach(None, move |x| match x {
            Message::Reset => {
                progress_bar.set_fraction(0.0);
                revealer.set_reveal_child(true);
                ControlFlow::Continue
            }
            Message::Progress(value) => {
                if value.is_nan() {
                    progress_bar.set_fraction(1.0);
                } else {
                    progress_bar.set_fraction(value);
                }

                ControlFlow::Continue
            }
            Message::Clone => {
                progress_bar.add_css_class("clone");
                progress_bar.remove_css_class("deltas");
                progress_bar.remove_css_class("fetch");
                ControlFlow::Continue
            }
            Message::Fetch => {
                progress_bar.add_css_class("fetch");
                progress_bar.remove_css_class("clone");
                progress_bar.remove_css_class("deltas");
                ControlFlow::Continue
            }
            Message::Deltas => {
                progress_bar.add_css_class("deltas");
                progress_bar.remove_css_class("clone");
                progress_bar.remove_css_class("fetch");
                ControlFlow::Continue
            }
            Message::Finish => {
                progress_bar.set_fraction(1.0);
                revealer.set_reveal_child(false);
                ControlFlow::Continue
            }
        });

        tx
    }

    fn get_row_revealer(row: &ListBoxRow) -> Revealer {
        row.child()
            .unwrap()
            .downcast::<Box>()
            .unwrap()
            .last_child()
            .unwrap()
            .downcast::<Box>()
            .unwrap()
            .last_child()
            .unwrap()
            .downcast::<Revealer>()
            .unwrap()
    }

    fn create_repo_row(&self, repo_object: &RepoObject) -> ListBoxRow {
        let name = Label::builder()
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .build();

        let link = Label::builder()
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .margin_top(4)
            .build();

        let pb = ProgressBar::builder()
            .halign(Align::Start)
            .pulse_step(1.0)
            .hexpand(true)
            .halign(Align::Fill)
            .build();

        let pb_box = Box::builder().orientation(Orientation::Horizontal).build();

        let popover_box = Box::builder().hexpand(true).build();
        let popover = Popover::builder()
            .child(&popover_box)
            .autohide(true)
            .has_arrow(true)
            .build();

        let remove_button = Button::builder().label("Remove").build();

        let repo_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Fill)
            .valign(Align::Center)
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .build();

        let revealer = Revealer::builder()
            .margin_top(4)
            .transition_type(RevealerTransitionType::Crossfade)
            .transition_duration(542)
            .child(&pb)
            .build();

        let gesture = GestureClick::new();

        gesture.connect_released(clone!(@weak popover => move |gesture, _, _, _,| {
            gesture.set_state(EventSequenceState::Claimed);
            popover.popup();
        }));

        repo_object
            .bind_property("name", &name, "label")
            .sync_create()
            .build();

        repo_object
            .bind_property("link", &link, "label")
            .sync_create()
            .build();

        if &link.label() == "INVALID" {
            name.add_css_class("error");
        }

        remove_button.add_css_class("destructive-action");
        name.add_css_class("heading");
        link.add_css_class("body");
        link.add_css_class("caption");
        link.add_css_class("dim-label");
        pb.add_css_class("osd");
        pb.add_css_class("row-progress");
        pb_box.append(&revealer);
        popover_box.append(&remove_button);
        popover.set_parent(&repo_box);
        repo_box.add_controller(gesture);
        repo_box.append(&name);
        repo_box.append(&link);
        repo_box.append(&pb_box);

        remove_button.connect_clicked(clone!(@weak self as window => move |_| {
            window.remove_repo(&link.label());
            window.show_message(&format!("Removed: {}", name.label()), 3);
            popover.popdown();
        }));

        ListBoxRow::builder().child(&repo_box).build()
    }

    fn new_repo(&self, main_view: bool) {
        let buffer = if main_view {
            self.imp().repo_entry.buffer()
        } else {
            self.imp().repo_entry_empty.buffer()
        };

        let mut content = buffer.text().to_string();

        buffer.set_text("");

        if content.is_empty() {
            return;
        }

        if content.ends_with('/') {
            content.pop();
            if content.is_empty() {
                return;
            };
        }

        let name = util::get_name(&content).to_owned();
        let repo = RepoObject::new(name, content);
        self.repos().append(&repo);
    }

    fn remove_repo(&self, repo: &str) {
        let repos = self.repos();
        let mut position = 0;
        while let Some(item) = repos.item(position) {
            let repo_object = item.downcast_ref::<RepoObject>().unwrap();

            if repo_object.link() == repo {
                repos.remove(position);
            } else {
                position += 1;
            }
        }
    }

    fn set_source_directory(&self, directory: &Path) {
        let mut source_dir = self.imp().source_directory.borrow_mut();
        *source_dir = directory
            .to_path_buf()
            .into_os_string()
            .into_string()
            .unwrap();
    }

    fn set_backup_directory(&self, directory: &PathBuf) {
        let mut dir = self.imp().backup_directory.borrow_mut();
        dir.clear();
        dir.push(directory);
    }

    fn set_thread_pool_limit(&self, value: u64) {
        *self.imp().thread_pool.lock().unwrap() = value;
    }

    fn restore_data(&self) {
        #[cfg(not(test))]
        let conf_file = util::xdg_path().unwrap();
        #[cfg(test)]
        let conf_file = PathBuf::from("/tmp/dorst_test_conf.yaml");

        if let Ok(file) = fs::File::open(conf_file) {
            let config: serde_yaml::Value = serde_yaml::from_reader(file).unwrap();

            if let Some(source_directory) = config["source_directory"].as_str() {
                *self.imp().source_directory.borrow_mut() = String::from(source_directory);
                self.imp()
                    .button_source_dest
                    .remove_css_class("suggested-action");
            }

            if let Some(targets) = config["targets"].as_sequence() {
                let repo_objects: Vec<RepoObject> = targets
                    .iter()
                    .filter_map(|target| {
                        target.as_str().map(|link| {
                            let mut link_string = String::from(link);
                            if link_string.ends_with('/') {
                                link_string.pop();
                            }

                            if link_string.is_empty() {
                                link_string.push_str("INVALID");
                            }

                            RepoData {
                                name: util::get_name(&link_string).to_owned(),
                                link: link_string,
                            }
                        })
                    })
                    .map(RepoObject::from_repo_data)
                    .collect();

                self.repos().extend_from_slice(&repo_objects);
            }
        }
    }

    fn filter(&self) -> Option<CustomFilter> {
        let filter_state = &self.imp().filter_option.borrow();
        let filter_ssh = CustomFilter::new(|obj| {
            let repo_object = obj
                .downcast_ref::<RepoObject>()
                .expect("The object needs to be of type `RepoObject`.");

            repo_object.repo_data().link.contains('@')
        });

        let filter_https = CustomFilter::new(|obj| {
            let repo_object = obj
                .downcast_ref::<RepoObject>()
                .expect("The object needs to be of type `RepoObject`.");

            repo_object.repo_data().link.contains("https://")
        });

        match filter_state.as_str() {
            "All" => None,
            "SSH" => Some(filter_ssh),
            "HTTPS" => Some(filter_https),
            _ => unreachable!(),
        }
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    pub fn show_message(&self, message: &str, timeout: u32) {
        let toast = adw::Toast::new(message);
        toast.set_timeout(timeout);
        self.add_toast(toast);
    }

    fn show_about_dialog(&self) {
        let about_window = AboutWindow::builder()
            .application_name("DØRST")
            .version(util::version_string())
            .license_type(License::MitX11)
            .support_url("https://github.com/charlesrocket/dorst/discussions")
            .issue_url("https://github.com/charlesrocket/dorst/issues")
            .website(env!("CARGO_PKG_REPOSITORY"))
            .comments(env!("CARGO_PKG_DESCRIPTION"))
            .transient_for(self)
            .build();

        about_window.add_link(
            "Release Notes",
            "https://github.com/charlesrocket/dorst/blob/trunk/CHANGELOG.md",
        );

        about_window.present();
    }

    fn save_settings(&self) {
        #[cfg(not(test))]
        let cache_dir = glib::user_cache_dir();
        #[cfg(test)]
        let cache_dir = PathBuf::from("/tmp");

        let keyfile = KeyFile::new();
        let size = self.default_size();
        let dest = self.imp().backup_directory.borrow();
        let filter_option = &self.imp().filter_option.borrow();
        let backups_enabled = *self.imp().backups_enabled.borrow();
        let threads = *self.imp().thread_pool.lock().unwrap();
        let task_limiter = self.task_limiter();
        let color_scheme = self.imp().color_scheme.lock().unwrap();

        keyfile.set_int64("window", "width", size.0.into());
        keyfile.set_int64("window", "height", size.1.into());
        keyfile.set_string("window", "theme", &color_scheme);
        keyfile.set_string("window", "filter", filter_option);
        keyfile.set_string("backup", "destination", dest.to_str().unwrap());
        keyfile.set_boolean("backup", "enabled", backups_enabled);
        keyfile.set_uint64("core", "threads", threads);
        keyfile.set_boolean("core", "task-limiter", task_limiter);

        let settings_path = cache_dir.join("dorst");
        std::fs::create_dir_all(&settings_path).expect("Failed to create settings path");

        let settings = settings_path.join("gui.ini");

        keyfile
            .save_to_file(settings)
            .expect("Failed to save settings");
    }

    fn load_settings(&self) {
        #[cfg(not(test))]
        let cache_dir = glib::user_cache_dir();
        #[cfg(test)]
        let cache_dir = PathBuf::from("/tmp");

        let keyfile = KeyFile::new();
        let settings_path = cache_dir.join("dorst");
        let settings = settings_path.join("gui.ini");
        let mut backups_enabled = self.imp().backups_enabled.borrow_mut();

        if settings.exists() {
            if keyfile
                .load_from_file(settings, glib::KeyFileFlags::NONE)
                .is_err()
            {
                let error_dialog = gtk::AlertDialog::builder()
                    .modal(true)
                    .detail("Failed to load settings")
                    .build();

                error_dialog.show(Some(self));
            }

            if let (Ok(width), Ok(height)) = (
                keyfile.int64("window", "width"),
                keyfile.int64("window", "height"),
            ) {
                self.set_default_size(width.try_into().unwrap(), height.try_into().unwrap());
            }

            if let Ok(color_scheme) = keyfile.string("window", "theme") {
                let variant = color_scheme.to_variant();

                self.imp()
                    .stack
                    .activate_action("win.color-scheme", Some(&variant))
                    .unwrap();
            }

            if let Ok(filter) = keyfile.string("window", "filter") {
                let variant = filter.to_variant();

                self.imp()
                    .stack
                    .activate_action("win.filter", Some(&variant))
                    .unwrap();
            }

            if let Ok(dest) = keyfile.string("backup", "destination") {
                if !dest.is_empty() {
                    self.set_backup_directory(&PathBuf::from(dest.as_str()));
                    self.imp()
                        .button_backup_dest
                        .remove_css_class("suggested-action");
                }
            }

            if let Ok(backup_state) = keyfile.boolean("backup", "enabled") {
                *backups_enabled = backup_state;

                self.imp().button_backup_dest.set_visible(backup_state);

                if backup_state {
                    self.imp().button_backup_state.set_active(true);
                }
            }

            if let Ok(threads) = keyfile.uint64("core", "threads") {
                self.set_thread_pool_limit(threads);
            }

            if let Ok(task_limiter) = keyfile.boolean("core", "task-limiter") {
                self.set_task_limiter(task_limiter);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::{helper, wait_ui};
    use std::{
        fs::{remove_dir_all, remove_file},
        io::Write,
        path::Path,
    };

    pub fn window() -> Window {
        gio::resources_register_include!("dorst.gresource").expect("Failed to register resources.");
        Object::builder::<Window>().build()
    }

    fn entry_buffer_from_str(string: &str) -> gtk::EntryBuffer {
        gtk::EntryBuffer::builder().text(string).build()
    }

    #[gtk::test]
    fn color_scheme() {
        let window = window();
        let default_scheme = "Default";

        window
            .imp()
            .stack
            .activate_action("win.color-scheme", Some(&default_scheme.to_variant()))
            .unwrap();

        assert!(*window.imp().color_scheme.lock().unwrap() == default_scheme);

        let invalid_scheme = "foo";

        window
            .imp()
            .stack
            .activate_action("win.color-scheme", Some(&invalid_scheme.to_variant()))
            .unwrap();

        assert!(*window.imp().color_scheme.lock().unwrap() == default_scheme);

        let force_light_scheme = "Force Light";

        window
            .imp()
            .stack
            .activate_action("win.color-scheme", Some(&force_light_scheme.to_variant()))
            .unwrap();

        assert!(*window.imp().color_scheme.lock().unwrap() == force_light_scheme);

        let force_dark_scheme = "Force Dark";

        window
            .imp()
            .stack
            .activate_action("win.color-scheme", Some(&force_dark_scheme.to_variant()))
            .unwrap();

        assert!(*window.imp().color_scheme.lock().unwrap() == force_dark_scheme);

        let prefer_light_scheme = "Prefer Light";

        window
            .imp()
            .stack
            .activate_action("win.color-scheme", Some(&prefer_light_scheme.to_variant()))
            .unwrap();

        assert!(*window.imp().color_scheme.lock().unwrap() == prefer_light_scheme);

        let prefer_dark_scheme = "Prefer Dark";

        window
            .imp()
            .stack
            .activate_action("win.color-scheme", Some(&prefer_dark_scheme.to_variant()))
            .unwrap();

        assert!(*window.imp().color_scheme.lock().unwrap() == prefer_dark_scheme);
    }

    #[gtk::test]
    fn config() {
        if Path::new("/tmp/dorst_test_conf.yaml").exists() {
            remove_file("/tmp/dorst_test_conf.yaml").unwrap();
        }

        let mut config = tempfile::Builder::new().tempfile_in("/tmp").unwrap();

        config.write_all(b"\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x66\x6f\x6f\x62\x61\x72\x2f\x0a\x20\x20\x2d\x20\x2f").unwrap();
        config.persist("/tmp/dorst_test_conf.yaml").unwrap();

        let window = window();

        assert!(window.imp().stack.visible_child_name() == Some("main".into()));
        assert!(window.get_repo_data().len() == 2);

        window.imp().close_request();

        assert!(Path::new("/tmp/dorst_test_conf.yaml").exists());
        remove_file("/tmp/dorst_test_conf.yaml").unwrap();
    }

    #[gtk::test]
    fn settings() {
        fs::create_dir_all("/tmp/dorst").unwrap();
        let mut settings = tempfile::Builder::new().tempfile_in("/tmp/dorst").unwrap();

        settings.write_all(b"\x5b\x77\x69\x6e\x64\x6f\x77\x5d\x0a\x77\x69\x64\x74\x68\x3d\x34\x32\x33\x0a\x68\x65\x69\x67\x68\x74\x3d\x36\x30\x30\x0a\x74\x68\x65\x6d\x65\x3d\x44\x65\x66\x61\x75\x6c\x74\x0a\x66\x69\x6c\x74\x65\x72\x3d\x41\x6c\x6c\x0a\x0a\x5b\x62\x61\x63\x6b\x75\x70\x5d\x0a\x64\x65\x73\x74\x69\x6e\x61\x74\x69\x6f\x6e\x3d\x74\x65\x73\x74\x2d\x67\x75\x69\x0a\x65\x6e\x61\x62\x6c\x65\x64\x3d\x74\x72\x75\x65\x0a\x0a\x5b\x63\x6f\x72\x65\x5d\x0a\x74\x68\x72\x65\x61\x64\x73\x3d\x31\x0a\x74\x61\x73\x6b\x2d\x6c\x69\x6d\x69\x74\x65\x72\x3d\x74\x72\x75\x65").unwrap();
        settings.persist("/tmp/dorst/gui.ini").unwrap();

        let window = window();

        if !window.imp().button_backup_state.is_active() {
            window.imp().button_backup_state.emit_clicked();
        };

        wait_ui(500);

        assert!(*window.imp().thread_pool.lock().unwrap() == 1);
        assert!(window.task_limiter());

        window.imp().close_request();

        assert!(Path::new("/tmp/dorst/gui.ini").exists());
        remove_dir_all("/tmp/dorst").unwrap();
    }

    #[gtk::test]
    fn entries() {
        if Path::new("/tmp/dorst_test_conf.yaml").exists() {
            remove_file("/tmp/dorst_test_conf.yaml").unwrap();
        }

        let window = window();

        window
            .imp()
            .repo_entry_empty
            .set_buffer(&entry_buffer_from_str("test1/"));

        window.imp().repo_entry_empty.emit_activate();

        window
            .imp()
            .repo_entry
            .set_buffer(&entry_buffer_from_str("test2"));

        window.imp().repo_entry.emit_activate();

        assert!(window.imp().stack.visible_child_name() == Some("main".into()));
        assert!(window.get_repo_data().len() == 2);
    }

    #[gtk::test]
    fn backup() {
        if Path::new("test-gui-src").exists() {
            remove_dir_all("test-gui-src").unwrap();
        }

        if Path::new("/tmp/dorst_test-gui").exists() {
            remove_dir_all("/tmp/dorst_test-gui").unwrap();
        }

        let repo = helper::test_repo();
        let repo_dir = String::from(repo.path().to_str().unwrap());
        let mut config = tempfile::Builder::new().tempfile_in("/tmp").unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap();

        config.write_all(b"\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x68\x74\x74\x70\x3a\x2f\x2f\x6c\x6f\x63\x61\x6c\x68\x6f\x73\x74\x3a\x37\x38\x37\x30").unwrap();
        config.persist("/tmp/dorst_test_conf.yaml").unwrap();
        runtime.spawn(async move {
            helper::serve(repo, 7870);
        });

        let window = window();

        if !window.imp().button_backup_state.is_active() {
            window.imp().button_backup_state.emit_clicked();
        };

        window.set_backup_directory(&PathBuf::from("/tmp/dorst_test-gui"));
        window.set_source_directory(&PathBuf::from("test-gui-src"));
        window.imp().button_start.emit_clicked();
        wait_ui(500);
        helper::commit(repo_dir);
        wait_ui(1000);
        window.imp().button_start.emit_clicked();
        wait_ui(1000);

        assert!(window.imp().success_list.lock().unwrap().len() == 1);
        assert!(window.imp().errors_list.lock().unwrap().len() == 0);
        assert!(Path::new("/tmp/dorst_test-gui/localhost:7870.dorst/FETCH_HEAD").exists());
        assert!(Path::new("test-gui-src/localhost:7870/foo").exists());

        remove_dir_all("/tmp/dorst_test-gui").unwrap();
        remove_dir_all("test-gui-src").unwrap();
    }

    #[gtk::test]
    fn ssh_filter() {
        if Path::new("/tmp/dorst_test_conf.yaml").exists() {
            remove_file("/tmp/dorst_test_conf.yaml").unwrap();
        }

        let window = window();

        let filter_ssh = "SSH".to_variant();
        let filter_https = "HTTPS".to_variant();

        window
            .imp()
            .repo_entry_empty
            .set_buffer(&entry_buffer_from_str("invalid"));

        window.imp().repo_entry_empty.emit_activate();
        window.imp().button_start.emit_clicked();
        wait_ui(500);

        assert!(window.imp().errors_list.lock().unwrap().len() == 1);

        window
            .imp()
            .stack
            .activate_action("win.filter", Some(&filter_ssh))
            .unwrap();

        window.imp().button_start.emit_clicked();
        wait_ui(500);

        assert!(window.imp().errors_list.lock().unwrap().len() == 0);

        window
            .imp()
            .stack
            .activate_action("win.filter", Some(&filter_https))
            .unwrap();

        window.imp().button_start.emit_clicked();
        wait_ui(500);

        assert!(window.imp().errors_list.lock().unwrap().len() == 0);
    }

    #[gtk::test]
    fn task_limiter() {
        if Path::new("/tmp/dorst_test_conf.yaml").exists() {
            remove_file("/tmp/dorst_test_conf.yaml").unwrap();
        }

        let window = window();

        window.set_thread_pool_limit(1);

        window
            .imp()
            .repo_entry_empty
            .set_buffer(&entry_buffer_from_str("invalid1"));

        window.imp().repo_entry_empty.emit_activate();

        window
            .imp()
            .repo_entry
            .set_buffer(&entry_buffer_from_str("invalid2"));

        window.imp().repo_entry.emit_activate();

        window
            .imp()
            .repo_entry
            .set_buffer(&entry_buffer_from_str("invalid3"));

        window.imp().repo_entry.emit_activate();
        window.set_task_limiter(true);
        window.imp().button_start.emit_clicked();
        wait_ui(100);

        assert!(window.task_limiter());

        window.set_task_limiter(false);

        assert!(!window.task_limiter());
    }

    #[gtk::test]
    fn remove_target() {
        if Path::new("/tmp/dorst_test_conf.yaml").exists() {
            remove_file("/tmp/dorst_test_conf.yaml").unwrap();
        }

        let window = window();

        window
            .imp()
            .repo_entry_empty
            .set_buffer(&entry_buffer_from_str("invalid"));

        window.imp().repo_entry_empty.emit_activate();

        assert!(window.repos().n_items() == 1);
        assert!(window.imp().stack.visible_child_name() == Some("main".into()));

        let row = window.imp().repos_list.row_at_index(0).unwrap();
        let button = row
            .child()
            .unwrap()
            .downcast::<Box>()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<Popover>()
            .unwrap()
            .child()
            .unwrap()
            .downcast::<Box>()
            .unwrap()
            .last_child()
            .unwrap()
            .downcast::<Button>()
            .unwrap();

        button.emit_clicked();

        assert!(window.repos().n_items() == 0);
        assert!(window.imp().stack.visible_child_name() == Some("empty".into()));
    }

    #[gtk::test]
    fn main_view() {
        let mut config = tempfile::Builder::new().tempfile_in("/tmp").unwrap();

        config.write_all(b"\x73\x6f\x75\x72\x63\x65\x5f\x64\x69\x72\x65\x63\x74\x6f\x72\x79\x3a\x20\x2f\x74\x6d\x70\x0a\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x49\x4e\x56\x41\x4c\x49\x44").unwrap();
        config.persist("/tmp/dorst_test_conf.yaml").unwrap();

        let window = window();

        assert!(window.imp().stack.visible_child_name() == Some("main".into()));
    }

    #[gtk::test]
    fn empty_view() {
        let mut config = tempfile::Builder::new().tempfile_in("/tmp").unwrap();

        config.write_all(b"\x2d\x2d\x2d\x0a").unwrap();
        config.persist("/tmp/dorst_test_conf.yaml").unwrap();

        let window = window();

        assert!(window.imp().stack.visible_child_name() == Some("empty".into()));
    }

    #[gtk::test]
    fn invalid_url() {
        let mut config = tempfile::Builder::new().tempfile_in("/tmp").unwrap();

        config.write_all(b"\x73\x6f\x75\x72\x63\x65\x5f\x64\x69\x72\x65\x63\x74\x6f\x72\x79\x3a\x20\x2f\x74\x6d\x70\x0a\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x49\x4e\x56\x41\x4c\x49\x44").unwrap();
        config.persist("/tmp/dorst_test_conf.yaml").unwrap();

        let window = window();

        window.imp().button_start.emit_clicked();
        wait_ui(500);

        assert!(window.imp().errors_list.lock().unwrap().len() > 0);
    }
}
