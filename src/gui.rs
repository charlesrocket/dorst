use adw::{gio, prelude::*};

use repo_object::RepoData;
use window::Window;

mod repo_object;
mod repo_row;
mod window;

const APP_ID: &str = "org.hellbyte.dorst";

pub fn start() {
    gio::resources_register_include!("dorst.gresource").expect("Failed to register resources.");
    let app = adw::Application::builder().application_id(APP_ID).build();
    let args: Vec<String> = vec![];

    app.connect_activate(build_ui);

    app.set_accels_for_action("win.toggle-color-scheme", &["<Primary>l"]);
    app.set_accels_for_action("win.close", &["<Primary>q"]);

    app.run_with_args(&args);
}

fn build_ui(app: &adw::Application) {
    let window = Window::new(app);
    window.present();
}
