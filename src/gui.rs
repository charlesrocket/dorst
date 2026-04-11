use adw::{Application, gio, prelude::*};
use gtk::{CssProvider, gdk::Display};

use repo_object::RepoData;
use window::Window;

mod preferences;
mod repo_box;
mod repo_object;
pub mod window;

const APP_ID: &str = "org.hellbyte.dorst";

fn builder() -> Application {
    let bytes = glib::Bytes::from_static(include_bytes!("resources/dorst.gresource"));
    let resource = gio::Resource::from_data(&bytes).expect("Failed to load resource");
    gio::resources_register(&resource);

    let builder = Application::builder().application_id(APP_ID).build();

    builder.connect_startup(|_| load_css());
    builder.connect_activate(build_ui);

    builder.set_accels_for_action("win.close", &["<Primary>q"]);
    builder.set_accels_for_action("win.task-limiter", &["<Primary>l"]);

    builder
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    window.present();
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("resources/style.css"));

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn start() {
    #[cfg(feature = "logs")]
    let _logger = libdorst::init_logs();

    let args: Vec<String> = vec![];
    let app = builder();

    app.run_with_args(&args);
}

#[cfg(test)]
pub fn wait_ui(ms: u64) {
    let main_loop = glib::MainLoop::new(None, false);

    glib::timeout_add(
        std::time::Duration::from_millis(ms),
        glib::clone!(
            #[strong]
            main_loop,
            move || {
                main_loop.quit();
                glib::ControlFlow::Break
            }
        ),
    );

    main_loop.run();
}
