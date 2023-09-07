use adw::{
    prelude::*, subclass::prelude::*, AboutWindow, ColorScheme, MessageDialog, ResponseAppearance,
};
use gtk::{
    gio::{self, ListStore, SimpleAction},
    glib::{self, clone, ControlFlow, KeyFile, MainContext, Object, Priority},
    pango::EllipsizeMode,
    Align, Box, Button, CustomFilter, FilterListModel, Label, License, ListBoxRow, NoSelection,
    Orientation, Popover, ProgressBar, Revealer, RevealerTransitionType,
};

#[cfg(feature = "logs")]
use tracing::info;

use std::{
    cell::Ref,
    fs,
    path::{Path, PathBuf},
    time,
};

mod imp;

use crate::{
    git,
    gui::{repo_object::RepoObject, RepoData},
    util,
};

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

pub enum RowMessage {
    Reset,
    Progress(f64, Status),
    Clone,
    Fetch,
    Deltas,
    Updated(String),
    Finish,
}

pub enum Status {
    Normal,
    Data,
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
        #[cfg(feature = "logs")]
        let action_logs = gio::PropertyAction::new("logs", self, "logs");

        self.add_action(&action_about);
        self.add_action(&action_process_targets);
        self.add_action(&action_close);
        self.add_action(&action_color_scheme);
        self.add_action(&action_task_limiter);
        #[cfg(feature = "logs")]
        self.add_action(&action_logs);
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

