use adw::{prelude::*, subclass::prelude::*, ActionRow};
use glib::{clone, KeyFile, MainContext, Object, PRIORITY_DEFAULT};
use gtk::{gio, glib, CustomFilter, FilterListModel, License, NoSelection};

use std::{
    cell::RefMut,
    fs,
    path::PathBuf,
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

enum Message {
    MirrorRepo(window::Window),
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        Object::builder::<Window>()
            .property("application", app)
            .build()
    }

    #[cfg(debug_assertions)]
    fn setup_debug(&self) {
        self.add_css_class("devel");
    }

    fn setup_actions(&self) {
        let (tx, rx) = MainContext::channel(PRIORITY_DEFAULT);

        rx.attach(None, move |x| match x {
            Message::MirrorRepo(window) => {
                window.imp().banner.set_revealed(false);
                window.imp().revealer_banner.set_reveal_child(false);
                window.imp().errors_list.lock().unwrap().clear();
                window.imp().success_list.lock().unwrap().clear();
                window.imp().progress_bar.set_fraction(0.0);
                window.imp().revealer.set_reveal_child(true);

                let repo_data = window.get_repo_data();
                let total_repos = repo_data.len();
                let completed_repos = Arc::new(AtomicUsize::new(0));

                for repo in repo_data {
                    let dest = window.get_dest().clone();
                    let completed_repos_clone = completed_repos.clone();
                    let errors_clone = window.imp().errors_list.clone();
                    let success_clone = window.imp().success_list.clone();

                    thread::spawn(move || {
                        let destination = format!("{}/{}.dorst", &dest.display().to_string(), util::get_name(&repo.link));
                        match git::mirror_repo(&destination, &repo.link, None, None) {
                            Ok(()) => {
                                let success_item = repo.link;
                                success_clone.lock().unwrap().push(success_item);
                            },
                            Err(error) => errors_clone.lock().unwrap().push(format!("{}: {}", repo.link, error)),
                        }
                        completed_repos_clone.fetch_add(1, Ordering::Relaxed);
                    });
                }

                glib::idle_add_local(
                    clone!(@weak window => @default-return Continue(true), move || {
                        let completed = completed_repos.load(Ordering::Relaxed) as f64;
                        let progress = completed / total_repos as f64;

                        window.update_repos();

                        if completed == total_repos as f64 {
                            let errors_locked = window.imp().errors_list.lock().unwrap().iter()
                                                                                        .map(|error| error.to_string())
                                                                                        .collect::<Vec<_>>()
                                .join("\n");

                            if !errors_locked.is_empty() {
                                window.imp().banner.set_title(&errors_locked);
                                window.imp().revealer_banner.set_reveal_child(true);
                                window.imp().banner.set_revealed(true);
                            }

                            window.imp().progress_bar.set_fraction(1.0);
                            window.imp().revealer.set_reveal_child(false);
                            Continue(false)
                        } else {
                            window.imp().progress_bar.set_fraction(progress);
                            Continue(true)
                        }
                    }),
                );

                Continue(true)
            }
        });

        let action_about = gio::SimpleAction::new("about", None);
        action_about.connect_activate(clone!(@weak self as window => move |_, _| {
            window.show_about_dialog();
        }));

        let action_mirror_all = gio::SimpleAction::new("mirror-all", None);
        action_mirror_all.connect_activate(clone!(@weak self as window => move |_, _| {
            let _ = tx.send(Message::MirrorRepo(window));

        }));

        let action_style_manager = gio::SimpleAction::new("toggle-color-scheme", None);
        action_style_manager.connect_activate(clone!(@weak self as window => move |_, _| {
            window.toggle_color_scheme();
        }));

        self.add_action(&action_about);
        self.add_action(&action_mirror_all);
        self.add_action(&action_style_manager);
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
                    *window.imp().filter_option.borrow_mut() = "".to_owned();
                }

                filter_model.set_filter(window.filter().as_ref());
                window.update_repos();
            }),
        );

        self.add_action(&action_filter);
    }

    fn repos(&self) -> gio::ListStore {
        self.imp()
            .repos
            .borrow()
            .clone()
            .expect("Could not get current repositories.")
    }

    fn update_repos(&self) {
        let repos = self.repos();

        for i in 0..repos.n_items() {
            if let Some(obj) = repos.item(i) {
                if let Some(repo_object) = obj.downcast_ref::<RepoObject>() {
                    let link = repo_object.repo_data().link.clone();
                    if self.imp().success_list.lock().unwrap().contains(&link) {
                        if let Some(row) = self.imp().repos_list.row_at_index(i as i32) {
                            row.remove_css_class("warning");
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
                            row.remove_css_class("warning");
                            row.remove_css_class("success");
                            row.add_css_class("error");
                        }
                    } else if let Some(row) = self.imp().repos_list.row_at_index(i as i32) {
                        row.remove_css_class("success");
                        row.remove_css_class("error");
                        row.add_css_class("warning");
                    }
                }
            }
        }
    }

    fn get_dest(&self) -> RefMut<PathBuf> {
        self.imp().directory_output.borrow_mut()
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

    fn create_repo_row(&self, repo_object: &RepoObject) -> ActionRow {
        let row = ActionRow::builder().build();

        repo_object
            .bind_property("name", &row, "title")
            .sync_create()
            .build();

        repo_object
            .bind_property("link", &row, "subtitle")
            .sync_create()
            .build();

        row
    }

    fn new_repo(&self) {
        let buffer = self.imp().repo_entry.buffer();
        let content = buffer.text().to_string();

        if content.is_empty() {
            return;
        }

        buffer.set_text("");

        let name = util::get_name(&content).to_owned();
        let repo = RepoObject::new(name, content);
        self.repos().append(&repo);
    }

    fn set_directory(&self, directory: &PathBuf) {
        let mut dir = self.imp().directory_output.borrow_mut();
        self.show_message(directory.to_str().unwrap(), 2);
        dir.clear();
        dir.push(directory)
    }

    fn restore_data(&self) {
        if let Ok(file) = fs::File::open(util::xdg_path().unwrap()) {
            let config: serde_yaml::Value = serde_yaml::from_reader(file).unwrap();

            if let Some(targets) = config["targets"].as_sequence() {
                let repo_objects: Vec<RepoObject> = targets
                    .iter()
                    .filter_map(|target| {
                        target.as_str().map(|link| RepoData {
                            name: util::get_name(link).to_owned(),
                            link: link.to_owned(),
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
        let style_manager = adw::StyleManager::default();

        if style_manager.is_dark() {
            style_manager.set_color_scheme(adw::ColorScheme::ForceLight);
        } else {
            style_manager.set_color_scheme(adw::ColorScheme::ForceDark);
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
        adw::AboutWindow::builder()
            .application_name("DØRST")
            .version(env!("CARGO_PKG_VERSION"))
            .license_type(License::MitX11)
            .website(env!("CARGO_PKG_REPOSITORY"))
            .comments(env!("CARGO_PKG_DESCRIPTION"))
            .build()
            .present();
    }

    fn save_settings(&self) {
        let keyfile = KeyFile::new();
        let size = self.default_size();

        keyfile.set_int64("window", "width", size.0.into());
        keyfile.set_int64("window", "height", size.1.into());

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

        if settings.exists() {
            keyfile
                .load_from_file(settings, glib::KeyFileFlags::NONE)
                .expect("Failed to load settings");

            let width = keyfile.int64("window", "width").unwrap();
            let height = keyfile.int64("window", "height").unwrap();

            self.set_default_size(width.try_into().unwrap(), height.try_into().unwrap());
        }
    }
}
