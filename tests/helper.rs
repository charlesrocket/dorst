use flate2::read::GzDecoder;
use serde::Serialize;
use tar::Archive;

use std::{env, fs::File, io::Write};

#[derive(Serialize)]
struct TestConfig {
    targets: Vec<String>,
}

fn dir() -> String {
    env::current_dir().unwrap().display().to_string()
}

pub fn test_config(dest: &str, target: &str) {
    let dest = format!("{}/{}", dir(), dest);
    let test_target = format!("{}/{}", dir(), target)
        .split(',')
        .map(ToString::to_string)
        .collect();

    let test_config = TestConfig {
        targets: test_target,
    };
    let config = serde_yaml::to_string(&test_config).unwrap();
    let mut file = File::create(dest).unwrap();

    file.write_all(config.as_bytes()).unwrap();
}

pub fn test_repo(bytes: &[u8]) {
    let mut repo = tempfile::NamedTempFile::new().unwrap();
    repo.write_all(bytes).unwrap();

    let tar_gz = File::open(repo.path()).unwrap();
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive.unpack(".").unwrap();
}
