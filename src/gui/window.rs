use adw::{prelude::*, subclass::prelude::*, AboutWindow, ColorScheme};
use anyhow::Result;
use glib::{clone, KeyFile, MainContext, Object, Sender, PRIORITY_DEFAULT};
use gtk::{
    gio, glib, pango::EllipsizeMode, Align, Box, Button, CustomFilter, EventSequenceState,
    FilterListModel, GestureClick, Label, License, ListBoxRow, NoSelection, Orientation, Popover,
    ProgressBar, Revealer, RevealerTransitionType,
};

use std::{
    cell::RefMut,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
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

        match &*self.imp().color_scheme.lock().unwrap().to_string() {
            "light-force" => self
                .imp()
                .style_manager
                .set_color_scheme(ColorScheme::ForceLight),
            "light-pref" => self
                .imp()
                .style_manager
                .set_color_scheme(ColorScheme::PreferLight),
            "dark-pref" => self
                .imp()
                .style_manager
                .set_color_scheme(ColorScheme::PreferDark),
            "dark-force" => self
                .imp()
                .style_manager
                .set_color_scheme(ColorScheme::ForceDark),
            _ => self
                .imp()
                .style_manager
                .set_color_scheme(ColorScheme::Default),
        }
    }

    fn setup_actions(&self) {
        let action_about = gio::SimpleAction::new("about", None);
        action_about.connect_activate(clone!(@weak self as window => move |_, _| {
            window.show_about_dialog();
        }));

        let action_process_targets = gio::SimpleAction::new("process-targets", None);
        action_process_targets.connect_activate(clone!(@weak self as window => move |_, _| {
            window.process_targets();
        }));

        let action_style_manager = gio::SimpleAction::new("toggle-color-scheme", None);
        action_style_manager.connect_activate(clone!(@weak self as window => move |_, _| {
            window.toggle_color_scheme();
        }));

        let action_close = gio::SimpleAction::new("close", None);
        action_close.connect_activate(clone!(@weak self as window => move |_, _| {
            window.close();
        }));

        let task_limiter = gio::SimpleAction::new_stateful(
            "task-limiter",
            Some(&String::static_variant_type()),
            "Disabled".to_variant(),
        );

        task_limiter.connect_activate(clone!(@weak self as window => move |action, parameter| {
            let parameter = parameter
                .unwrap()
                .get::<String>()
                .unwrap();

            let value = match parameter.as_str() {
                "Disabled" => false,
                "Enabled" => true,
                _ => unreachable!()
            };

            *window.imp().task_limiter.lock().unwrap() = value;
            action.set_state(parameter.to_variant());
        }));

        self.add_action(&action_about);
        self.add_action(&action_process_targets);
        self.add_action(&action_style_manager);
        self.add_action(&action_close);
        self.add_action(&task_limiter);
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
        let model = gio::ListStore::new(RepoObject::static_type());
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
            }));

        let action_filter = gio::SimpleAction::new("toggle-ssh-filter", None);
        action_filter.connect_activate(
            clone!(@weak self as window, @weak filter_model => move |_, _| {
                if window.imp().filter_option.borrow().to_owned().is_empty() {
                    *window.imp().filter_option.borrow_mut() = "SSH".to_owned();
                } else {
                    *window.imp().filter_option.borrow_mut() = String::new();
                }

                filter_model.set_filter(window.filter().as_ref());

                if window.imp().errors_list.lock().unwrap().len() > 0 || window.imp().success_list.lock().unwrap().len() > 0 {
                    window.update_rows();
                }
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
        self.imp().progress_bar.set_fraction(0.0);
        self.imp().revealer.set_reveal_child(true);

        let mut active_task = false;
        let repos = self.repos();
        let total_repos = self.get_repo_data().len();
        let completed_repos = Arc::new(AtomicUsize::new(0));
        let completed_repos_clone = completed_repos.clone();
        let dest_clone = self.get_dest_clone();
        let dest_backup = self.get_dest_backup();
        let backups_enabled = *self.imp().backups_enabled.borrow();

        let thread_pool = Arc::new(Mutex::new(0));

        glib::idle_add_local(
            clone!(@weak self as window => @default-return Continue(true), move || {
                let completed = completed_repos_clone.load(Ordering::Relaxed) as f64;
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
                    window.controls_disabled(false);
                    Continue(false)
                } else {
                    window.imp().progress_bar.set_fraction(progress);
                    Continue(true)
                }
            }),
        );

        for i in 0..repos.n_items() {
            if let Some(obj) = repos.item(i) {
                if let Some(repo) = obj.downcast_ref::<RepoObject>() {
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
                        let completed_repos_clone = completed_repos.clone();
                        let errors_clone = self.imp().errors_list.clone();
                        let success_clone = self.imp().success_list.clone();
                        let revealer = window::Window::get_row_revealer(&row);
                        let progress_bar =
                            revealer.child().unwrap().downcast::<ProgressBar>().unwrap();

                        progress_bar.set_fraction(0.0);
                        revealer.set_reveal_child(true);

                        if *self.imp().task_limiter.lock().unwrap() {
                            while *thread_pool_clone.lock().unwrap() > 6 {
                                self.update_rows();
                                let wait_loop = glib::MainLoop::new(None, false);

                                glib::timeout_add(
                                    std::time::Duration::from_millis(50),
                                    glib::clone!(@strong wait_loop => move || {
                                        wait_loop.quit();
                                        glib::Continue(false)
                                    }),
                                );

                                wait_loop.run();
                            }
                        }

                        *thread_pool_clone.lock().unwrap() += 1;

                        thread::spawn(move || {
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

                            completed_repos_clone.fetch_add(1, Ordering::Relaxed);
                            *thread_pool_clone.lock().unwrap() -= 1;
                        });
                    }
                }
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

    fn repos(&self) -> gio::ListStore {
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

    fn set_repo_list_visible(&self, repos: &gio::ListStore) {
        let repos_list_activated = repos.n_items() > 0;
        self.imp().repos_list.set_visible(repos_list_activated);

        if repos_list_activated {
            self.imp().stack.set_visible_child_name("main");
        } else {
            self.imp().stack.set_visible_child_name("empty");
        }
    }

    fn set_row_channel(row: &ListBoxRow) -> glib::Sender<Message> {
        let (tx, rx) = MainContext::channel(PRIORITY_DEFAULT);
        let revealer = window::Window::get_row_revealer(row);
        let progress_bar = revealer.child().unwrap().downcast::<ProgressBar>().unwrap();

        rx.attach(None, move |x| match x {
            Message::Reset => {
                progress_bar.set_fraction(0.0);
                revealer.set_reveal_child(true);

                Continue(true)
            }
            Message::Progress(value) => {
                if value.is_nan() {
                    progress_bar.set_fraction(1.0);
                } else {
                    progress_bar.set_fraction(value);
                }

                if progress_bar.fraction() == 1.0 {
                    revealer.set_reveal_child(false);
                }

                Continue(true)
            }
            Message::Clone => {
                progress_bar.add_css_class("clone");
                progress_bar.remove_css_class("deltas");
                progress_bar.remove_css_class("fetch");
                Continue(true)
            }
            Message::Fetch => {
                progress_bar.add_css_class("fetch");
                progress_bar.remove_css_class("clone");
                progress_bar.remove_css_class("deltas");
                Continue(true)
            }
            Message::Deltas => {
                progress_bar.add_css_class("deltas");
                progress_bar.remove_css_class("clone");
                progress_bar.remove_css_class("fetch");
                Continue(true)
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

        match filter_state.as_str() {
            "" => None,
            "SSH" => Some(filter_ssh),
            _ => unreachable!(),
        }
    }

    fn toggle_color_scheme(&self) {
        if self.imp().style_manager.is_dark() {
            self.imp()
                .style_manager
                .set_color_scheme(ColorScheme::ForceLight);
        } else {
            self.imp()
                .style_manager
                .set_color_scheme(ColorScheme::ForceDark);
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
        let backups_enabled = *self.imp().backups_enabled.borrow();
        let mut color_scheme = self.imp().color_scheme.lock().unwrap();

        match self.imp().style_manager.color_scheme() {
            ColorScheme::ForceLight => *color_scheme = "light-force".to_owned(),
            ColorScheme::PreferLight => *color_scheme = "light-pref".to_owned(),
            ColorScheme::PreferDark => *color_scheme = "dark-pref".to_owned(),
            ColorScheme::ForceDark => *color_scheme = "dark-force".to_owned(),
            _ => *color_scheme = "default".to_owned(),
        }

        keyfile.set_int64("window", "width", size.0.into());
        keyfile.set_int64("window", "height", size.1.into());
        keyfile.set_string("window", "theme", &color_scheme);
        keyfile.set_string("backup", "destination", dest.to_str().unwrap());
        keyfile.set_boolean("backup", "enabled", backups_enabled);

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
        let mut theme = self.imp().color_scheme.lock().unwrap();

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
                *theme = color_scheme.to_string();
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::{helper, wait_ui, window};
    use std::{
        fs::{remove_dir_all, remove_file},
        io::Write,
        path::Path,
    };

    fn entry_buffer_from_str(string: &str) -> gtk::EntryBuffer {
        gtk::EntryBuffer::builder().text(string).build()
    }

    #[gtk::test]
    fn color_scheme() {
        let window = window();
        let style_manager = &window.imp().style_manager;
        let color_scheme_a = style_manager.color_scheme();

        window
            .imp()
            .stack
            .activate_action("win.toggle-color-scheme", None)
            .unwrap();

        let color_scheme_b = style_manager.color_scheme();

        assert!(color_scheme_a != color_scheme_b);

        window
            .imp()
            .stack
            .activate_action("win.toggle-color-scheme", None)
            .unwrap();

        let color_scheme_c = style_manager.color_scheme();

        assert!(color_scheme_b != color_scheme_c);
    }

    #[gtk::test]
    fn settings() {
        fs::create_dir_all("/tmp/dorst").unwrap();
        let mut settings = tempfile::Builder::new().tempfile_in("/tmp/dorst").unwrap();

        settings.write_all(b"\x5b\x77\x69\x6e\x64\x6f\x77\x5d\x0a\x77\x69\x64\x74\x68\x3d\x34\x32\x33\x0a\x68\x65\x69\x67\x68\x74\x3d\x36\x30\x30\x0a\x74\x68\x65\x6d\x65\x3d\x64\x65\x66\x61\x75\x6c\x74\x0a\x0a\x5b\x62\x61\x63\x6b\x75\x70\x5d\x0a\x64\x65\x73\x74\x69\x6e\x61\x74\x69\x6f\x6e\x3d\x74\x65\x73\x74\x2d\x67\x75\x69\x0a\x65\x6e\x61\x62\x6c\x65\x64\x3d\x74\x72\x75\x65").unwrap();
        settings.persist("/tmp/dorst/gui.ini").unwrap();

        let window = window();

        if !window.imp().button_backup_state.is_active() {
            window.imp().button_backup_state.emit_clicked();
        };

        wait_ui(500);
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
        helper::commit(repo_dir);
        wait_ui(500);
        window.imp().button_start.emit_clicked();
        wait_ui(500);

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
            .activate_action("win.toggle-ssh-filter", None)
            .unwrap();

        window.imp().button_start.emit_clicked();
        wait_ui(500);

        assert!(window.imp().errors_list.lock().unwrap().len() == 0);

        window
            .imp()
            .stack
            .activate_action("win.toggle-ssh-filter", None)
            .unwrap();

        window.imp().button_start.emit_clicked();
        wait_ui(500);

        assert!(window.imp().errors_list.lock().unwrap().len() == 1);
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
}
