//! Cross-platform utility functions for binary detection, version parsing,
//! port probing, and service health checks.
//!
//! Extracted from `homebrew.rs` and `service.rs` so callers don't need
//! Homebrew-specific code just to check whether a binary or service exists.

use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

use crate::types::{BootstrapTrace, TraceAction};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

// Alias the correct platform module so EXTRA_PATHS and enrich_path stay DRY.
#[cfg(not(target_os = "windows"))]
use crate::platform::macos as plat;
#[cfg(target_os = "windows")]
use crate::platform::windows as plat;

/// Build a PATH string that includes well-known platform directories.
/// Delegates to the current platform's enrichment logic.
/// Used when spawning child processes from an .app bundle where PATH is minimal.
pub fn enrich_path() -> String {
    plat::enrich_path()
}

/// Find a binary in PATH (and well-known locations).
///
/// Uses `which` on unix and `where` on windows. If that fails, checks
/// [`EXTRA_PATHS`] and `~/.local/bin` directly — this handles macOS .app
/// bundles where the process PATH doesn't include Homebrew or user-local bins.
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

    // Fallback: check well-known directories directly (platform-defined)
    for dir in plat::EXTRA_PATHS {
        let candidate = format!("{dir}/{name}");
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    // ~/.local/bin — used by `make install-dev` for dev builds.
    // Cannot be a static constant because it depends on $HOME.
    if let Ok(home) = std::env::var("HOME") {
        let candidate = format!("{home}/.local/bin/{name}");
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

/// Fetch the version string from a service's health endpoint.
///
/// Supported services:
/// - `"daemon"` — `GET http://127.0.0.1:{port}/health` → `json["version"]`
/// - `"ollama"` — `GET http://127.0.0.1:{port}/api/version` → `json["version"]`
/// - `"postgresql"` — `psql --version` → last whitespace token
pub fn fetch_service_version(name: &str, port: u16) -> Option<String> {
    match name {
        "daemon" | "ollama" => {
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .ok()?;
            let path = if name == "daemon" { "health" } else { "api/version" };
            let url = format!("http://127.0.0.1:{port}/{path}");
            let json: serde_json::Value = client.get(&url).send().ok()?.json().ok()?;
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
    fn version_parsing_no_digit_returns_none() {
        let parsed = parse_version_from_line("no digits here at all");
        assert_eq!(parsed, None);
    }

    #[test]
    fn version_parsing_empty_returns_none() {
        let parsed = parse_version_from_line("");
        assert_eq!(parsed, None);
    }

    // ── probe_port success ───────────────────────────────────────────────────

    #[test]
    fn probe_port_open() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(probe_port(port), "should connect to an actively listening port");
    }

    // ── probe_traced success path ────────────────────────────────────────────

    #[test]
    fn probe_traced_open_port() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let t = probe_traced("open_port_test", "Check open port", "127.0.0.1", port);
        assert!(t.ok, "open port probe should succeed");
        assert!(t.err.is_empty(), "no error on success");
        assert!(t.exit.is_none(), "TCP probes have no exit code");
    }

    // ── binary_version ───────────────────────────────────────────────────────

    #[test]
    fn binary_version_returns_some_for_real_binary() {
        // echo with a digit-containing arg gives us predictable output
        let v = binary_version("echo", "1.2.3");
        // echo outputs "1.2.3" which contains digits → parse_version_from_line finds it
        assert_eq!(v.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn binary_version_returns_none_for_nonexistent_binary() {
        let v = binary_version("sensei-totally-nonexistent-xyz", "--version");
        assert_eq!(v, None);
    }

    // ── enrich_path ─────────────────────────────────────────────────────────

    #[test]
    fn enrich_path_includes_extra_paths() {
        let p = enrich_path();
        // At least one of the platform's EXTRA_PATHS must appear (macOS/Linux only).
        let contains_extra = plat::EXTRA_PATHS.iter().any(|ep| p.contains(ep));
        // On Windows EXTRA_PATHS is empty, so this check is vacuously true.
        if !plat::EXTRA_PATHS.is_empty() {
            assert!(contains_extra, "enrich_path should include at least one known extra path");
        }
    }

    #[test]
    fn enrich_path_does_not_add_extra_paths_twice() {
        // enrich_path guarantees it won't append an EXTRA_PATH that's already present.
        let p = enrich_path();
        for extra in plat::EXTRA_PATHS {
            let count = p.split(plat::PATH_SEPARATOR).filter(|seg| *seg == *extra).count();
            assert!(
                count <= 1,
                "EXTRA_PATH '{extra}' should appear at most once in enrich_path output, found {count}"
            );
        }
    }

    // ── fetch_service_version ────────────────────────────────────────────────

    #[test]
    fn fetch_service_version_unknown_service_returns_none() {
        // Unknown service names fall through to the `_ => None` arm
        let result = fetch_service_version("unknown-service-xyz", 19998);
        assert_eq!(result, None);
    }

    #[test]
    fn fetch_service_version_daemon_unreachable_returns_none() {
        // Port 19997 is almost certainly not running our daemon
        let result = fetch_service_version("daemon", 19997);
        assert_eq!(result, None, "unreachable daemon port should yield None");
    }

    #[test]
    fn fetch_service_version_ollama_unreachable_returns_none() {
        let result = fetch_service_version("ollama", 19996);
        assert_eq!(result, None);
    }
}
