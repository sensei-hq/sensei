//! `scripts/db-backup-essential.sh` (#181), driven with a fake `dbd` and `psql`.
//!
//! The backup runs before every install. It kept three ~4 GB snapshots at rest
//! because it pruned BEFORE writing, offered no way to skip it while iterating,
//! and swallowed a failed table export — `|| echo skip` with stderr discarded —
//! so a snapshot missing `sessions` still reported "complete".

use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/db-backup-essential.sh")
}

/// Fake tools on PATH: `psql` always reachable; `dbd export -n <t> --output <d>`
/// writes `<d>/<t>.jsonl`, or fails when `<t>` equals `$FAIL_TABLE`.
fn fakes(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let write = |name: &str, body: &str| {
        let p = bin.join(name);
        std::fs::write(&p, body).unwrap();
        Command::new("chmod").arg("+x").arg(&p).status().unwrap();
    };
    write("psql", "#!/bin/sh\nexit 0\n");
    write(
        "dbd",
        r#"#!/bin/sh
t=""; out=""
while [ $# -gt 0 ]; do
  case "$1" in -n) t="$2"; shift;; --output) out="$2"; shift;; esac; shift
done
[ "$t" = "$FAIL_TABLE" ] && { echo "export of $t failed" >&2; exit 3; }
mkdir -p "$out" && echo '{}' > "$out/$t.jsonl"
"#,
    );
    bin
}

fn run(dir: &Path, extra: &[(&str, &str)]) -> std::process::Output {
    let bin = fakes(dir);
    let mut c = Command::new("bash");
    c.arg(script())
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("BACKUP_DIR", dir.join("essential"))
        .env("BACKUP_KEEP", "2")
        .env_remove("SKIP_DB_BACKUP")
        .env_remove("FAIL_TABLE");
    for (k, v) in extra {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn snapshots(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir.join("essential"))
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

#[test]
fn it_keeps_the_newest_two_after_writing_not_three_at_rest() {
    let t = tempfile::tempdir().unwrap();
    for _ in 0..3 {
        let o = run(t.path(), &[]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        std::thread::sleep(std::time::Duration::from_millis(1100)); // distinct timestamps
    }
    assert_eq!(snapshots(t.path()).len(), 2, "pruned after writing: two at rest");
}

#[test]
fn a_failed_table_export_fails_the_backup_and_keeps_the_previous_snapshots() {
    let t = tempfile::tempdir().unwrap();
    assert!(run(t.path(), &[]).status.success());
    let before = snapshots(t.path());
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let o = run(t.path(), &[("FAIL_TABLE", "activity.sessions")]);

    assert!(!o.status.success(), "a backup missing a table must not report success");
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("activity.sessions"), "names the table: {err}");
    assert_eq!(snapshots(t.path()), before, "the partial snapshot is removed; earlier ones stay");
}

#[test]
fn skip_db_backup_writes_nothing() {
    let t = tempfile::tempdir().unwrap();
    let o = run(t.path(), &[("SKIP_DB_BACKUP", "1")]);
    assert!(o.status.success());
    assert!(snapshots(t.path()).is_empty());
    assert!(String::from_utf8_lossy(&o.stdout).contains("SKIP_DB_BACKUP"), "says it skipped");
}
