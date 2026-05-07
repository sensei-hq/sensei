//! Cross-platform utility functions for binary detection, version parsing,
//! port probing, and service health checks.
//!
//! Extracted from `homebrew.rs` and `service.rs` so callers don't need
//! Homebrew-specific code just to check whether a binary or service exists.

use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

use crate::types::{BootstrapTrace, ComponentState, ComponentStatus, TraceAction};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Build a PATH string that includes well-known directories.
/// Used when spawning child processes from a macOS .app bundle
/// where the inherited PATH is minimal.
pub fn enrich_path() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let mut parts: Vec<&str> = current.split(':').collect();
    for extra in EXTRA_PATHS {
        if !parts.contains(extra) {
            parts.push(extra);
        }
    }
    parts.join(":")
}

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
    let output = Command::new(binary).arg(flag).env("PATH", enrich_path()).output().ok()?;
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

    // Build a PATH that includes Homebrew so the daemon can find psql, etc.
    let enriched_path = enrich_path();

    // Use `senseid start` which daemonizes itself.
    let output = Command::new(&binary)
        .args(["start", "--port", &port.to_string()])
        .env("PATH", &enriched_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("failed to start senseid: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("senseid start failed: {stderr}"));
    }

    // Give it a moment to bind
    std::thread::sleep(Duration::from_millis(2000));

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

/// Generate a monotonically increasing trace ID.
pub fn next_trace_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("trace-{:08x}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// Current time as an ISO 8601 string.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Run a shell command and return a fully-populated BootstrapTrace.
/// No file I/O or side effects beyond the atomic trace-ID counter.
pub fn run_traced(step: &str, desc: &str, cmd: &str, args: &[&str]) -> BootstrapTrace {
    let ts = now_iso();
    let start = Instant::now();
    let result = std::process::Command::new(cmd)
        .args(args)
        .env("PATH", enrich_path())
        .output();
    let ms = start.elapsed().as_millis() as u64;
    let cmd_str = format!("{} {}", cmd, args.join(" ")).trim().to_string();

    match result {
        Ok(output) => BootstrapTrace {
            id:            next_trace_id(),
            ts,
            action_type:   TraceAction::Check,
            step:          step.to_string(),
            desc:          desc.to_string(),
            cmd:           cmd_str,
            exit:          output.status.code(),
            out:           String::from_utf8_lossy(&output.stdout).trim().to_string(),
            err:           String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ms,
            ok:            output.status.success(),
            fix_attempted: false,
            fix_approach:  None,
            fix_ok:        None,
        },
        Err(e) => BootstrapTrace {
            id:            next_trace_id(),
            ts,
            action_type:   TraceAction::Check,
            step:          step.to_string(),
            desc:          desc.to_string(),
            cmd:           cmd_str,
            exit:          Some(-1),
            out:           String::new(),
            err:           e.to_string(),
            ms,
            ok:            false,
            fix_attempted: false,
            fix_approach:  None,
            fix_ok:        None,
        },
    }
}

/// Probe a TCP port and return a BootstrapTrace with a synthetic command string.
/// No file I/O or side effects beyond the atomic trace-ID counter.
pub fn probe_traced(step: &str, desc: &str, host: &str, port: u16) -> BootstrapTrace {
    let ts = now_iso();
    let cmd = format!("tcp probe {host}:{port}");
    let start = Instant::now();
    let addr_str = format!("{host}:{port}");
    let addr = match addr_str.parse() {
        Ok(a) => a,
        Err(e) => {
            return BootstrapTrace {
                id:            next_trace_id(),
                ts,
                action_type:   TraceAction::Check,
                step:          step.to_string(),
                desc:          desc.to_string(),
                cmd,
                exit:          None,
                out:           String::new(),
                err:           format!("invalid address {addr_str}: {e}"),
                ms:            start.elapsed().as_millis() as u64,
                ok:            false,
                fix_attempted: false,
                fix_approach:  None,
                fix_ok:        None,
            };
        }
    };
    let ok = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok();
    let ms = start.elapsed().as_millis() as u64;

    BootstrapTrace {
        id:            next_trace_id(),
        ts,
        action_type:   TraceAction::Check,
        step:          step.to_string(),
        desc:          desc.to_string(),
        cmd,
        exit:          None,
        out:           String::new(),
        err:           if ok { String::new() } else { format!("port {port} not reachable") },
        ms,
        ok,
        fix_attempted: false,
        fix_approach:  None,
        fix_ok:        None,
    }
}

/// Check if a binary exists, returning status and traces. Pure function.
pub fn check_binary_traced(
    name: &str,
    binary: &str,
    version_flag: &str,
) -> (ComponentStatus, Vec<BootstrapTrace>) {
    #[cfg(unix)]
    let which_cmd = "which";
    #[cfg(windows)]
    let which_cmd = "where";

    let which_t = run_traced(
        &format!("{name}_which"),
        &format!("Locate {name} binary"),
        which_cmd,
        &[binary],
    );

    let path = which_binary(binary);
    if path.is_none() {
        let mut t = which_t;
        t.ok = false;
        t.err = format!("{binary} not found in PATH or known directories");
        return (ComponentStatus::missing(name), vec![t]);
    }
    let path = path.unwrap();

    let version_t = run_traced(
        &format!("{name}_version"),
        &format!("Check {name} version"),
        binary,
        &[version_flag],
    );
    let version = parse_version_from_line(&version_t.out);

    // If which command failed (EXTRA_PATHS fallback found it), fix the trace
    let mut which_final = which_t;
    if !which_final.ok {
        which_final.ok = true;
        which_final.out = path.clone();
    }

    let mut status = ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"));
    status.detail = Some(path);
    (status, vec![which_final, version_t])
}

/// Check binary + service port, returning status and traces. Pure function.
pub fn check_binary_and_service_traced(
    name: &str,
    binary: &str,
    _version_flag: &str,
    port: u16,
) -> (ComponentStatus, Vec<BootstrapTrace>) {
    #[cfg(unix)]
    let which_cmd = "which";
    #[cfg(windows)]
    let which_cmd = "where";

    let which_t = run_traced(
        &format!("{name}_which"),
        &format!("Locate {name} binary"),
        which_cmd,
        &[binary],
    );

    let path = which_binary(binary);
    if path.is_none() {
        let mut t = which_t;
        t.ok = false;
        t.err = format!("{binary} not found in PATH or known directories");
        return (ComponentStatus::missing(name), vec![t]);
    }
    let path = path.unwrap();

    let mut which_final = which_t;
    if !which_final.ok {
        which_final.ok = true;
        which_final.out = path.clone();
    }

    let port_t = probe_traced(
        &format!("{name}_port"),
        &format!("Check {name} service on port {port}"),
        "127.0.0.1",
        port,
    );
    let port_ok = port_t.ok;
    let traces = vec![which_final, port_t];

    if !port_ok {
        let mut status = ComponentStatus::failed(
            name,
            &format!("installed at {path} but service not running on port {port}"),
        );
        status.detail = Some(path);
        return (status, traces);
    }

    let svc_version = fetch_service_version(name, port);
    let mut status = ComponentStatus::ready(
        name,
        svc_version.as_deref().unwrap_or("unknown"),
    );
    status.detail = Some(path);
    (status, traces)
}

/// Check a service port only, returning status and a trace. Pure function.
pub fn check_service_traced(name: &str, port: u16) -> (ComponentStatus, Vec<BootstrapTrace>) {
    let port_t = probe_traced(
        &format!("{name}_port"),
        &format!("Check {name} on port {port}"),
        "127.0.0.1",
        port,
    );
    if port_t.ok {
        let version = fetch_service_version(name, port);
        let status = ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"));
        (status, vec![port_t])
    } else {
        let status = ComponentStatus::failed(name, &format!("not reachable on port {port}"));
        (status, vec![port_t])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_traced_captures_echo() {
        let t = run_traced("echo_test", "Echo hello", "echo", &["hello"]);
        assert!(t.ok, "echo should succeed");
        assert_eq!(t.out, "hello");
        assert_eq!(t.err, "");
        assert!(t.exit == Some(0));
        assert_eq!(t.step, "echo_test");
        assert!(t.ms < 5000, "should complete in under 5 seconds");
        assert!(!t.id.is_empty());
        assert!(!t.ts.is_empty());
    }

    #[test]
    fn run_traced_captures_failure() {
        let t = run_traced("bad_cmd", "Run nonexistent", "sensei-nonexistent-xyz-binary", &[]);
        assert!(!t.ok);
        assert!(!t.err.is_empty(), "error should capture the failure reason");
        assert_eq!(t.exit, Some(-1), "spawn failure should use -1 sentinel");
    }

    #[test]
    fn probe_traced_closed_port() {
        let t = probe_traced("port_check", "Check port 1", "127.0.0.1", 1);
        assert!(!t.ok);
        assert!(t.exit.is_none(), "TCP probes have no exit code");
        assert!(t.cmd.contains("tcp probe"));
        assert!(t.cmd.contains("127.0.0.1:1"));
        assert!(t.err.contains("not reachable"), "should populate err when port closed");
    }

    #[test]
    fn next_trace_id_is_unique() {
        let a = next_trace_id();
        let b = next_trace_id();
        assert_ne!(a, b);
        assert!(a.starts_with("trace-"));
    }

    #[test]
    fn now_iso_looks_like_rfc3339() {
        let ts = now_iso();
        assert!(ts.contains('T'), "should have T separator");
        assert!(ts.ends_with('Z') || ts.contains('+'), "should have timezone");
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

    #[test]
    fn check_binary_traced_finds_ls() {
        let (status, traces) = check_binary_traced("ls", "ls", "--help");
        // ls exists everywhere — expect ready
        assert!(status.is_ready(), "ls should be found");
        assert!(!traces.is_empty(), "should have at least one trace");
        assert!(traces.iter().any(|t| t.step.contains("ls")));
    }

    #[test]
    fn check_binary_traced_missing() {
        let (status, traces) = check_binary_traced(
            "nope", "sensei-nonexistent-xyz", "--version"
        );
        assert!(status.is_failed(), "nonexistent binary should fail");
        assert!(!traces.is_empty());
    }

    #[test]
    fn check_service_traced_closed_port() {
        let (status, traces) = check_service_traced("nope_service", 1);
        assert!(status.is_failed());
        assert!(!traces.is_empty());
        assert!(traces.iter().any(|t| t.cmd.contains("tcp probe")));
    }

    #[test]
    fn check_binary_and_service_traced_missing_binary() {
        let (status, traces) = check_binary_and_service_traced(
            "nope", "sensei-nonexistent-xyz", "--version", 1
        );
        assert!(status.is_failed());
        assert!(!traces.is_empty());
    }
}
