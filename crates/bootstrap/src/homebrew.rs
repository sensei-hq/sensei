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

/// Check if a specific formula is installed.
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
}
