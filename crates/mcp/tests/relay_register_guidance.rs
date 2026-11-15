//! Fix gate F9: MCP `relay_register` surfaces the daemon's guidance block
//! verbatim. The binary pipes daemon 2xx bodies through `daemon_result`
//! untouched, so this is a characterization pin (green from the start —
//! no MCP change needed): a stub daemon serves a canned register response
//! carrying guidance, and the tools/call result text must contain the
//! block's markers. Mirrors `relay_auth_headers.rs` (stub HTTP daemon +
//! spawned sensei-mcp + stdio tools/call).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const GENERIC_MARKER: &str = "guidance-generic-proof";
const FAMILY_MARKER: &str = "guidance-family-proof";

/// Serve one canned 200 + JSON body per connection, recording every request
/// head. Raw TCP, no new deps.
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
            heads
                .lock()
                .expect("heads lock")
                .push(String::from_utf8_lossy(&head[..read]).into_owned());
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

/// Drive one `relay_register` tools/call through a spawned `sensei-mcp`
/// pointed at the stub; return the raw JSON-RPC response line.
fn call_register(port: u16) -> String {
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
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{{\"name\":\"relay_register\",\"arguments\":{{\"role\":\"contributor\",\"family\":\"muse\",\"crew_id\":\"01a1bc94-43de-7f01-b87e-0dbfe6872d0f\"}}}}}}"
    )
    .expect("write call");
    stdin.flush().expect("flush call");
    let response = lines.next().expect("call response").expect("call line");
    let _ = child.kill();
    let _ = child.wait();
    response
}

#[test]
fn relay_register_surfaces_guidance_verbatim() {
    let heads: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (port, stub) = stub_daemon(
        r#"{"lane_token":"tok","lane_id":"lid","crew_id":"cid","guidance":{"version":"1","generic":"guidance-generic-proof beat every 60s","family_notes":"guidance-family-proof muse lanes"}}"#,
        heads.clone(),
    );
    let response = call_register(port);
    stub.join().expect("stub join");
    // The call completed through the real send path (not a vacuous capture).
    let heads = heads.lock().expect("heads lock");
    assert!(!heads.is_empty(), "stub saw no request");
    assert!(
        heads.iter().any(|h| h.contains("/api/relay/register")),
        "stub saw no register call: {heads:?}"
    );
    // The guidance block arrives verbatim in the result text.
    assert!(response.contains(GENERIC_MARKER), "generic guidance lost: {response}");
    assert!(response.contains(FAMILY_MARKER), "family notes lost: {response}");
    // (The result text is JSON-RPC-escaped, hence the backslashes.)
    assert!(response.contains(r#"\"version\": \"1\""#), "version lost: {response}");
}
