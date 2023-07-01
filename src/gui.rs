use adw::{gio, prelude::*};
use gtk::{gdk::Display, CssProvider};

use repo_object::RepoData;
use window::Window;

mod repo_object;
pub mod window;

const APP_ID: &str = "org.hellbyte.dorst";

pub fn start() {
    gio::resources_register_include!("dorst.gresource").expect("Failed to register resources.");
    let app = adw::Application::builder().application_id(APP_ID).build();
    let args: Vec<String> = vec![];

    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);

    app.set_accels_for_action("win.toggle-color-scheme", &["<Primary>l"]);
    app.set_accels_for_action("win.close", &["<Primary>q"]);

    app.run_with_args(&args);
}

fn build_ui(app: &adw::Application) {
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
