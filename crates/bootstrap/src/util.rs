//! Cross-platform utility functions for binary detection, version parsing,
//! port probing, and service health checks.
//!
//! Extracted from `homebrew.rs` and `service.rs` so callers don't need
//! Homebrew-specific code just to check whether a binary or service exists.

use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

use crate::types::{ComponentState, ComponentStatus};

/// Well-known binary directories to search when PATH is limited.
/// macOS .app bundles inherit a minimal PATH that excludes Homebrew.
const EXTRA_PATHS: &[&str] = &[
    "/opt/homebrew/bin",
    "/opt/homebrew/sbin",
    "/usr/local/bin",
    "/usr/local/sbin",
];

/// Find a binary in PATH (and well-known locations).
///
/// Uses `which` on unix and `where` on windows. If that fails, checks
/// [`EXTRA_PATHS`] directly — this handles macOS .app bundles where the
/// process PATH doesn't include `/opt/homebrew/bin`.
///
/// Returns the full path if the binary is found, `None` otherwise.
pub fn which_binary(name: &str) -> Option<String> {
    // Try system which/where first
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

    // Fallback: check well-known directories directly
    for dir in EXTRA_PATHS {
        let candidate = format!("{dir}/{name}");
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    None
}

/// Run `<binary> <flag>` and extract a version string from the first line.
///
/// Scans tokens right-to-left for the first one containing a digit.
/// For example `"postgres (PostgreSQL) 17.2"` yields `"17.2"`.
pub fn binary_version(binary: &str, flag: &str) -> Option<String> {
    let output = Command::new(binary).arg(flag).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_version_from_line(text.lines().next()?)
}

/// Pure parsing helper — find a version-like token in a single line.
fn parse_version_from_line(line: &str) -> Option<String> {
    line.split_whitespace()
        .rev()
        .find(|w| w.chars().any(|c| c.is_ascii_digit()))
        .map(|v| v.to_string())
}

/// Check if a binary exists AND its service is running on a port.
///
/// Returns `missing` if the binary isn't found, `failed` if the binary
/// exists but the port isn't responsive, and `ready` if both pass.
/// This produces a single status for components that have both a binary
/// and a service (e.g. postgresql, ollama).
pub fn check_binary_and_service(
    name: &str,
    binary: &str,
    version_flag: &str,
    port: u16,
) -> ComponentStatus {
    // Binary must exist first
    let path = match which_binary(binary) {
        Some(p) => p,
        None => return ComponentStatus::missing(name),
    };

    let version = binary_version(binary, version_flag);

    // Then check the service port
    if !probe_port(port) {
        let mut status = ComponentStatus::failed(
            name,
            &format!("installed at {} but service not running on port {}", path, port),
        );
        status.version = version;
        status.detail = Some(path);
        return status;
    }

    // Both binary and service are good
    let svc_version = fetch_service_version(name, port);
    let mut status = ComponentStatus::ready(
        name,
        svc_version.as_deref().or(version.as_deref()).unwrap_or("unknown"),
    );
    status.detail = Some(path);
    status
}

/// Check if a binary exists in PATH and report its version.
///
/// Combines [`which_binary`] + [`binary_version`].
/// The resolved path is stored in `detail`.
/// Returns a `missing` status if the binary is not found.
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

/// TCP connect to 127.0.0.1:`port` with a 2-second timeout.
///
/// Returns `true` if the connection succeeds (something is listening).
pub fn probe_port(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

/// Check whether a service is reachable on the given port.
///
/// Probes the port, then tries to fetch the service version via
/// [`fetch_service_version`]. Returns `ready` or `failed`.
pub fn check_service(name: &str, port: u16) -> ComponentStatus {
    if probe_port(port) {
        let version = fetch_service_version(name, port);
        ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"))
    } else {
        ComponentStatus::failed(name, &format!("not reachable on port {port}"))
    }
}

/// Fetch the version string from a service's health endpoint.
///
/// Supported services:
/// - `"daemon"` — `GET http://127.0.0.1:{port}/health` → `json["version"]`
/// - `"ollama"` — `GET http://127.0.0.1:{port}/api/version` → `json["version"]`
/// - `"postgresql"` — `psql --version` → last whitespace token
pub fn fetch_service_version(name: &str, port: u16) -> Option<String> {
    match name {
        "daemon" => {
            let url = format!("http://127.0.0.1:{port}/health");
            let resp = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .ok()?
                .get(&url)
                .send()
                .ok()?;
            let json: serde_json::Value = resp.json().ok()?;
            json["version"].as_str().map(|s| s.to_string())
        }
        "ollama" => {
            let url = format!("http://127.0.0.1:{port}/api/version");
            let resp = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .ok()?
                .get(&url)
                .send()
                .ok()?;
            let json: serde_json::Value = resp.json().ok()?;
            json["version"].as_str().map(|s| s.to_string())
        }
        "postgresql" => {
            let output = Command::new("psql").args(["--version"]).output().ok()?;
            let text = String::from_utf8_lossy(&output.stdout);
            text.split_whitespace().last().map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Start the sensei daemon on the given port.
///
/// Locates `senseid` via [`which_binary`], spawns `senseid --port {port}`,
/// waits 500ms, then probes the port. Returns `ready` if the port responds,
/// or a `Starting` state if the daemon was spawned but hasn't bound yet.
pub fn start_daemon(port: u16) -> Result<ComponentStatus, String> {
    let binary = which_binary("senseid").ok_or("senseid binary not found in PATH")?;

    Command::new(&binary)
        .args(["--port", &port.to_string()])
        .spawn()
        .map_err(|e| format!("failed to start senseid: {e}"))?;

    // Give it a moment to bind
    std::thread::sleep(Duration::from_millis(500));

    if probe_port(port) {
        let version = fetch_service_version("daemon", port);
        Ok(ComponentStatus::ready(
            "daemon",
            version.as_deref().unwrap_or("unknown"),
        ))
    } else {
        Ok(ComponentStatus {
            name: "daemon".to_string(),
            state: ComponentState::Starting,
            version: None,
            detail: Some("started but not yet responding".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn probe_port_closed() {
        // Port 1 is almost certainly not in use
        assert!(!probe_port(1));
    }

    #[test]
    fn check_binary_nonexistent() {
        let status = check_binary("nope", "sensei-nonexistent-binary-xyz", "--version");
        assert!(status.is_failed());
        assert_eq!(status.name, "nope");
    }

    #[test]
    fn version_parsing_postgres() {
        let parsed = parse_version_from_line("postgres (PostgreSQL) 17.2");
        assert_eq!(parsed, Some("17.2".to_string()));
    }

    #[test]
    fn version_parsing_ollama() {
        let parsed = parse_version_from_line("ollama version 0.5.4");
        assert_eq!(parsed, Some("0.5.4".to_string()));
    }
}
