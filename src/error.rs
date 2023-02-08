use std::{env, fmt, io};

#[derive(Debug)]
pub enum Error {
    CloneFailed(String),
    Config(String),
    Io(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CloneFailed(e) | Self::Config(e) | Self::Io(e) => e,
        })
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Self::Config(e.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<env::VarError> for Error {
    fn from(e: env::VarError) -> Self {
        Self::Config(e.to_string())
    }
}

impl From<git2::Error> for Error {
    fn from(e: git2::Error) -> Self {
        Self::CloneFailed(e.to_string())
    }
}
