use rouille::{cgi::CgiRun, Server};
use tempfile::TempDir;

use std::{fs::File, path::Path, process::Command, thread};

pub fn serve() {
    let repo = test_repo();
    let server = Server::new("localhost:7868", move |request| {
        let mut cmd = Command::new("git");

        cmd.arg("http-backend");
        cmd.env("GIT_PROJECT_ROOT", repo.path());

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

fn test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let sig = git2::Signature::now("foo", "bar").unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();

    File::create(&dir.path().join(".git").join("git-daemon-export-ok")).unwrap();

    {
        let mut index = repo.index().unwrap();

        File::create(&dir.path().join("foo")).unwrap();

        index.add_path(Path::new("foo")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "test",
            &repo.find_tree(tree_id).unwrap(),
            &[],
        )
        .unwrap();
    }

    dir
}
