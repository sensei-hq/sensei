//! The database schema, compiled into the binary.
//!
//! The daemon used to fetch its schema at deploy time from the source repo's
//! tag (`sensei-hq/sensei/database@v{version}`). That repo is private, so for
//! every user outside the org a fresh install could not create its database.
//! The schema now ships inside the binary: `build.rs` embeds the git-tracked
//! files under `database/`, and [`materialize`] writes them to a directory dbd
//! can deploy from. The schema is therefore always the one this binary was
//! built with, and installing needs no network.

use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/embedded_schema.rs"));
include!("schema_select.rs");

/// Write the embedded schema to `dest`, replacing anything already there, and
/// return `dest`. Files are written to a sibling temp directory first and moved
/// into place, so an interrupted run never leaves a half-written schema at
/// `dest` for a later deploy to apply.
pub fn materialize(dest: &Path) -> Result<PathBuf, String> {
    let parent = dest
        .parent()
        .ok_or_else(|| format!("schema destination has no parent: {}", dest.display()))?;
    let name = dest
        .file_name()
        .ok_or_else(|| format!("schema destination has no name: {}", dest.display()))?
        .to_string_lossy();
    let tmp = parent.join(format!(".{name}.tmp-{}", std::process::id()));
    let fail = |what: &str, p: &Path, e: std::io::Error| format!("{what} {}: {e}", p.display());

    std::fs::create_dir_all(parent).map_err(|e| fail("creating", parent, e))?;
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).map_err(|e| fail("clearing", &tmp, e))?;
    }
    for (rel, bytes) in EMBEDDED_SCHEMA {
        let file = tmp.join(rel);
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| fail("creating", dir, e))?;
        }
        std::fs::write(&file, bytes).map_err(|e| fail("writing", &file, e))?;
    }
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| fail("replacing", dest, e))?;
    }
    std::fs::rename(&tmp, dest).map_err(|e| fail("moving schema into", dest, e))?;
    Ok(dest.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn names() -> Vec<&'static str> {
        EMBEDDED_SCHEMA.iter().map(|(n, _)| *n).collect()
    }

    #[test]
    fn embeds_exactly_the_tracked_schema_files() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let out = Command::new("git")
            .args(["-C", root, "ls-files", "database"])
            .output()
            .expect("git ls-files");
        let mut expected: Vec<String> = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .filter_map(|p| p.strip_prefix("database/"))
            .filter(|p| is_shipped(p))
            .map(str::to_string)
            .collect();
        expected.sort();
        let mut got: Vec<String> = names().iter().map(|s| s.to_string()).collect();
        got.sort();
        assert_eq!(got, expected);
        assert!(got.iter().any(|n| n == "design.yaml"));
        assert!(got.iter().any(|n| n.starts_with("ddl/")));
    }

    #[test]
    fn never_embeds_personal_capture_or_test_files() {
        for n in names() {
            assert!(!n.starts_with("tests/"), "test file embedded: {n}");
            assert!(!n.starts_with("backup/"), "backup embedded: {n}");
            assert_ne!(n, "import/staging/assistant_events.jsonl");
            assert_ne!(n, "import/staging/memories.jsonl");
        }
    }

    #[test]
    fn materialized_schema_loads_as_a_dbd_design() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("schema").join("v9.9.9");
        let at = materialize(&dest).unwrap();
        assert_eq!(at, dest);
        let design =
            dbd_core::Design::from_config_with_dir(&at.join("design.yaml"), "prod", Some(&at))
                .expect("embedded schema is a loadable dbd design");
        assert!(!design.entities().is_empty());
    }

    #[test]
    fn materialize_replaces_whatever_was_there() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("v1");
        std::fs::create_dir_all(dest.join("ddl")).unwrap();
        std::fs::write(dest.join("ddl").join("stale.ddl"), "drop table everything;").unwrap();
        materialize(&dest).unwrap();
        assert!(!dest.join("ddl").join("stale.ddl").exists());
        assert!(dest.join("design.yaml").is_file());
    }
}
