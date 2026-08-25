//! #84 T2 Slice B — Probe an MCP server for its tool manifest.
//!
//! Spawns the configured `command` + `args` with `env` merged over the
//! current process's environment, speaks JSON-RPC over stdio using the MCP
//! line-delimited protocol, and returns the `tools/list` response.
//!
//! Failure modes are exhaustive by design: spawn errors, protocol errors,
//! timeouts, and non-JSON garbage all resolve to `ProbeOutcome::Error` with
//! a short human-readable message. Callers store the message on
//! `mcp_tool_manifests.error` so the Playground tab can show why a server
//! isn't listing tools right now.
//!
//! No global state; every probe is a self-contained subprocess with a hard
//! wall-clock deadline. Cheap enough to run on-demand from an HTTP handler.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// Wall-clock cap for one probe. Covers spawn + initialize round-trip +
/// tools/list round-trip on a slow MCP; a hung server is killed at the
/// deadline.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// The MCP protocol version we negotiate with the server. Servers that
/// implement an older version should still respond with a valid
/// `serverInfo` block.
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// One tool descriptor extracted from a `tools/list` response.
#[derive(Debug, Clone)]
pub struct ProbedManifest {
    pub protocol_version: Option<String>,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    /// Raw tool array from `tools/list.result.tools`. Each entry is a JSON
    /// object with `name`, `description`, `inputSchema`.
    pub tools: Vec<Value>,
}

/// Result of one probe attempt.
pub enum ProbeOutcome {
    Ok(ProbedManifest),
    Error(String),
}

/// Spawn `command` with `args`, merge `env` over the current process's
/// environment, and run the MCP handshake + tools/list. Returns
/// `ProbeOutcome::Error` for any failure (spawn, timeout, JSON parse,
/// error response) — never panics.
pub async fn probe_tools(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: Option<&std::path::Path>,
) -> ProbeOutcome {
    if command.is_empty() {
        return ProbeOutcome::Error("no command configured".into());
    }
    match tokio::time::timeout(PROBE_TIMEOUT, probe_inner(command, args, env, cwd)).await {
        Ok(Ok(manifest)) => ProbeOutcome::Ok(manifest),
        Ok(Err(e)) => ProbeOutcome::Error(e),
        Err(_) => ProbeOutcome::Error(format!("timed out after {}s", PROBE_TIMEOUT.as_secs())),
    }
}

async fn probe_inner(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: Option<&std::path::Path>,
) -> Result<ProbedManifest, String> {
    let mut cmd = Command::new(command);
    cmd.args(args);
    // Plugin MCP configs sometimes use a command/arg path relative to the
    // config file's directory (e.g. `node packages/mcp-stdio/dist/index.js`),
    // so run from the config's dir when the caller provides one.
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    // Merge env over process env; the mcp entry usually only wants to set
    // API keys, so we don't strip the process's PATH etc.
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("stdin unavailable")?;
    let stdout = child.stdout.take().ok_or("stdout unavailable")?;
    let mut reader = BufReader::new(stdout);

    // 1. initialize.
    write_frame(&mut stdin, &initialize_request()).await?;
    let init_resp = read_response_matching(&mut reader, 1).await?;
    let init_result = init_resp.get("result").cloned().unwrap_or(Value::Null);
    let protocol_version =
        init_result.get("protocolVersion").and_then(|v| v.as_str()).map(String::from);
    let server_info = init_result.get("serverInfo").cloned().unwrap_or(Value::Null);
    let server_name = server_info.get("name").and_then(|v| v.as_str()).map(String::from);
    let server_version = server_info.get("version").and_then(|v| v.as_str()).map(String::from);

    // 2. Send the initialized notification (fire-and-forget — no id/no response).
    write_frame(&mut stdin, &initialized_notification()).await?;

    // 3. tools/list.
    write_frame(&mut stdin, &tools_list_request()).await?;
    let list_resp = read_response_matching(&mut reader, 2).await?;
    if let Some(err) = list_resp.get("error") {
        let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("tools/list error");
        return Err(format!("server error: {msg}"));
    }
    let tools = list_resp
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    // 4. Best-effort clean shutdown. The `kill_on_drop` on the child means
    // an early return still terminates the process.
    let _ = stdin.shutdown().await;
    let _ = child.kill().await;

    Ok(ProbedManifest { protocol_version, server_name, server_version, tools })
}

