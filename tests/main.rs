use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;

use std::{env, error::Error, fs::remove_dir_all, io::Write, path::Path, thread};

use crate::{
    files::{CONFIG_BOOTSTRAP, CONFIG_EMPTY, CONFIG_INVALID_URL, CONFIG_MIRROR},
    helper::{commit, serve, test_repo},
};

mod files {
    pub const CONFIG_BOOTSTRAP: &[u8; 72] = b"\x2d\x2d\x2d\x0a\x73\x6f\x75\x72\x63\x65\x5f\x64\x69\x72\x65\x63\x74\x6f\x72\x79\x3a\x20\x74\x65\x73\x74\x2d\x62\x6f\x6f\x74\x73\x74\x72\x61\x70\x0a\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x68\x74\x74\x70\x3a\x2f\x2f\x6c\x6f\x63\x61\x6c\x68\x6f\x73\x74\x3a\x37\x38\x36\x38\x0a";

    pub const CONFIG_MIRROR: &[u8; 69] = b"\x2d\x2d\x2d\x0a\x73\x6f\x75\x72\x63\x65\x5f\x64\x69\x72\x65\x63\x74\x6f\x72\x79\x3a\x20\x74\x65\x73\x74\x2d\x6d\x69\x72\x72\x6f\x72\x0a\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x68\x74\x74\x70\x3a\x2f\x2f\x6c\x6f\x63\x61\x6c\x68\x6f\x73\x74\x3a\x37\x38\x36\x38\x0a";

    pub const CONFIG_EMPTY: &[u8; 4] = b"\x2d\x2d\x2d\x0a";

    pub const CONFIG_INVALID_URL: &[u8; 38] =
        b"\x73\x6f\x75\x72\x63\x65\x5f\x64\x69\x72\x65\x63\x74\x6f\x72\x79\x3a\x20\x7e\x2f\x73\x72\x63\x0a\x74\x61\x72\x67\x65\x74\x73\x3a\x0a\x20\x20\x2d\x20\x2f";
}

mod helper {
    use git2::{Commit, ObjectType, Repository, Signature};
    use rouille::{cgi::CgiRun, Server};
    use tempfile::TempDir;

    use std::{fs::File, path::Path, process::Command, thread};

    pub fn test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let sig = Signature::now("foo", "bar").unwrap();
        let repo = Repository::init(&dir).unwrap();

        File::create(dir.path().join(".git").join("git-daemon-export-ok")).unwrap();
        File::create(dir.path().join("foo")).unwrap();
        File::create(dir.path().join("bar")).unwrap();

        {
            let mut index = repo.index().unwrap();

            index.add_path(Path::new("foo")).unwrap();
            index.write().unwrap();

            let tree_id = index.write_tree().unwrap();

            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "test1",
                &repo.find_tree(tree_id).unwrap(),
                &[],
            )
            .unwrap();
        }

        dir
    }

    pub fn serve(dir: TempDir) {
        let server = Server::new("localhost:7868", move |request| {
            let mut cmd = Command::new("git");

            cmd.arg("http-backend");
            cmd.env("GIT_PROJECT_ROOT", dir.path());
            cmd.env("GIT_HTTP_EXPORT_ALL", "");
            cmd.start_cgi(request).unwrap()
        })
        .unwrap();

        let (_handle, sender) = server.stoppable();

        thread::spawn(move || {
            thread::sleep(std::time::Duration::from_secs(10));
            sender.send(()).unwrap();
        });
    }

    pub fn commit(dir: String) {
        let repo = Repository::open(dir).unwrap();
        let mut index = repo.index().unwrap();

        index.add_path(Path::new("bar")).unwrap();

        let oid = index.write_tree().unwrap();
        let sig = Signature::now("foo", "bar").unwrap();
        let parent = last_commit(&repo);

        repo.commit(
            Some("refs/heads/dev"),
            &sig,
            &sig,
            "test2",
            &repo.find_tree(oid).unwrap(),
            &[&parent],
        )
        .unwrap();
    }

    fn last_commit(repo: &Repository) -> Commit {
        let obj = repo
            .head()
            .unwrap()
            .resolve()
            .unwrap()
            .peel(ObjectType::Commit)
            .unwrap();

        obj.into_commit().unwrap()
    }
}

