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
/// deadline. Measured 2026-10-26: semgrep's server answers in 5.6s warm and
/// overran 10s cold, so its tools were never listed. A probe is cached, so a
/// generous cap costs one slow read, not one per view.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

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

/// Result of one tool call through a server.
pub enum CallOutcome {
    /// The server answered; `is_error` is the tool's own `isError`.
    Ok { result: Value, is_error: bool },
    /// The call never got an answer: spawn, protocol, timeout.
    Error(String),
}

/// Wall-clock cap for one tool call — longer than a probe: the tool does work.
pub const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Call one tool on a server over stdio, exactly as an assistant would: the
/// same handshake, then `tools/call`. What the Playground shows is what the
/// server returns, not a daemon-side imitation of it.
pub async fn call_tool(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: Option<&std::path::Path>,
    tool: &str,
    arguments: &Value,
) -> CallOutcome {
    if command.is_empty() {
        return CallOutcome::Error("no command configured".into());
    }
    let run = async {
        let mut session = Session::start(command, args, env, cwd).await?;
        let resp =
            session.request(2, "tools/call", json!({"name": tool, "arguments": arguments})).await;
        session.close().await;
        resp
    };
    match tokio::time::timeout(CALL_TIMEOUT, run).await {
        Ok(Ok(result)) => {
            let is_error = result.get("isError").and_then(Value::as_bool).unwrap_or(false);
            CallOutcome::Ok { result, is_error }
        }
        Ok(Err(e)) => CallOutcome::Error(e),
        Err(_) => CallOutcome::Error(format!("timed out after {}s", CALL_TIMEOUT.as_secs())),
    }
}

/// A server spawned over stdio, past the MCP handshake.
struct Session {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    protocol_version: Option<String>,
    server_name: Option<String>,
    server_version: Option<String>,
}

impl Session {
    async fn start(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
        cwd: Option<&std::path::Path>,
    ) -> Result<Self, String> {
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
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);

        let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("stdout unavailable")?;
        let mut reader = BufReader::new(stdout);

        write_frame(&mut stdin, &initialize_request()).await?;
        let init_resp = read_response_matching(&mut reader, 1).await?;
        let init_result = init_resp.get("result").cloned().unwrap_or(Value::Null);
        let server_info = init_result.get("serverInfo").cloned().unwrap_or(Value::Null);
        // Fire-and-forget: a notification has no id and no response.
        write_frame(&mut stdin, &initialized_notification()).await?;
        Ok(Self {
            child,
            stdin,
            reader,
            protocol_version: init_result
                .get("protocolVersion")
                .and_then(|v| v.as_str())
                .map(String::from),
            server_name: server_info.get("name").and_then(|v| v.as_str()).map(String::from),
            server_version: server_info.get("version").and_then(|v| v.as_str()).map(String::from),
        })
    }

    /// One request; its `result`, or the server's `error` as an `Err`.
    async fn request(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        write_frame(&mut self.stdin, &msg).await?;
        let resp = read_response_matching(&mut self.reader, id).await?;
        if let Some(err) = resp.get("error") {
            let text = err.get("message").and_then(|v| v.as_str()).unwrap_or("error");
            return Err(format!("server error: {text}"));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Best-effort clean shutdown; `kill_on_drop` covers an early return.
    async fn close(mut self) {
        let _ = self.stdin.shutdown().await;
        let _ = self.child.kill().await;
    }
}

async fn probe_inner(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: Option<&std::path::Path>,
) -> Result<ProbedManifest, String> {
    let mut session = Session::start(command, args, env, cwd).await?;
    let listed = session.request(2, "tools/list", json!({})).await;
    let (protocol_version, server_name, server_version) = (
        session.protocol_version.take(),
        session.server_name.take(),
        session.server_version.take(),
    );
    session.close().await;
    let tools = listed?.get("tools").and_then(|t| t.as_array()).cloned().unwrap_or_default();
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

/// A stand-in MCP server for tests: answers initialize, then one tools/call —
/// with a result when asked for `echo` with `{"x":1}`, else a tool error.
#[cfg(test)]
pub(crate) mod fake {
    pub(crate) fn server() -> (String, Vec<String>) {
        let script = r#"read l
echo '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","serverInfo":{"name":"fake"}}}'
read l
read l
case "$l" in
  *'"name":"echo"'*'"x":1'*) echo '{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"echoed"}],"isError":false}}' ;;
  *) echo '{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"bad args"}],"isError":true}}' ;;
esac"#;
        ("sh".into(), vec!["-c".into(), script.into()])
    }
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

    /// A tool runs through the server itself, as an assistant would call it:
    /// the arguments reach the server and its result comes back whole.
    #[tokio::test]
    async fn a_tool_call_reaches_the_server_and_returns_its_result() {
        let (cmd, args) = fake::server();
        let got = call_tool(&cmd, &args, &HashMap::new(), None, "echo", &json!({"x": 1})).await;
        match got {
            CallOutcome::Ok { result, is_error } => {
                assert!(!is_error);
                assert_eq!(result["content"][0]["text"], "echoed");
            }
            CallOutcome::Error(e) => panic!("expected a result, got {e}"),
        }
    }

    /// A tool that reports failure is a failure the caller sees — `isError`
    /// is carried, never flattened into a success.
    #[tokio::test]
    async fn a_tool_error_is_reported_as_one() {
        let (cmd, args) = fake::server();
        let got = call_tool(&cmd, &args, &HashMap::new(), None, "echo", &json!({"x": 2})).await;
        match got {
            CallOutcome::Ok { result, is_error } => {
                assert!(is_error, "the server said isError");
                assert_eq!(result["content"][0]["text"], "bad args");
            }
            CallOutcome::Error(e) => panic!("a tool error is a result, not a transport error: {e}"),
        }
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