async fn write_frame(stdin: &mut tokio::process::ChildStdin, msg: &Value) -> Result<(), String> {
    let mut line = serde_json::to_string(msg).map_err(|e| format!("serialise: {e}"))?;
    line.push('\n');
    stdin.write_all(line.as_bytes()).await.map_err(|e| format!("write: {e}"))?;
    stdin.flush().await.map_err(|e| format!("flush: {e}"))?;
    Ok(())
}

/// Read newline-delimited JSON frames from `reader` until one with `id ==
/// expected_id` arrives. Skips notifications and unrelated messages. Returns
/// an error if the reader closes before a match is seen.
async fn read_response_matching(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    expected_id: u64,
) -> Result<Value, String> {
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await.map_err(|e| format!("read: {e}"))?;
        if bytes == 0 {
            return Err("server closed stdout before response".into());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => return Err(format!("bad json from server: {e}")),
        };
        // Match the response id.
        if msg.get("id").and_then(|v| v.as_u64()) == Some(expected_id) {
            return Ok(msg);
        }
        // Non-matching id → probably a server-originated notification or
        // request; ignore and keep reading.
    }
}

fn initialize_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": MCP_PROTOCOL_VERSION,
            "capabilities":    {},
            "clientInfo":      { "name": "sensei", "version": env!("CARGO_PKG_VERSION") }
        }
    })
}

fn initialized_notification() -> Value {
    json!({
        "jsonrpc": "2.0",
        "method":  "notifications/initialized"
    })
}

fn tools_list_request() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id":      2,
        "method":  "tools/list",
        "params":  {}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_request_has_expected_shape() {
        let req = initialize_request();
        assert_eq!(req["jsonrpc"], "2.0");
        assert_eq!(req["id"], 1);
        assert_eq!(req["method"], "initialize");
        assert_eq!(req["params"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(req["params"]["clientInfo"]["name"], "sensei");
        assert!(
            req["params"]["clientInfo"]["version"].is_string(),
            "version populated from CARGO_PKG_VERSION"
        );
    }

    #[test]
    fn tools_list_request_matches_mcp_protocol() {
        let req = tools_list_request();
        assert_eq!(req["jsonrpc"], "2.0");
        assert_eq!(
            req["id"], 2,
            "response id must be 2 so we can match it in read_response_matching"
        );
        assert_eq!(req["method"], "tools/list");
    }

    #[test]
    fn initialized_notification_has_no_id() {
        let n = initialized_notification();
        assert!(n.get("id").is_none(), "notifications carry no id");
        assert_eq!(n["method"], "notifications/initialized");
    }

    #[tokio::test]
    async fn empty_command_is_early_error_no_spawn() {
        let outcome = probe_tools("", &[], &HashMap::new(), None).await;
        match outcome {
            ProbeOutcome::Error(msg) => assert!(msg.contains("no command"), "got: {msg}"),
            _ => panic!("expected early Error for empty command"),
        }
    }

    #[tokio::test]
    async fn nonexistent_binary_returns_spawn_error() {
        let outcome =
            probe_tools("definitely_not_a_real_binary_xyz_88", &[], &HashMap::new(), None).await;
        match outcome {
            ProbeOutcome::Error(msg) => assert!(msg.contains("spawn failed"), "got: {msg}"),
            _ => panic!("expected spawn Error"),
        }
    }

    #[tokio::test]
    async fn silent_server_hits_timeout() {
        // A sleep binary reads nothing and writes nothing — the probe
        // should time out rather than block. Using `sleep 5` (not `cat`
        // which would echo our own JSON-RPC back) so the wrapper hits the
        // timeout branch.
        let outcome = tokio::time::timeout(
            Duration::from_secs(2),
            probe_inner_test_wrapper(
                "sleep",
                &["5".into()],
                &HashMap::new(),
                Duration::from_millis(200),
            ),
        )
        .await
        .expect("wrapper itself must not hang");
        assert!(
            outcome.contains("timed out") || outcome.contains("read") || outcome.contains("closed"),
            "expected timeout/read/closed error, got: {outcome}"
        );
    }

    /// Test-only shim that runs probe_inner with a short timeout so the
    /// silent-server test doesn't need to wait 10s.
    async fn probe_inner_test_wrapper(
        cmd: &str,
        args: &[String],
        env: &HashMap<String, String>,
        timeout: Duration,
    ) -> String {
        match tokio::time::timeout(timeout, probe_inner(cmd, args, env, None)).await {
            Ok(Ok(_)) => "ok".into(),
            Ok(Err(e)) => e,
            Err(_) => format!("timed out after {}ms", timeout.as_millis()),
        }
    }
}
