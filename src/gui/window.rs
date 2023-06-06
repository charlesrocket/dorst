use adw::{prelude::*, subclass::prelude::*, ActionRow};
use git2::{AutotagOption, FetchOptions, Repository};
use glib::{clone, Object};
use gtk::{gio, glib, CustomFilter, FilterListModel, License, NoSelection};

use std::{
    fs,
    path::{Path, PathBuf},
};

mod imp;

use crate::gui::repo_object::RepoObject;
use crate::gui::RepoData;
use crate::{git, util};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
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
        let selection_model = NoSelection::new(Some(filter_model));
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
    }

    fn repos(&self) -> gio::ListStore {
        self.imp()
            .repos
            .borrow()
            .clone()
            .expect("Could not get current repositories.")
    }

    fn set_repo_list_visible(&self, repos: &gio::ListStore) {
        self.imp().repos_list.set_visible(repos.n_items() > 0);
    }

    fn create_repo_row(&self, repo_object: &RepoObject) -> ActionRow {
        let row = ActionRow::builder().build();

        repo_object
            .bind_property("link", &row, "title")
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

        let repo = RepoObject::new(content);
        self.repos().append(&repo);
    }

    fn set_directory(&self, directory: &PathBuf) {
        let mut dir = self.imp().directory_output.borrow_mut();
        dir.clear();
        dir.push(directory)
    }

    fn clone_repo(
        &self,
        target: &str,
        destination: &str,
        git_config: &git2::Config,
    ) -> Result<Repository, git2::Error> {
        let callbacks = git::set_callbacks(git_config);
        let _target_name = util::get_name(target);

        let mut fetch_options = FetchOptions::new();
        let mut repo_builder = git2::build::RepoBuilder::new();
        let builder = repo_builder
            .bare(true)
            .remote_create(|repo, name, url| repo.remote_with_fetch(name, url, "+refs/*:refs/*"));

        fetch_options
            .remote_callbacks(callbacks)
            .download_tags(AutotagOption::All);

        let mirror = builder
            .fetch_options(fetch_options)
            .clone(target, Path::new(&destination))?;

        mirror.config()?.set_bool("remote.origin.mirror", true)?;
        git::set_default_branch(&mirror)?;

        Ok(mirror)
    }

    fn fetch_repo(
        &self,
        target: &str,
        path: &str,
        git_config: &git2::Config,
    ) -> Result<Repository, git2::Error> {
        let mirror = Repository::open(path)?;
        let _target_name = util::get_name(target);

        {
            let callbacks = git::set_callbacks(git_config);
            let mut fetch_options = FetchOptions::new();
            let mut remote = mirror
                .find_remote("origin")
                .or_else(|_| mirror.remote_anonymous(target))?;

            fetch_options.remote_callbacks(callbacks);
            remote.download(&[] as &[&str], Some(&mut fetch_options))?;

            let default_branch = remote.default_branch()?;

            mirror.set_head(default_branch.as_str().unwrap())?;
            remote.disconnect()?;
            remote.update_tips(None, true, AutotagOption::Unspecified, None)?;
        }

        Ok(mirror)
    }

    fn mirror_repo(&self, target: &str) {
        let git_config = git2::Config::open_default().unwrap();
        let dest = format!(
            "{}/{}.dorst",
            &self.imp().directory_output.borrow_mut().display(),
            util::get_name(target)
        );

        if Path::new(&dest).exists() {
            self.fetch_repo(target, &dest, &git_config).unwrap()
        } else {
            self.clone_repo(target, &dest, &git_config).unwrap()
        };
    }

    fn mirror_all(&self) {
        let links: Vec<RepoData> = self
            .repos()
            .snapshot()
            .iter()
            .filter_map(Cast::downcast_ref::<RepoObject>)
            .map(RepoObject::repo_data)
            .collect();

        for repo_data in links {
            self.mirror_repo(&repo_data.link);
        }
    }

    fn restore_data(&self) {
        if let Ok(file) = fs::File::open(util::xdg_path().unwrap()) {
            let config: serde_yaml::Value = serde_yaml::from_reader(file).unwrap();

            if let Some(targets) = config["targets"].as_sequence() {
                let repo_objects: Vec<RepoObject> = targets
                    .iter()
                    .filter_map(|target| {
                        target.as_str().map(|link| RepoData {
                            link: link.to_owned(),
                        })
                    })
                    .map(RepoObject::from_repo_data)
                    .collect();

                self.repos().extend_from_slice(&repo_objects);
            }
        }
    }

    // TODO
    fn filter(&self) -> Option<CustomFilter> {
        let filter_state: String = "All".to_string();
        let filter_gitlab = CustomFilter::new(|obj| {
            let repo_object = obj
                .downcast_ref::<RepoObject>()
                .expect("The object needs to be of type `RepoObject`.");

            !repo_object.repo_data().link.contains("gitlab.com")
        });

        let filter_github = CustomFilter::new(|obj| {
            let repo_object = obj
                .downcast_ref::<RepoObject>()
                .expect("The object needs to be of type `RepoObject`.");

            !repo_object.repo_data().link.contains("github.com")
        });

        match filter_state.as_str() {
            "All" => None,
            "GitLab" => Some(filter_gitlab),
            "GitHub" => Some(filter_github),
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
}
