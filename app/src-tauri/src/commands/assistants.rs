//! Assistant detection and configuration — thin proxy to the daemon API.
//!
//! The Tauri sidecar owns bootstrap/health/upgrade only.
//! All assistant logic (detection, configuration, removal) is daemon-owned.
//! These commands are pure passthroughs — no types are duplicated here.

use serde_json::{json, Value};

/// Daemon base URL — derived from `SenseiConfig::from_env()` (the
/// single source of truth in `sensei-bootstrap`). Never hardcode the
/// port or the prod/dev distinction here: previously this file used
/// `cfg!(debug_assertions)` to talk to port 7745 in debug builds,
/// which broke silently when the dev/prod split was ripped out (port
/// is 7744 unconditionally now and nothing listens on 7745).
fn daemon_url() -> String {
    sensei_bootstrap::SenseiConfig::from_env().daemon_url()
}

/// Returns the daemon's full assistant list. Caller receives raw daemon JSON.
/// Fails closed: a daemon-unreachable or unparseable response is an `Err`, never
/// an empty list (which would read as "no assistants detected"). See #109 audit.
#[tauri::command]
pub fn detect_assistants() -> Result<Vec<Value>, String> {
    fetch_assistants(&daemon_url(), "assistants")
}

#[tauri::command]
pub fn configure_mcp(assistants: Vec<String>) -> Result<Vec<String>, String> {
    configure_at(&daemon_url(), assistants)
}

/// Returns the daemon's full assistant status list. Caller receives raw daemon JSON.
/// Fails closed like [`detect_assistants`]: unreachable/unparseable → `Err`, not [].
#[tauri::command]
pub fn check_assistant_configs() -> Result<Vec<Value>, String> {
    fetch_assistants(&daemon_url(), "assistant configs")
}

/// The daemon is local, so no proxy: ureq 3's default agent reads HTTP_PROXY
/// and friends from the environment, which ureq 2 (used before) never did.
fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().proxy(None).build().new_agent()
}

/// `GET {base}/api/assistants/detect`; `what` names the list in a parse error.
fn fetch_assistants(base: &str, what: &str) -> Result<Vec<Value>, String> {
    match agent().get(&format!("{base}/api/assistants/detect")).call() {
        Ok(resp) => resp.into_body().read_json().map_err(|e| format!("parse {what}: {e}")),
        Err(e) => Err(format!("Daemon not available: {e}")),
    }
}

fn configure_at(base: &str, assistants: Vec<String>) -> Result<Vec<String>, String> {
    let body = json!({"acps": assistants});
    match agent().post(&format!("{base}/api/assistants/configure")).send_json(&body) {
        Ok(resp) => {
            // A malformed 200 body must error, not silently become {} → an empty
            // "configured nothing, no errors" success (#109 audit).
            let result: Value = resp
                .into_body()
                .read_json()
                .map_err(|e| format!("parse configure response: {e}"))?;
            configured_or_errors(&result)
        }
        Err(e) => Err(format!("Daemon not available: {e}")),
    }
}

/// What the daemon configured; its errors only when it configured nothing.
fn configured_or_errors(result: &Value) -> Result<Vec<String>, String> {
    let configured: Vec<String> = result["configured"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if let Some(errors) = result["errors"].as_array() {
        if !errors.is_empty() && configured.is_empty() {
            let err_msgs: Vec<&str> = errors.iter().filter_map(|e| e.as_str()).collect();
            return Err(err_msgs.join(", "));
        }
    }
    Ok(configured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A one-request daemon on a free port: answers `status` with `body`.
    /// Returns its base URL and a handle yielding the request it received.
    fn daemon(status: &str, body: &str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut seen = Vec::new();
            let mut buf = [0u8; 4096];
            // Read the head, then as much body as Content-Length announces.
            loop {
                let n = stream.read(&mut buf).unwrap();
                seen.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&seen).to_string();
                if let Some(end) = text.find("\r\n\r\n") {
                    let len = text
                        .lines()
                        .find_map(|l| {
                            l.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|v| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if seen.len() >= end + 4 + len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            stream.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&seen).to_string()
        });
        (base, handle)
    }

    #[test]
    fn a_detect_list_is_returned_as_the_daemon_sent_it() {
        let (base, req) = daemon("200 OK", r#"[{"id":"claude"},{"id":"zed"}]"#);
        let got = fetch_assistants(&base, "assistants").unwrap();
        assert_eq!(got, vec![json!({"id":"claude"}), json!({"id":"zed"})]);
        assert!(req.join().unwrap().starts_with("GET /api/assistants/detect "));
    }

    /// ureq 3 reads HTTP_PROXY by default (ureq 2 did not). The daemon is on
    /// this machine, so a proxy in the environment must not sit between them.
    #[test]
    fn a_proxy_in_the_environment_is_not_used_for_the_daemon() {
        let (base, _req) = daemon("200 OK", "[]");
        // A proxy that refuses every connection: a call routed through it fails.
        std::env::set_var("HTTP_PROXY", "http://127.0.0.1:9");
        std::env::set_var("http_proxy", "http://127.0.0.1:9");
        let got = fetch_assistants(&base, "assistants");
        std::env::remove_var("HTTP_PROXY");
        std::env::remove_var("http_proxy");
        assert_eq!(got.unwrap(), Vec::<Value>::new());
    }

    #[test]
    fn an_unparseable_detect_body_is_an_error_not_an_empty_list() {
        let (base, _req) = daemon("200 OK", "not json");
        let err = fetch_assistants(&base, "assistants").unwrap_err();
        assert!(err.starts_with("parse assistants:"), "{err}");
    }

    #[test]
    fn a_daemon_error_status_is_an_error() {
        let (base, _req) = daemon("500 Internal Server Error", "{}");
        let err = fetch_assistants(&base, "assistants").unwrap_err();
        assert!(err.starts_with("Daemon not available:"), "{err}");
    }

    #[test]
    fn an_unreachable_daemon_is_an_error() {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let err = fetch_assistants(&format!("http://127.0.0.1:{port}"), "assistants").unwrap_err();
        assert!(err.starts_with("Daemon not available:"), "{err}");
    }

    #[test]
    fn configure_posts_the_acps_and_returns_what_was_configured() {
        let (base, req) = daemon("200 OK", r#"{"configured":["claude"],"errors":["zed: nope"]}"#);
        let got = configure_at(&base, vec!["claude".into(), "zed".into()]).unwrap();
        assert_eq!(got, vec!["claude".to_string()]);
        let req = req.join().unwrap();
        assert!(req.starts_with("POST /api/assistants/configure "), "{req}");
        let body: Value = serde_json::from_str(req.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body, json!({"acps": ["claude", "zed"]}));
    }

    #[test]
    fn configure_with_only_errors_is_an_error_carrying_them() {
        let (base, _req) = daemon("200 OK", r#"{"configured":[],"errors":["a","b"]}"#);
        assert_eq!(configure_at(&base, vec!["x".into()]).unwrap_err(), "a, b");
    }

    #[test]
    fn a_malformed_configure_body_is_an_error() {
        let (base, _req) = daemon("200 OK", "<html>");
        let err = configure_at(&base, vec![]).unwrap_err();
        assert!(err.starts_with("parse configure response:"), "{err}");
    }
}
