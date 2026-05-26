//! Cross-platform binary discovery.
//!
//! This module exposes a single helper — `which_binary` — used by every
//! sensei consumer (CLI, daemon, MCP, Tauri sidecar). It looks up a binary
//! on PATH, then falls back to well-known package-manager directories so
//! Tauri's macOS .app bundle (which has a narrow process PATH) can still
//! find Homebrew-installed binaries.

use std::process::Command;

/// Directories searched when `which`/`where` lookup misses. These cover the
/// standard Homebrew prefixes on macOS / Linux and the user-local install
/// directory used by `make install-dev`.
const EXTRA_BIN_DIRS: &[&str] = &[
    "/opt/homebrew/bin",        // macOS Apple Silicon Homebrew
    "/usr/local/bin",           // macOS Intel Homebrew, common Linux
    "/home/linuxbrew/.linuxbrew/bin", // Linuxbrew on dedicated user
];

/// Find a binary on PATH (and well-known directories).
///
/// Uses `which` on Unix and `where` on Windows; falls back to scanning
/// `EXTRA_BIN_DIRS` and `~/.local/bin` directly. Returns the full path
/// if found, `None` otherwise.
pub fn which_binary(name: &str) -> Option<String> {
    #[cfg(unix)]
    let cmd = "which";
    #[cfg(windows)]
    let cmd = "where";

    if let Ok(output) = Command::new(cmd).arg(name).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    for dir in EXTRA_BIN_DIRS {
        let candidate = format!("{dir}/{name}");
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let candidate = format!("{home}/.local/bin/{name}");
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_binary_returns_none_for_missing() {
        assert!(which_binary("definitely_not_a_real_binary_xyz_77").is_none());
    }

    #[test]
    fn which_binary_finds_ls_on_unix() {
        #[cfg(unix)]
        {
            let p = which_binary("ls").expect("ls must exist on unix");
            assert!(p.ends_with("/ls") || p.ends_with("/bin/ls"));
        }
    }
}