        self.connect_completed_notify(|window| {
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

    fn setup_repos(&self) {
        let model = gio::ListStore::new::<RepoObject>();
        self.imp().repos.replace(Some(model));

        let filter_model = FilterListModel::new(Some(self.repos()), self.filter());
        self.imp().repos_filtered.replace(filter_model.clone());

        let selection_model = NoSelection::new(Some(filter_model.clone()));

        self.imp().repos_list.bind_model(
            Some(&selection_model),
            clone!(@weak self as window => @default-panic, move |obj| {
                let repo_object = obj.downcast_ref().expect("The object should be of type `RepoObject`.");
                let row = window.create_repo_row(repo_object);
                row.upcast()
            }),
        );

        self.imp().repos_list.connect_row_activated(clone!(@weak self as window => move |_, row| {
            let popover_box = Box::builder().hexpand(true).build();
            let popover = Popover::builder()
                .child(&popover_box)
                .autohide(true)
                .has_arrow(true)
                .build();

            let edit_button = Button::builder()
                .label("Edit")
                .build();

            let remove_button = Button::builder()
                .label("Remove")
                .css_classes(["destructive-action"])
                .build();

            edit_button.connect_clicked(clone!(@weak window, @weak row, @weak popover => move |_| {
                let repos = window.imp().repos_filtered.borrow().clone();
                let repo_pos = row.index();
                let repo = repos.item(repo_pos.try_into().unwrap()).unwrap().downcast::<RepoObject>().unwrap();
                let entry = gtk::Entry::builder()
                    .placeholder_text(repo.link())
                    .activates_default(true)
                    .build();

                let cancel_response = "cancel";
                let edit_response = "edit";
                let dialog = MessageDialog::builder()
                    .default_width(350)
                    .heading("Edit link")
                    .body(format!("<tt>{}</tt>", repo.name()))
                    .body_use_markup(true)
                    .transient_for(&window)
                    .modal(true)
                    .destroy_with_parent(true)
                    .close_response(cancel_response)
                    .default_response(edit_response)
                    .extra_child(&entry)
                    .build();

                dialog.add_responses(&[(cancel_response, "Cancel"), (edit_response, "Edit")]);
                dialog.set_response_enabled(edit_response, false);
                dialog.set_response_appearance(edit_response, ResponseAppearance::Suggested);

                entry.connect_changed(clone!(@weak dialog => move |entry| {
                    let text = entry.text();
                    let empty = text.is_empty();

                    dialog.set_response_enabled(edit_response, !empty);

                    if empty {
                        entry.add_css_class("error");
                    } else {
                        entry.remove_css_class("error");
                    }
                }));

                dialog.connect_response(
                    None,
                    clone!(@weak entry => move |dialog, response| {
                        dialog.destroy();

                        if response != edit_response {
                            return;
                        }

                        repo.set_link(entry.text().to_string());
                        repo.set_name(util::get_name(&entry.text()));
                    }),
                );

                dialog.present();
                popover.popdown();
            }));

            remove_button.connect_clicked(clone!(@weak window, @strong row, @weak popover=> move |_| {
                let repos_filtered = window.imp().repos_filtered.borrow().clone();
                let repo_pos = row.index();
                let repo = repos_filtered.item(repo_pos.try_into().unwrap()).unwrap().downcast::<RepoObject>().unwrap();

                let cancel_response = "cancel";
                let remove_response = "remove";
                let dialog = MessageDialog::builder()
                    .heading("Remove repository")
                    .body(format!("<tt>{}</tt>", repo.name()))
                    .body_use_markup(true)
                    .transient_for(&window)
                    .modal(true)
                    .destroy_with_parent(true)
                    .close_response(cancel_response)
                    .default_response(cancel_response)
                    .build();

                dialog.add_responses(&[(cancel_response, "Cancel"), (remove_response, "Remove")]);
                dialog.set_response_appearance(remove_response, ResponseAppearance::Destructive);

                dialog.connect_response(
                    None,
                    clone!(@weak window => move |dialog, response| {
                        dialog.destroy();

                        if response != remove_response {
                            return;
                        }

                        let link = repo.link();
                        let repos = window.repos();
                        let mut position = 0;
                        while let Some(item) = repos.item(position) {
                            let repo_object = item.downcast_ref::<RepoObject>().unwrap();

                            if repo_object.link() == link {
                                repos.remove(position);
                            } else {
                                position += 1;
                            }
                        }

                        window.show_message(&format!("Removed: {}", repo.name()), 3);
                    }),
                );

                dialog.present();
                popover.popdown();
            }));

            popover_box.add_css_class("linked");
            popover_box.append(&edit_button);
            popover_box.append(&remove_button);
            popover.set_parent(row);
            popover.popup();
        }));

        self.set_repo_list_visible(&self.repos());
        self.repos()
            .connect_items_changed(clone!(@weak self as window => move |repos, _, _, _| {
                window.set_repo_list_visible(repos);
                window.set_repo_list_stack();
            }));

        filter_model.connect_items_changed(clone!(@weak self as window => move |model, _, _, _| {
            window.imp().repos_list_count.set(model.n_items());
        }));

        let action_filter = SimpleAction::new_stateful(
            "filter",
            Some(&String::static_variant_type()),
            &"All".to_variant(),
        );

        action_filter.connect_activate(clone!(@weak self as window => move |action, parameter| {
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
        }));

        self.add_action(&action_filter);
    }

    fn controls_disabled(&self, state: bool) {
        if state {
            self.imp().button_start.set_sensitive(false);
            self.imp().button_source_dest.set_sensitive(false);
            self.imp().button_backup_dest.set_sensitive(false);
            self.imp().button_backup_state.set_sensitive(false);
            self.imp().repo_entry.set_sensitive(false);

            for i in 0..self.imp().repos_list_count.get() {
                self.imp()
                    .repos_list
                    .row_at_index(i.try_into().unwrap())
                    .unwrap()
                    .set_activatable(false);
            }

            for i in 0..self.repos().n_items() {
                let obj = self.repos().item(i).unwrap();
                if let Some(repo) = obj.downcast_ref::<RepoObject>() {
                    repo.set_status("started");
                }
            }
        } else {
            self.imp().button_start.set_sensitive(true);
            self.imp().button_source_dest.set_sensitive(true);
            self.imp().button_backup_dest.set_sensitive(true);
            self.imp().button_backup_state.set_sensitive(true);
            self.imp().repo_entry.set_sensitive(true);

            for i in 0..self.imp().repos_list_count.get() {
                self.imp()
                    .repos_list
                    .row_at_index(i.try_into().unwrap())
                    .unwrap()
                    .set_activatable(true);
            }
        }
    }

    fn process_targets(&self) {
        self.controls_disabled(true);
        self.imp().banner.set_revealed(false);
        self.imp().revealer_banner.set_reveal_child(false);
        self.imp().updated_list.lock().unwrap().clear();
        self.imp().errors_list.lock().unwrap().clear();
        self.imp().success_list.lock().unwrap().clear();
        self.imp().button_source_dest.add_css_class("with_bar");
        self.imp().button_backup_state.add_css_class("with_bar");
        self.imp().progress_bar.set_fraction(0.0);
        self.imp().revealer.set_reveal_child(true);
        self.set_completed(0);

        let mut active_task = false;
        let repos = self.repos();
        let dest_clone = self.get_dest_clone();
        let dest_backup = self.get_dest_backup();
        let backups_enabled = self.imp().backups_enabled.get();
        #[cfg(feature = "logs")]
        let logs = self.imp().logs.get();

        #[cfg(feature = "logs")]
        if logs {
            info!("Started");
        }

        for i in 0..repos.n_items() {
            let obj = repos.item(i).unwrap();
            if let Some(repo) = obj.downcast_ref::<RepoObject>() {
                active_task = true;

                let repo_link = repo.link();
                let tx = self.set_row_channel(obj.clone());
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

                if self.task_limiter() {
                    while *self.imp().active_threads.lock().unwrap()
                        > *self.imp().thread_pool.lock().unwrap()
                    {
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

                *self.imp().active_threads.lock().unwrap() += 1;

                repo.process_repo(
                    &destination_clone,
                    &destination_backup,
                    backups_enabled,
                    #[cfg(feature = "gui")]
                    Some(tx.clone()),
                    #[cfg(feature = "logs")]
                    logs,
                    self.imp().active_threads.clone(),
                );
            }
        }

        if !active_task {
            self.controls_disabled(false);
        }
    }

    fn update_rows(&self) {
        let repos = self.repos();

        for i in 0..repos.n_items() {
            if let Some(obj) = repos.item(i) {
                if let Some(repo_object) = obj.downcast_ref::<RepoObject>() {
                    let link = repo_object.link().clone();
                    if self.imp().success_list.lock().unwrap().contains(&link) {
                        let mut path = self.get_dest_clone();
                        path.push(repo_object.name());

                        let branch = git::current_branch(path).unwrap();
                        repo_object.set_branch(branch);
                        repo_object.set_status("ok");

                        if self.imp().updated_list.lock().unwrap().contains(&link) {
                            repo_object.set_status("updated");
                        }
                    } else if self
                        .imp()
                        .errors_list
                        .lock()
                        .unwrap()
                        .iter()
                        .any(|x| x.contains(&link))
                    {
                        repo_object.set_status("err");
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

    fn get_dest_backup(&self) -> Ref<PathBuf> {
        self.imp().backup_directory.borrow()
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

    fn set_row_channel(&self, row: Object) -> glib::Sender<RowMessage> {
        let (tx, rx) = MainContext::channel(Priority::DEFAULT);
        let repo = row.downcast::<RepoObject>().unwrap();
        let updated_list_clone = self.imp().updated_list.clone();

        rx.attach(None, move |x| match x {
            RowMessage::Reset => {
                repo.set_progress(0.0);
                ControlFlow::Continue
            }
            RowMessage::Progress(value, progress) => {
                if value.is_nan() {
                    repo.set_progress(1.0);
                } else {
                    let fraction = match progress {
                        Status::Deltas => (value / 2.0) + 0.5,
                        Status::Data => value / 2.0,
                        Status::Normal => value,
                    };

                    repo.set_progress(fraction);
                }

                ControlFlow::Continue
            }
            RowMessage::Clone => {
                repo.set_status("cloning");
                ControlFlow::Continue
            }
            RowMessage::Fetch => {
                repo.set_status("fetching");
                ControlFlow::Continue
            }
            RowMessage::Deltas => {
                repo.set_status("resolving");
                ControlFlow::Continue
            }
            RowMessage::Updated(link) => {
                updated_list_clone.lock().unwrap().push(link);
                ControlFlow::Continue
            }
            RowMessage::Finish => {
                repo.set_progress(1.0);
                ControlFlow::Continue
            }
        });

        tx
    }

    fn create_repo_row(&self, repo_object: &RepoObject) -> ListBoxRow {
        let status_image = gtk::Image::builder().css_classes(["dim-label"]).build();

        let name = Label::builder()
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .css_classes(["heading"])
            .margin_end(4)
            .build();

        let link = Label::builder()
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .margin_top(4)
            .css_classes(["body", "caption", "dim-label"])
            .build();

        let branch = Label::builder()
            .halign(Align::Start)
            .ellipsize(EllipsizeMode::End)
            .css_classes(["caption-heading", "monospace"])
            .build();

        let pb = ProgressBar::builder()
            .halign(Align::Start)
            .pulse_step(1.0)
            .hexpand(true)
            .halign(Align::Fill)
            .build();

        let text_box = Box::builder().orientation(Orientation::Vertical).build();
        let widget_box = Box::builder().orientation(Orientation::Horizontal).build();
        let pb_box = Box::builder().orientation(Orientation::Horizontal).build();
        let status_box = Box::builder()
            .orientation(Orientation::Horizontal)
            .halign(Align::End)
            .hexpand(true)
            .build();

        let row_box = Box::builder().orientation(Orientation::Horizontal).build();
        let repo_box = Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Fill)
            .valign(Align::Center)
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .build();

        let name_box = Box::builder().orientation(Orientation::Horizontal).build();

        let revealer = Revealer::builder()
            .margin_top(4)
            .transition_type(RevealerTransitionType::Crossfade)
            .transition_duration(542)
            .child(&pb)
            .build();

        let branch_revealer = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideRight)
            .transition_duration(142)
            .child(&branch)
            .build();

        let status_revealer = Revealer::builder()
            .margin_start(12)
            .transition_type(RevealerTransitionType::Crossfade)
            .transition_duration(142)
            .child(&status_image)
            .build();

        repo_object.connect_status_notify(
            clone!(@weak self as window, @weak name, @weak pb, @weak revealer, @weak status_image, @weak status_revealer, @weak branch_revealer => move |repo_object| {
                if repo_object.status() == "ok" {
                    name.add_css_class("success");
                    name.remove_css_class("error");
                    name.remove_css_class("accent");
                    status_image.set_from_icon_name(Some("emblem-ok-symbolic"));
                    status_revealer.set_reveal_child(true);
                    branch_revealer.set_reveal_child(true);
                    revealer.set_reveal_child(false);
                } else if repo_object.status() == "updated" {
                    name.add_css_class("accent");
                    name.remove_css_class("success");
                    name.remove_css_class("error");
                    status_image.set_from_icon_name(Some("emblem-default-symbolic"));
                } else if repo_object.status() == "err" {
                    name.add_css_class("error");
                    name.remove_css_class("success");
                    name.remove_css_class("accent");
                    status_image.set_from_icon_name(Some("dialog-error-symbolic"));
                    status_revealer.set_reveal_child(true);
                    branch_revealer.set_reveal_child(false);
                    revealer.set_reveal_child(false);
                } else if repo_object.status() == "started"{
                    name.remove_css_class("error");
                    name.remove_css_class("success");
                    name.remove_css_class("accent");
                    pb.set_fraction(0.0);
                    status_revealer.set_reveal_child(false);
                    branch_revealer.set_reveal_child(false);
                    revealer.set_reveal_child(true);
                } else if repo_object.status() == "finished"{
                    if repo_object.error().is_empty() {
                        let success_list = &window.imp().success_list;
                        let link = repo_object.link();

                        success_list.lock().unwrap().push(link);

                        let mut path = window.get_dest_clone();
                        path.push(repo_object.name());

                        let branch = git::current_branch(path).unwrap();
                        repo_object.set_branch(branch);

                        if window.imp().updated_list.lock().unwrap().contains(&repo_object.link()) {
                            repo_object.set_status("updated");
                        }
                    }
                } else if repo_object.status() == "cloning"{
                    pb.add_css_class("clone");
                    pb.remove_css_class("deltas");
                    pb.remove_css_class("fetch");
                } else if repo_object.status() == "fetching"{
                    pb.add_css_class("fetch");
                    pb.remove_css_class("clone");
                    pb.remove_css_class("deltas");
                } else if repo_object.status() == "resolving"{
                    pb.add_css_class("deltas");
                    pb.remove_css_class("clone");
                    pb.remove_css_class("fetch");
                }
            }),
        );

        repo_object.connect_progress_notify(clone!(@weak pb => move |repo| {
            let value = repo.progress();
            pb.set_fraction(value);
        }));

        repo_object.connect_error_notify(clone!(@weak self as window => move |repo| {
            let errors_list = &window.imp().errors_list;
            let error = repo.error();

            errors_list.lock().unwrap().push(error);
        }));

        repo_object.connect_completed_notify(clone!(@weak self as window => move |_| {
            let completed = window.completed() + 1;

            window.set_completed(completed);

        }));

        repo_object
            .bind_property("name", &name, "label")
            .sync_create()
            .build();

        repo_object
            .bind_property("link", &link, "label")
            .sync_create()
            .build();

        repo_object
            .bind_property("branch", &branch, "label")
            .sync_create()
            .build();

        if &link.label() == "INVALID" {
            name.add_css_class("error");
        }

        pb.add_css_class("osd");
        pb.add_css_class("row-progress");
        pb_box.append(&revealer);
        name_box.append(&name);
        name_box.append(&branch_revealer);
        status_box.append(&status_revealer);
        text_box.append(&name_box);
        text_box.append(&link);
        widget_box.append(&text_box);
        widget_box.append(&status_box);
        repo_box.append(&widget_box);
        repo_box.append(&pb_box);
        row_box.append(&repo_box);

        ListBoxRow::builder().child(&row_box).build()
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
        let repo = RepoObject::new(
            name,
            content,
            String::new(),
            0.0,
            String::new(),
            String::new(),
            false,
        );

        self.repos().append(&repo);
    }

    fn set_source_directory(&self, directory: &Path) {
        let mut source_dir = self.imp().source_directory.borrow_mut();
        *source_dir = directory
            .to_path_buf()
            .into_os_string()
            .into_string()
            .unwrap();
    }

    fn select_source_directory(&self, directory: &Path) {
        self.set_source_directory(directory);
        self.show_message(
            &format!("Source directory: {}", directory.to_str().unwrap()),
            3,
        );

        self.imp()
            .button_source_dest
            .remove_css_class("suggested-action");
    }

    fn set_backup_directory(&self, directory: &Path) {
        let mut dir = self.imp().backup_directory.borrow_mut();
        dir.clear();
        dir.push(directory);
    }

    fn select_backup_directory(&self, directory: &Path) {
        self.set_backup_directory(directory);
        self.show_message(
            &format!("Backup directory: {}", directory.to_str().unwrap()),
            3,
        );

        self.imp()
            .button_backup_dest
            .remove_css_class("suggested-action");
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
                                branch: String::new(),
                                progress: 0.0,
                                status: String::new(),
                                error: String::new(),
                                completed: false,
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

            repo_object.link().contains('@')
        });

        let filter_https = CustomFilter::new(|obj| {
            let repo_object = obj
                .downcast_ref::<RepoObject>()
                .expect("The object needs to be of type `RepoObject`.");

            repo_object.link().contains("https://")
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
        let backups_enabled = self.imp().backups_enabled.get();
        let threads = *self.imp().thread_pool.lock().unwrap();
        let task_limiter = self.task_limiter();
        #[cfg(feature = "logs")]
        let logs = self.logs();
        let color_scheme = self.imp().color_scheme.lock().unwrap();

        keyfile.set_int64("window", "width", size.0.into());
        keyfile.set_int64("window", "height", size.1.into());
        keyfile.set_string("window", "theme", &color_scheme);
        keyfile.set_string("window", "filter", filter_option);
        keyfile.set_string("backup", "destination", dest.to_str().unwrap());
        keyfile.set_boolean("backup", "enabled", backups_enabled);
        keyfile.set_uint64("core", "threads", threads);
        keyfile.set_boolean("core", "task-limiter", task_limiter);
        #[cfg(feature = "logs")]
        keyfile.set_boolean("core", "logs", logs);

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
                self.imp().backups_enabled.set(backup_state);
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

            #[cfg(feature = "logs")]
            if let Ok(logs) = keyfile.boolean("core", "logs") {
                self.set_logs(logs);
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
    fn backup_error() {
        if Path::new("/tmp/dorst_test_conf.yaml").exists() {
            remove_file("/tmp/dorst_test_conf.yaml").unwrap();
        }

        let window = window();

        window
            .imp()
            .repo_entry_empty
            .set_buffer(&entry_buffer_from_str("test_backup"));

        window.imp().repo_entry_empty.emit_activate();

        if !window.imp().button_backup_state.is_active() {
            window.imp().button_backup_state.emit_clicked();
        };

        window.imp().button_start.emit_clicked();
        wait_ui(2000);

        assert!(window.imp().success_list.lock().unwrap().len() == 0);
        assert!(window.imp().updated_list.lock().unwrap().len() == 0);
        assert!(window.imp().errors_list.lock().unwrap().len() == 1);
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

        window.select_backup_directory(&PathBuf::from("/tmp/dorst_test-gui"));
        window.select_source_directory(&PathBuf::from("test-gui-src"));
        window.imp().button_start.emit_clicked();
        wait_ui(1000);
        helper::commit(repo_dir);
        wait_ui(1000);
        window.imp().button_start.emit_clicked();
        wait_ui(2000);

        assert!(window.imp().success_list.lock().unwrap().len() == 1);
        assert!(window.imp().updated_list.lock().unwrap().len() == 1);
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

        if Path::new("test-gui-src-filter").exists() {
            remove_dir_all("test-gui-src-filter").unwrap();
        }

        let repo = helper::test_repo();
        let repo_dir = String::from(repo.path().to_str().unwrap());
        let mut config = tempfile::Builder::new().tempfile_in("/tmp").unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .build()
            .unwrap();

        config.write_all(b"\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x68\x74\x74\x70\x3a\x2f\x2f\x6c\x6f\x63\x61\x6c\x68\x6f\x73\x74\x3a\x37\x38\x37\x31").unwrap();
        config.persist("/tmp/dorst_test_conf.yaml").unwrap();
        runtime.spawn(async move {
            helper::serve(repo, 7871);
        });

        let window = window();

        window.select_backup_directory(&PathBuf::from("/tmp/dorst_test-gui-filter"));
        window.select_source_directory(&PathBuf::from("test-gui-src-filter"));

        let filter_ssh = "SSH".to_variant();
        let filter_https = "HTTPS".to_variant();

        window
            .imp()
            .repo_entry
            .set_buffer(&entry_buffer_from_str("invalid"));

        window.imp().repo_entry.emit_activate();
        wait_ui(500);
        window.imp().button_start.emit_clicked();
        wait_ui(2000);

        assert!(window.imp().errors_list.lock().unwrap().len() == 1);
        assert!(window.imp().repos_list_count.get() == 2);
        assert!(window.imp().updated_list.lock().unwrap().len() == 0);

        helper::commit(repo_dir);
        wait_ui(500);

        window
            .imp()
            .stack
            .activate_action("win.filter", Some(&filter_ssh))
            .unwrap();

        window.imp().button_start.emit_clicked();
        wait_ui(1000);

        assert!(window.imp().errors_list.lock().unwrap().len() == 1);
        assert!(window.imp().repos_list_count.get() == 0);
        assert!(window.imp().updated_list.lock().unwrap().len() == 1);

        window
            .imp()
            .stack
            .activate_action("win.filter", Some(&filter_https))
            .unwrap();

        window.imp().button_start.emit_clicked();
        wait_ui(1000);

        assert!(window.imp().errors_list.lock().unwrap().len() == 1);
        assert!(window.imp().repos_list_count.get() == 0);
        assert!(window.imp().updated_list.lock().unwrap().len() == 1);

        remove_dir_all("test-gui-src-filter").unwrap();
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
    fn edit_target() {
        if Path::new("/tmp/dorst_test_conf.yaml").exists() {
            remove_file("/tmp/dorst_test_conf.yaml").unwrap();
        }

        let window = window();

        window
            .imp()
            .repo_entry_empty
            .set_buffer(&entry_buffer_from_str("invalid"));

        window.imp().repo_entry_empty.emit_activate();

        let row = window.imp().repos_list.row_at_index(0).unwrap();

        row.emit_activate();

        let button = row
            .last_child()
            .unwrap()
            .downcast::<Popover>()
            .unwrap()
            .child()
            .unwrap()
            .downcast::<Box>()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<Button>()
            .unwrap();

        button.emit_clicked();

        let dialog = &gtk::Window::list_toplevels()[0]
            .clone()
            .downcast::<MessageDialog>()
            .unwrap();

        let entry = dialog
            .extra_child()
            .unwrap()
            .downcast::<gtk::Entry>()
            .unwrap();

        let buffer = entry.buffer();

        buffer.set_text("invalid23");
        dialog.response("edit");

        let link = window
            .repos()
            .item(0)
            .unwrap()
            .downcast::<RepoObject>()
            .unwrap()
            .repo_data()
            .link;

        assert!(link == "invalid23");
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

        row.emit_activate();

        let button = row
            .last_child()
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

        let dialog = &gtk::Window::list_toplevels()[0]
            .clone()
            .downcast::<MessageDialog>()
            .unwrap();

        dialog.response("remove");

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
        wait_ui(1000);

        assert!(window.imp().errors_list.lock().unwrap().len() > 0);
    }

    #[gtk::test]
    fn select_source_directory() {
        let window = window();

        window.select_source_directory(&PathBuf::from("/source_foobar"));

        assert!(*window.imp().source_directory.borrow() == "/source_foobar");
    }

    #[gtk::test]
    fn select_backup_directory() {
        let window = window();

        window.select_backup_directory(&PathBuf::from("/backup_foobar"));

        assert!(*window.imp().backup_directory.borrow() == PathBuf::from("/backup_foobar"));
    }

    #[gtk::test]
    fn about_window() {
        let window = window();
        let version = env!("CARGO_PKG_VERSION");

        window
            .imp()
            .stack
            .activate_action("win.about", None)
            .unwrap();

        let about_window = &gtk::Window::list_toplevels()[0]
            .clone()
            .downcast::<AboutWindow>()
            .unwrap();

        assert!(about_window.version().contains(version));

        about_window.clone().close();
    }
}
