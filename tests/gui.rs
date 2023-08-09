mod gui {
    use assert_cmd::prelude::*;

    use std::{error::Error, process::Command};

    #[cfg(feature = "gui")]
    #[test]
    fn start() -> Result<(), Box<dyn Error>> {
        use std::{thread, time::Duration};

        let mut cmd = Command::cargo_bin("dorst")?;
        cmd.arg("--gui");

        let mut spawned = cmd.spawn()?;

        thread::sleep(Duration::from_secs(1));

        let status = spawned.try_wait()?;
        assert!(status.is_none(), "early exit");

        spawned.kill()?;

        Ok(())
    }

    #[cfg(not(feature = "gui"))]
    #[test]
    fn start() -> Result<(), Box<dyn Error>> {
        use predicates::str::contains;

        let mut cmd = Command::cargo_bin("dorst")?;
        cmd.arg("--gui")
            .assert()
            .failure()
            .stderr(contains("The GUI feature is disabled"));

        Ok(())
    }
}
