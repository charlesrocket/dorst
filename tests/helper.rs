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

pub fn test_config() {
    let dest = format!("{}/local.yaml", dir());
    let target = format!("{}/testrepo", dir())
        .split(',')
        .map(ToString::to_string)
        .collect();

    let test_config = TestConfig { targets: target };
    let config = serde_yaml::to_string(&test_config).unwrap();
    let mut file = File::create(dest).unwrap();

    file.write_all(config.as_bytes()).unwrap();
}

pub fn test_repo() {
    let repo = format!("{}/tests/testrepo.tar.gz", dir());
    let tar_gz = File::open(repo).unwrap();
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive.unpack(".").unwrap();
}
