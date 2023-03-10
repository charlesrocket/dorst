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

fn test_config(target: &str, dest: &str) {
    let config_dest = format!("{}/config.yaml", dest);
    let test_target = format!("{}/{}", dir(), target)
        .split(',')
        .map(ToString::to_string)
        .collect();

    let test_config = TestConfig {
        targets: test_target,
    };

    std::fs::create_dir_all(dest).unwrap();

    let config = serde_yaml::to_string(&test_config).unwrap();
    let mut file = File::create(config_dest).unwrap();

    file.write_all(config.as_bytes()).unwrap();
}

fn test_repo(bytes: &[u8], dest: &str) {
    let mut repo = tempfile::NamedTempFile::new().unwrap();
    repo.write_all(bytes).unwrap();

    let tar_gz = File::open(repo.path()).unwrap();
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive.unpack(dest).unwrap();
}

pub fn test_setup(file: &[u8], target: &str, dest: &str) {
    test_repo(file, dest);
    test_config(target, dest);
}
