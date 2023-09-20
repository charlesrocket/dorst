use std::process::Command;

fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");

    #[cfg(feature = "gui")]
    Command::new("glib-compile-resources")
        .args([
            "src/resources/resources.xml",
            "--sourcedir=src/resources",
            "--target=src/resources/dorst.gresource",
        ])
        .status()
        .unwrap();
}
