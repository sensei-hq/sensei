//! Homebrew detection and formula management.

use std::process::Command;

use crate::types::ComponentStatus;

const BREW_PATH: &str = "/opt/homebrew/bin/brew";
const BREW_PATH_INTEL: &str = "/usr/local/bin/brew";

/// Check if Homebrew is installed.
pub fn check() -> ComponentStatus {
    let path = brew_path();
    match path {
        Some(p) => match version(&p) {
            Some(v) => ComponentStatus::ready("homebrew", &v),
            None => ComponentStatus::failed("homebrew", "installed but version check failed"),
        },
        None => ComponentStatus::missing("homebrew"),
    }
}

/// Check if a specific formula is installed via Homebrew.
pub fn check_formula(name: &str) -> ComponentStatus {
    let brew = match brew_path() {
        Some(p) => p,
        None => return ComponentStatus::failed(name, "homebrew not installed"),
    };

    let output = Command::new(&brew)
        .args(["list", "--versions", name])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let ver = text.split_whitespace().nth(1).unwrap_or("unknown");
            ComponentStatus::ready(name, ver)
        }
        _ => ComponentStatus::missing(name),
    }
}

/// Check if a binary exists in PATH and report its version.
///
/// Uses `which <binary>` to detect presence (works regardless of install method),
/// then `<binary> <version_flag>` to extract the version string.
/// The binary path is stored in `detail` — a `/opt/homebrew/` prefix indicates
/// a Homebrew install; anything else means it was installed another way.
pub fn check_binary(name: &str, binary: &str, version_flag: &str) -> ComponentStatus {
    let path = match which_binary(binary) {
        Some(p) => p,
        None => return ComponentStatus::missing(name),
    };

    let version = binary_version(binary, version_flag);
    let mut status = ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"));
    status.detail = Some(path);
    status
}

/// Find a binary in PATH.
fn which_binary(name: &str) -> Option<String> {
    let output = Command::new("which")
        .arg(name)
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Run `<binary> <flag>` and extract a version string from the output.
fn binary_version(binary: &str, flag: &str) -> Option<String> {
    let output = Command::new(binary)
        .arg(flag)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let first_line = text.lines().next()?;
    // Extract version-like token: last word, or word containing digits and dots
    first_line
        .split_whitespace()
        .rev()
        .find(|w| w.chars().any(|c| c.is_ascii_digit()))
        .map(|v| v.to_string())
}

/// Install a formula via brew.
pub fn install_formula(name: &str) -> Result<ComponentStatus, String> {
    let brew = brew_path().ok_or("homebrew not installed")?;

    let output = Command::new(&brew)
        .args(["install", name])
        .output()
        .map_err(|e| format!("failed to run brew install: {e}"))?;

    if output.status.success() {
        Ok(check_formula(name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("brew install {name} failed: {stderr}"))
    }
}

/// Upgrade a formula via brew.
pub fn upgrade_formula(name: &str) -> Result<ComponentStatus, String> {
    let brew = brew_path().ok_or("homebrew not installed")?;

    let output = Command::new(&brew)
        .args(["upgrade", name])
        .output()
        .map_err(|e| format!("failed to run brew upgrade: {e}"))?;

    if output.status.success() {
        Ok(check_formula(name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("brew upgrade {name} failed: {stderr}"))
    }
}

/// Find the brew binary path (public for service module).
pub(crate) fn brew_path_pub() -> Option<String> { brew_path() }

/// Find the brew binary path.
fn brew_path() -> Option<String> {
    if std::path::Path::new(BREW_PATH).exists() {
        Some(BREW_PATH.to_string())
    } else if std::path::Path::new(BREW_PATH_INTEL).exists() {
        Some(BREW_PATH_INTEL.to_string())
    } else {
        None
    }
}

/// Get Homebrew version.
fn version(brew_path: &str) -> Option<String> {
    let output = Command::new(brew_path)
        .args(["--version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // "Homebrew 4.4.2\n..." → "4.4.2"
    text.lines()
        .next()
        .and_then(|l| l.strip_prefix("Homebrew "))
        .map(|v| v.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brew_path_returns_some_on_macos_with_brew() {
        // This test only makes sense on a dev machine with Homebrew
        let path = brew_path();
        if std::path::Path::new(BREW_PATH).exists() || std::path::Path::new(BREW_PATH_INTEL).exists() {
            assert!(path.is_some());
        }
    }

    #[test]
    fn check_returns_ready_or_missing() {
        let status = check();
        assert!(status.is_ready() || status.is_failed());
        assert_eq!(status.name, "homebrew");
    }

    #[test]
    fn check_formula_nonexistent() {
        // A formula that definitely doesn't exist
        let status = check_formula("sensei-nonexistent-test-package-xyz");
        assert!(status.is_failed());
    }

    #[test]
    fn check_formula_sets_name() {
        let status = check_formula("git");
        assert_eq!(status.name, "git");
        // git may or may not be installed via brew, so we just check the name
    }

    #[test]
    fn version_parsing() {
        // Test the version extraction logic without running brew
        let fake_output = "Homebrew 4.4.2\nHomebrew/homebrew-core";
        let parsed = fake_output.lines()
            .next()
            .and_then(|l| l.strip_prefix("Homebrew "))
            .map(|v| v.trim().to_string());
        assert_eq!(parsed, Some("4.4.2".to_string()));
    }

    #[test]
    fn which_binary_finds_ls() {
        let result = which_binary("ls");
        assert!(result.is_some(), "should find ls in PATH");
    }

    #[test]
    fn which_binary_returns_none_for_nonexistent() {
        let result = which_binary("sensei-nonexistent-binary-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn check_binary_sets_name_and_detail() {
        // ls exists everywhere — check that name and detail (path) are set
        let status = check_binary("test-ls", "ls", "--version");
        assert_eq!(status.name, "test-ls");
        // detail should contain the binary path
        assert!(status.detail.is_some(), "detail should contain binary path");
    }

    #[test]
    fn check_binary_nonexistent() {
        let status = check_binary("nope", "sensei-nonexistent-binary-xyz", "--version");
        assert!(status.is_failed());
        assert_eq!(status.name, "nope");
    }

    #[test]
    fn binary_version_parsing_postgres() {
        // Simulates: "postgres (PostgreSQL) 17.2"
        let line = "postgres (PostgreSQL) 17.2";
        let parsed = line
            .split_whitespace()
            .rev()
            .find(|w| w.chars().any(|c| c.is_ascii_digit()))
            .map(|v| v.to_string());
        assert_eq!(parsed, Some("17.2".to_string()));
    }

    #[test]
    fn binary_version_parsing_ollama() {
        // Simulates: "ollama version 0.5.4"
        let line = "ollama version 0.5.4";
        let parsed = line
            .split_whitespace()
            .rev()
            .find(|w| w.chars().any(|c| c.is_ascii_digit()))
            .map(|v| v.to_string());
        assert_eq!(parsed, Some("0.5.4".to_string()));
    }
}
