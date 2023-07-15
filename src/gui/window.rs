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
        Arc,
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

        let action_mirror_all = gio::SimpleAction::new("mirror-all", None);
        action_mirror_all.connect_activate(clone!(@weak self as window => move |_, _| {
            window.mirror_all();
        }));

        let action_style_manager = gio::SimpleAction::new("toggle-color-scheme", None);
        action_style_manager.connect_activate(clone!(@weak self as window => move |_, _| {
            window.toggle_color_scheme();
        }));

        let action_close = gio::SimpleAction::new("close", None);
        action_close.connect_activate(clone!(@weak self as window => move |_, _| {
            window.close();
        }));

        self.add_action(&action_about);
        self.add_action(&action_mirror_all);
        self.add_action(&action_style_manager);
        self.add_action(&action_close);
    }

    fn setup_callbacks(&self) {
        self.imp()
            .repo_entry
            .connect_activate(clone!(@weak self as window => move |_| {
                window.new_repo();
            }));

        self.imp()
            .repo_entry
            .connect_icon_release(clone!(@weak self as window => move |_,_| {
                window.new_repo();
            }));

        self.imp()
            .button_backup_state
            .connect_toggled(clone!(@weak self as window => move |_| {
                let mut state = window.imp().backups_enabled.borrow_mut();
                *state = window.imp().button_backup_state.is_active();
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

    fn mirror_all(&self) {
        self.imp().button_start.set_sensitive(false);
        self.imp().button_source_dest.set_sensitive(false);
        self.imp().button_backup_dest.set_sensitive(false);
        self.imp().button_backup_state.set_sensitive(false);
        self.imp().repo_entry.set_sensitive(false);
        self.imp().banner.set_revealed(false);
        self.imp().revealer_banner.set_reveal_child(false);
        self.imp().errors_list.lock().unwrap().clear();
        self.imp().success_list.lock().unwrap().clear();
        self.imp().progress_bar.set_fraction(0.0);
        self.imp().revealer.set_reveal_child(true);

        let repos = self.repos();
        let total_repos = self.get_repo_data().len();
        let completed_repos = Arc::new(AtomicUsize::new(0));

        let dest_clone = self.get_dest_clone();
        let dest_backup = self.get_dest_backup();
        let backups_enabled = *self.imp().backups_enabled.borrow();

        for i in 0..repos.n_items() {
            if let Some(obj) = repos.item(i) {
                if let Some(repo) = obj.downcast_ref::<RepoObject>() {
                    let row = self.imp().repos_list.row_at_index(i as i32).unwrap();
                    let repo_link = repo.link();
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
                    let progress_bar = revealer.child().unwrap().downcast::<ProgressBar>().unwrap();

                    progress_bar.set_fraction(0.0);
                    revealer.set_reveal_child(true);

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
                    });
                }
            }
        }

        glib::idle_add_local(
            clone!(@weak self as window => @default-return Continue(true), move || {
                let completed = completed_repos.load(Ordering::Relaxed) as f64;
                let progress = completed / total_repos as f64;

                window.update_rows();

                if completed == total_repos as f64 {
                    let errors_locked = window.imp().errors_list.lock().unwrap().iter()
                                                                                .map(std::string::ToString::to_string)
                                                                                .collect::<Vec<_>>()
                        .join("\n");

                    if !errors_locked.is_empty() {
                        window.imp().banner.set_title(&errors_locked);
                        window.imp().revealer_banner.set_reveal_child(true);
                        window.imp().banner.set_revealed(true);
                    }

                    window.imp().progress_bar.set_fraction(1.0);
                    window.imp().revealer.set_reveal_child(false);
                    window.imp().button_source_dest.set_sensitive(true);
                    window.imp().button_backup_dest.set_sensitive(true);
                    window.imp().button_backup_state.set_sensitive(true);
                    window.imp().repo_entry.set_sensitive(true);
                    window.imp().button_start.set_sensitive(true);
                    Continue(false)
                } else {
                    window.imp().progress_bar.set_fraction(progress);
                    Continue(true)
                }
            }),
        );
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
        self.imp().repos_list.set_visible(repos.n_items() > 0);
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

    fn new_repo(&self) {
        let buffer = self.imp().repo_entry.buffer();
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
        if let Ok(file) = fs::File::open(util::xdg_path().unwrap()) {
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

        let cache_dir = glib::user_cache_dir();
        let settings_path = cache_dir.join("dorst");
        std::fs::create_dir_all(&settings_path).expect("Failed to create settings path");

        let settings = settings_path.join("gui.ini");

        keyfile
            .save_to_file(settings)
            .expect("Failed to save settings");
    }

    fn load_settings(&self) {
        let keyfile = KeyFile::new();
        let cache_dir = glib::user_cache_dir();
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

                if backup_state {
                    self.imp().button_backup_state.set_active(true);
                }
            }
        }
    }
}
