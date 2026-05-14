//! Simple append-only file logger → /tmp/sensei-bootstrap.log
//!
//! One line per event, always flushed. Safe to `tail -f` during a test run.
//! Written unconditionally — no feature flag needed, it's a debug artefact.

use std::fs::OpenOptions;
use std::io::Write;

const LOG_PATH: &str = "/tmp/sensei-bootstrap.log";

/// Append one timestamped line to /tmp/sensei-bootstrap.log.
pub fn log(msg: &str) {
    let ts = chrono::Utc::now().format("%H:%M:%S%.3f");
    let line = format!("[{ts}] {msg}\n");

    // Best-effort — never panic over logging
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(LOG_PATH) {
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
