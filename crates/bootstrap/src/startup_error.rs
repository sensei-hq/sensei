//! Why the daemon last failed to start, in `<sensei_dir>/startup-error.log` (#179).
//!
//! One owner for the path, shared by the daemon (which records it on every exit
//! before it binds) and `senseid status` (which shows it). A daemon run under
//! launchd has no stderr anyone reads, so this file is the only place the reason
//! survives. Cleared once the daemon is up.
use std::path::{Path, PathBuf};

const FILE: &str = "startup-error.log";

fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// Record `msg` as the last startup failure. Best-effort: the caller is already
/// failing, and a write that fails must not replace its reason with another.
pub fn record_in(dir: &Path, msg: &str) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(path_in(dir), msg);
}

/// The last recorded startup failure, if any.
pub fn last_in(dir: &Path) -> Option<String> {
    std::fs::read_to_string(path_in(dir))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn clear_in(dir: &Path) {
    let _ = std::fs::remove_file(path_in(dir));
}

fn dir() -> PathBuf {
    crate::SenseiConfig::from_env().sensei_dir()
}

pub fn record(msg: &str) {
    record_in(&dir(), msg);
}

pub fn last() -> Option<String> {
    last_in(&dir())
}

pub fn clear() {
    clear_in(&dir());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorded_reason_reads_back_until_cleared() {
        let t = tempfile::tempdir().unwrap();
        assert_eq!(last_in(t.path()), None);
        record_in(t.path(), "could not bind :7744\n");
        assert_eq!(last_in(t.path()).as_deref(), Some("could not bind :7744"));
        clear_in(t.path());
        assert_eq!(last_in(t.path()), None);
    }
}
