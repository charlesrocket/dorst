use adw::{gio, prelude::*, Application};
use gtk::{gdk::Display, CssProvider};

use repo_object::RepoData;
use window::Window;

mod repo_object;
pub mod window;

const APP_ID: &str = "org.hellbyte.dorst";

fn builder() -> Application {
    gio::resources_register_include!("dorst.gresource").expect("Failed to register resources.");
    let builder = Application::builder().application_id(APP_ID).build();

    builder.connect_startup(|_| load_css());
    builder.connect_activate(build_ui);

    builder.set_accels_for_action("win.close", &["<Primary>q"]);

    builder
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    window.present();
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_str!("resources/style.css"));

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn start() {
    let args: Vec<String> = vec![];
    let app = builder();

    app.run_with_args(&args);
}

#[cfg(test)]
mod tests {
    use super::*;
    use glib::subclass::types::ObjectSubclassIsExt;
    use std::io::Write;

    pub(crate) mod helper {
        use git2::{Commit, ObjectType, Repository, Signature};
        use rouille::{cgi::CgiRun, Server};
        use tempfile::TempDir;

        use std::{fs::File, path::Path, process::Command, thread};

        pub fn test_repo() -> TempDir {
            let dir = TempDir::new().unwrap();
            let sig = Signature::now("foo", "bar").unwrap();
            let repo = Repository::init(&dir).unwrap();

            File::create(dir.path().join(".git").join("git-daemon-export-ok")).unwrap();
            File::create(dir.path().join("foo")).unwrap();
            File::create(dir.path().join("bar")).unwrap();

            {
                let mut index = repo.index().unwrap();

                index.add_path(Path::new("foo")).unwrap();
                index.write().unwrap();

                let tree_id = index.write_tree().unwrap();

                repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    "test1",
                    &repo.find_tree(tree_id).unwrap(),
                    &[],
                )
                .unwrap();
            }

            dir
        }

        pub fn serve(dir: TempDir, port: u32) {
            let server = Server::new(format!("localhost:{port}"), move |request| {
                let mut cmd = Command::new("git");

                cmd.arg("http-backend");
                cmd.env("GIT_PROJECT_ROOT", dir.path());
                cmd.start_cgi(request).unwrap()
            })
            .unwrap();

            let (_handle, sender) = server.stoppable();

            thread::spawn(move || {
                thread::sleep(std::time::Duration::from_secs(100));
                sender.send(()).unwrap();
            });
        }

        pub fn commit(dir: String) {
            let repo = Repository::open(dir).unwrap();
            let mut index = repo.index().unwrap();

            index.add_path(Path::new("bar")).unwrap();

            let oid = index.write_tree().unwrap();
            let sig = Signature::now("foo", "bar").unwrap();
            let parent = last_commit(&repo);

            repo.commit(
                Some("refs/heads/dev"),
                &sig,
                &sig,
                "test2",
                &repo.find_tree(oid).unwrap(),
                &[&parent],
            )
            .unwrap();
        }

        fn last_commit(repo: &Repository) -> Commit {
            let obj = repo
                .head()
                .unwrap()
                .resolve()
                .unwrap()
                .peel(ObjectType::Commit)
                .unwrap();

            obj.into_commit().unwrap()
        }
    }

    pub(crate) fn wait_ui(ms: u64) {
        let main_loop = glib::MainLoop::new(None, false);

        glib::timeout_add(
            std::time::Duration::from_millis(ms),
            glib::clone!(@strong main_loop => move || {
                main_loop.quit();
                Continue(false)
            }),
        );

        main_loop.run();
    }

    pub(crate) fn window() -> Window {
        let app = builder();
        Window::new(&app)
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
    fn invalid() {
        let mut config = tempfile::Builder::new().tempfile_in("/tmp").unwrap();

        config.write_all(b"\x73\x6f\x75\x72\x63\x65\x5f\x64\x69\x72\x65\x63\x74\x6f\x72\x79\x3a\x20\x2f\x74\x6d\x70\x0a\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x49\x4e\x56\x41\x4c\x49\x44").unwrap();
        config.persist("/tmp/dorst_test_conf.yaml").unwrap();

        let window = window();

        window.imp().button_start.emit_clicked();
        wait_ui(500);

        assert!(window.imp().errors_list.lock().unwrap().len() > 0);
    }
}
