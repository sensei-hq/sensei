//! #179: a daemon that dies before it binds must leave its reason where it can
//! be read, and `senseid status` (which `sensei status` runs) must show it.
//! Before, the pre-bind exits — the already-running guard, the `.expect()`s on
//! the data dir, the log and the spawn — only wrote to the parent's stderr,
//! which under launchd nobody reads; and nothing read the file at all.

use std::path::Path;
use std::process::Command;

fn senseid(home: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_senseid"))
        .args(args)
        .env("HOME", home)
        .env_remove("SENSEI_INSTANCE")
        .output()
        .unwrap()
}

fn data_dir(home: &Path) -> std::path::PathBuf {
    home.join(".sensei")
}

#[test]
fn a_start_refused_because_one_is_running_records_why() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(data_dir(home.path())).unwrap();
    // This test's own pid: alive, so the guard refuses.
    std::fs::write(data_dir(home.path()).join("serve.pid"), std::process::id().to_string())
        .unwrap();

    let out = senseid(home.path(), &["start"]);

    assert_eq!(out.status.code(), Some(1));
    let recorded = std::fs::read_to_string(data_dir(home.path()).join("startup-error.log"))
        .expect("the reason is recorded");
    assert!(recorded.contains("already running"), "{recorded}");
}

#[test]
fn status_shows_why_the_daemon_is_not_running() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(data_dir(home.path())).unwrap();
    std::fs::write(
        data_dir(home.path()).join("startup-error.log"),
        "[senseid] could not bind :7744 — address in use",
    )
    .unwrap();

    // Port 9: nothing listens there, so the daemon reads as not running.
    let out = senseid(home.path(), &["--port", "9", "status"]);

    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("not running"), "{text}");
    assert!(text.contains("address in use"), "the recorded reason is shown: {text}");
}
