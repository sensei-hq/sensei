//! Simple append-only file logger → `/tmp/sensei-bootstrap.log` (the temp dir
//! on Windows, which has no `/tmp`).
//!
//! One line per event, always flushed. Safe to `tail -f` during a test run.
//! Written unconditionally — no feature flag needed, it's a debug artefact.

use std::fs::OpenOptions;
use std::io::Write;

/// The bootstrap log: `/tmp` on Unix, kept fixed so `tail -f` finds it (macOS's
/// `TMPDIR` is a per-user `/var/folders/…` path); the temp dir on Windows.
pub fn log_path() -> std::path::PathBuf {
    let dir = if cfg!(unix) { std::path::PathBuf::from("/tmp") } else { std::env::temp_dir() };
    dir.join("sensei-bootstrap.log")
}

/// Append one timestamped line to the bootstrap log ([`log_path`]).
pub fn log(msg: &str) {
    let ts = chrono::Utc::now().format("%H:%M:%S%.3f");
    let line = format!("[{ts}] {msg}\n");

    // Best-effort — never panic over logging
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(log_path()) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Convenience macro — same syntax as eprintln! but goes to the log file.
#[macro_export]
macro_rules! flog {
    ($($arg:tt)*) => {
        $crate::flog::log(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn log_path_is_tmp_on_unix_and_the_temp_dir_on_windows() {
        let p = super::log_path();
        assert_eq!(p.file_name().unwrap(), "sensei-bootstrap.log");
        if cfg!(unix) {
            assert_eq!(p, std::path::Path::new("/tmp/sensei-bootstrap.log"));
        } else {
            assert_eq!(p.parent().unwrap(), std::env::temp_dir());
        }
    }
}
