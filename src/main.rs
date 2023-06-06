#![forbid(unsafe_code)]

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "gui")]
mod gui;

mod git;
mod util;

fn main() {
    if std::env::args().count() == 2
        && std::env::args().last() == Some("gui".to_owned())
        && cfg!(feature = "gui")
    {
        #[cfg(feature = "gui")]
        gui::start();
    } else {
        #[cfg(feature = "cli")]
        cli::start();
    }
}
