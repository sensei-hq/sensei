//! Service health checks — TCP port probes and process management.

use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

use crate::types::ComponentStatus;

/// Check if a service is reachable on the given port.
pub fn check(name: &str, port: u16) -> ComponentStatus {
    match probe_port(port) {
        true => {
            let version = fetch_version(name, port);
            ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"))
        }
        false => ComponentStatus::failed(name, &format!("not reachable on port {port}")),
    }
}

/// Start a brew service.
pub fn start_brew_service(name: &str) -> Result<ComponentStatus, String> {
    let brew = crate::homebrew::brew_path_pub()
        .ok_or_else(|| "homebrew not installed".to_string())?;

    let output = Command::new(&brew)
        .args(["services", "start", name])
        .output()
        .map_err(|e| format!("failed to start {name}: {e}"))?;

    if output.status.success() {
        Ok(ComponentStatus {
            name: name.to_string(),
            state: crate::types::ComponentState::Starting,
            version: None,
            detail: Some("service started, waiting for port".into()),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("brew services start {name} failed: {stderr}"))
    }
}

/// Start the sensei daemon.
pub fn start_daemon(port: u16) -> Result<ComponentStatus, String> {
    let binary = which("senseid").ok_or("senseid binary not found in PATH")?;

    Command::new(&binary)
        .args(["serve", "--port", &port.to_string()])
        .spawn()
        .map_err(|e| format!("failed to start senseid: {e}"))?;

    // Give it a moment to bind
    std::thread::sleep(Duration::from_millis(500));

    if probe_port(port) {
        let version = fetch_version("daemon", port);
        Ok(ComponentStatus::ready("daemon", version.as_deref().unwrap_or("unknown")))
    } else {
        Ok(ComponentStatus {
            name: "daemon".to_string(),
            state: crate::types::ComponentState::Starting,
            version: None,
            detail: Some("started but not yet responding".into()),
        })
    }
}

/// Probe a TCP port (connect with 2s timeout).
fn probe_port(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_secs(2),
    ).is_ok()
}

/// Fetch version from a service's health endpoint.
fn fetch_version(name: &str, port: u16) -> Option<String> {
    match name {
        "daemon" => {
            let url = format!("http://127.0.0.1:{port}/health");
            let resp = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build().ok()?
                .get(&url).send().ok()?;
            let json: serde_json::Value = resp.json().ok()?;
            json["version"].as_str().map(|s| s.to_string())
        }
        "ollama" => {
            let url = format!("http://127.0.0.1:{port}/api/version");
            let resp = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build().ok()?
                .get(&url).send().ok()?;
            let json: serde_json::Value = resp.json().ok()?;
            json["version"].as_str().map(|s| s.to_string())
        }
        "postgresql" => {
            // pg_isready doesn't return version; use psql
            let output = Command::new("psql")
                .args(["--version"])
                .output().ok()?;
            let text = String::from_utf8_lossy(&output.stdout);
            // "psql (PostgreSQL) 17.2" → "17.2"
            text.split_whitespace().last().map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Find a binary in PATH.
fn which(name: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_port_closed() {
        // Port 1 is almost certainly not in use
        assert!(!probe_port(1));
    }

    #[test]
    fn which_finds_existing_binary() {
        let result = which("ls");
        assert!(result.is_some(), "should find ls in PATH");
    }

    #[test]
    fn which_returns_none_for_nonexistent() {
        let result = which("sensei-nonexistent-binary-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn check_unreachable_service() {
        let status = check("test-service", 19999);
        assert!(status.is_failed());
        assert_eq!(status.name, "test-service");
    }

    #[test]
    fn psql_version_parsing() {
        let fake = "psql (PostgreSQL) 17.2";
        let parsed = fake.split_whitespace().last();
        assert_eq!(parsed, Some("17.2"));
    }
}
