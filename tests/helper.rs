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
        Some("HEAD"),
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
