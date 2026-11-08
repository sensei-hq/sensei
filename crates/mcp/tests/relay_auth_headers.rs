//! Run-02 blocker: authed MCP relay calls must carry the lane token on the
//! wire. `relay_authed` shapes the token into `DaemonRequest.headers`, but
//! `send_daemon_request` never applied them — every authed relay tool 401d
//! while raw curl 200d. This test drives `relay_heartbeat` through the real
//! binary against a stub daemon and asserts the Authorization header arrives
//! (and the token leaks nowhere else). It fails on the shaping-only code:
//! the stub sees no auth header. Mirrors `daemon_url_override.rs` (stub HTTP
//! daemon + spawned sensei-mcp + stdio tools/call).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";
const LANE_ID: &str = "01a1bc85-b136-75b3-bfa3-c8fd3a9420c3";
const MARKER: &str = "auth-header-proof";

/// Serve one canned 200 + JSON body per connection, recording every request
/// head. Raw TCP, no new deps (mirrors the override test's stub).
fn stub_daemon(
    body: &'static str,
    heads: Arc<Mutex<Vec<String>>>,
) -> (u16, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("stub bind");
    let port = listener.local_addr().expect("stub addr").port();
    let handle = std::thread::spawn(move || {
        listener.set_nonblocking(true).expect("stub nonblocking");
        let mut idle_since = std::time::Instant::now();
        loop {
            let mut stream = match listener.accept() {
                Ok((stream, _)) => {
                    idle_since = std::time::Instant::now();
                    stream
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if idle_since.elapsed() > Duration::from_secs(5) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(_) => return,
            };
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut head = vec![0u8; 8192];
            let mut read = 0;
            while read < head.len() {
                match stream.read(&mut head[read..]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        read += n;
                        if head[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                }
            }
            heads.lock().expect("heads lock").push(String::from_utf8_lossy(&head[..read]).into_owned());
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    (port, handle)
}

/// Drive one `relay_heartbeat` tools/call through a spawned `sensei-mcp`
/// pointed at the stub; return the raw JSON-RPC response line.
fn call_heartbeat(port: u16) -> String {
    let bin = env!("CARGO_BIN_EXE_sensei-mcp");
    let mut child = Command::new(bin);
    child
        .arg("--daemon-url")
        .arg(format!("127.0.0.1:{port}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("SENSEI_DAEMON_URL");
    let mut child = child.spawn().expect("spawn sensei-mcp");
    let mut stdin = child.stdin.take().expect("mcp stdin");
    let stdout = child.stdout.take().expect("mcp stdout");
    let mut lines = BufReader::new(stdout).lines();
    writeln!(stdin, "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{{}}}}")
        .expect("write initialize");
    stdin.flush().expect("flush initialize");
    let _hello = lines.next().expect("init response").expect("init line");
    writeln!(
        stdin,
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{{\"name\":\"relay_heartbeat\",\"arguments\":{{\"lane_id\":\"{LANE_ID}\",\"lane_token\":\"{TOKEN}\"}}}}}}"
    )
    .expect("write call");
    stdin.flush().expect("flush call");
    let response = lines.next().expect("call response").expect("call line");
    let _ = child.kill();
    let _ = child.wait();
    response
}

#[test]
fn relay_authed_call_sends_bearer_token_in_headers_only() {
    let heads: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (port, stub) = stub_daemon(r#"{"stub":"auth-header-proof"}"#, heads.clone());
    let response = call_heartbeat(port);
    stub.join().expect("stub join");
    // The call completed through the real send path (not a vacuous capture).
    assert!(response.contains(MARKER), "heartbeat did not round-trip the stub body: {response}");
    let heads = heads.lock().expect("heads lock");
    assert!(!heads.is_empty(), "stub saw no request");
    let expected = format!("Bearer {TOKEN}");
    assert!(
        heads.iter().any(|h| h
            .lines()
            .any(|l| l.to_lowercase().starts_with("authorization:") && l.contains(&expected))),
        "no Authorization Bearer header on the wire: {heads:?}"
    );
    // The token rides the header only — never the request line (path/query).
    for h in heads.iter() {
        let request_line = h.lines().next().unwrap_or("");
        assert!(!request_line.contains(TOKEN), "token leaked into request line: {request_line}");
    }
}
