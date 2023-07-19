#![forbid(unsafe_code)]

#[cfg(feature = "cli")]
mod cli;
#[cfg(any(feature = "cli", feature = "gui"))]
mod git;
#[cfg(feature = "gui")]
mod gui;
#[cfg(any(feature = "cli", feature = "gui"))]
mod util;

fn main() {
    if gui_flag() {
        #[cfg(feature = "gui")]
        gui::start();
        #[cfg(not(feature = "gui"))]
        {
            eprintln!("Error: The GUI feature is disabled.");
            std::process::exit(1);
        }
    } else {
        #[cfg(feature = "cli")]
        cli::start();
        #[cfg(not(feature = "cli"))]
        {
            println!("The CLI feature is disabled. Exiting...");
        }
    }
}

fn gui_flag() -> bool {
    std::env::args().any(|arg| arg == "--gui")
}
