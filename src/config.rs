use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub source_directory: String,
    pub targets: Vec<String>,
}