#[test]
fn init() -> Result<(), Box<dyn Error>> {
    env::set_var("XDG_CONFIG_HOME", "test-init");

    if Path::new("test-init").exists() {
        remove_dir_all("test-init")?;
    }

    let mut cmd = Command::cargo_bin("dorst")?;

    cmd.write_stdin("/tmp\ninit-test-target/\n")
        .assert()
        .failure()
        .stderr(contains("init-test-target: unsupported URL protocol;"));

    if Path::new("test-init").exists() {
        remove_dir_all("test-init")?;
    }

    Ok(())
}

#[test]
fn bootstrap() -> Result<(), Box<dyn Error>> {
    if Path::new("test-bootstrap").exists() {
        remove_dir_all("test-bootstrap")?;
    }

    let repo = test_repo();
    let mut clone = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()?;

    config.write_all(CONFIG_BOOTSTRAP)?;
    runtime.spawn(async move {
        serve(repo);
    });

    thread::sleep(std::time::Duration::from_millis(300));

    clone
        .arg("--bootstrap")
        .arg("--config")
        .arg(config.path())
        .assert()
        .success()
        .stdout(contains(
            "COMPLETED\u{1b}[0m \
             \u{1b}[37m(\u{1b}[0m\u{1b}[1;92m1\u{1b}[0m\u{1b}[37m)\u{1b}[0m",
        ));

    assert!(Path::new("test-bootstrap/localhost:7868/.git").exists());
    assert!(Path::new("test-bootstrap/localhost:7868/foo").exists());

    if Path::new("test-bootstrap").exists() {
        remove_dir_all("test-bootstrap")?;
    }

    Ok(())
}

#[test]
fn mirror() -> Result<(), Box<dyn Error>> {
    if Path::new("test-mirror").exists() {
        remove_dir_all("test-mirror")?;
    }

    let repo = test_repo();
    let repo_dir = String::from(repo.path().to_str().unwrap());
    let mut clone = Command::cargo_bin("dorst")?;
    let mut fetch = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .build()?;

    config.write_all(CONFIG_MIRROR)?;
    runtime.spawn(async move {
        serve(repo);
    });

    thread::sleep(std::time::Duration::from_millis(300));

    clone
        .arg("--config")
        .arg(config.path())
        .arg("test-mirror")
        .assert()
        .success()
        .stdout(contains(
            "COMPLETED\u{1b}[0m \
             \u{1b}[37m(\u{1b}[0m\u{1b}[1;92m1\u{1b}[0m\u{1b}[37m)\u{1b}[0m",
        ));

    commit(repo_dir);
    fetch
        .arg("--config")
        .arg(config.path())
        .arg("test-mirror")
        .assert()
        .success()
        .stdout(contains(
            "COMPLETED\u{1b}[0m \
             \u{1b}[37m(\u{1b}[0m\u{1b}[1;92m1\u{1b}[0m\u{1b}[37m)\u{1b}[0m",
        ));

    if Path::new("test-mirror").exists() {
        remove_dir_all("test-mirror")?;
    }

    Ok(())
}

#[test]
fn config_empty() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;

    config.write_all(CONFIG_EMPTY)?;
    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains("missing field"));

    Ok(())
}

#[test]
fn config_invalid_url() -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::cargo_bin("dorst")?;
    let mut config = NamedTempFile::new()?;

    config.write_all(CONFIG_INVALID_URL)?;
    cmd.arg("--config")
        .arg(config.path())
        .assert()
        .failure()
        .stderr(contains("Invalid URL"));

    Ok(())
}
