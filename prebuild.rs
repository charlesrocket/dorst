fn main() {
    built::write_built_file().expect("Failed to acquire build-time information");

    #[cfg(feature = "gui")]
    glib_build_tools::compile_resources(
        &["src/resources"],
        "src/resources/resources.gresource.xml",
        "dorst.gresource",
    );
}
