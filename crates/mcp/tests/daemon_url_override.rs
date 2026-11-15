//! F1 reds: `--daemon-url` / `SENSEI_DAEMON_URL` must address a non-default port.
//!
//! Each test spins a stub HTTP daemon on an ephemeral port, drives one MCP
//! `get_rules` call (a single daemon GET, no required args) through the
//! override, and asserts the stub's marker comes back — which only happens on
//! a 200 with the stub's body. Run-01 went raw-curl-only because no override
//! existed; these fail until phase B wires it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::Duration;

const MARKER: &str = "daemon-url-proof";

/// Serve one canned 200 + JSON body to the first few connections, then stop.
/// Raw TCP, no new deps: read the request head, answer, close.
fn stub_daemon(body: &'static str) -> (u16, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("stub bind");
    let port = listener.local_addr().expect("stub addr").port();
    let handle = std::thread::spawn(move || {
        // Nonblocking accept with an idle deadline: serve every connection
        // the client opens (call count varies), then exit — no lingering
        // thread or held port past the join (N1).
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
            let mut head = vec![0u8; 4096];
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

/// Drive `get_rules` through a spawned `sensei-mcp` and return the raw
/// JSON-RPC response line for the tools/call request.
fn call_get_rules(configure: &dyn Fn(&mut Command)) -> String {
    let bin = env!("CARGO_BIN_EXE_sensei-mcp");
    let mut child = Command::new(bin);
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("SENSEI_DAEMON_URL");
    configure(&mut child);
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
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{{\"name\":\"get_rules\",\"arguments\":{{}}}}}}"
    )
    .expect("write call");
    stdin.flush().expect("flush call");
    let response = lines.next().expect("call response").expect("call line");
    let _ = child.kill();
    let _ = child.wait();
    response
}

#[test]
fn mcp_flag_override_reaches_ephemeral_port() {
    let (port, stub) = stub_daemon(r#"{"stub":"daemon-url-proof"}"#);
    let target = format!("127.0.0.1:{port}");
    let response = call_get_rules(&|cmd| {
        cmd.arg("--daemon-url").arg(&target);
    });
    assert!(response.contains(MARKER), "flag override did not reach the stub daemon: {response}");
    stub.join().expect("stub join");
}

#[test]
fn mcp_env_override_reaches_ephemeral_port() {
    let (port, stub) = stub_daemon(r#"{"stub":"daemon-url-proof"}"#);
    let target = format!("http://127.0.0.1:{port}");
    let response = call_get_rules(&|cmd| {
        cmd.env("SENSEI_DAEMON_URL", &target);
    });
    assert!(response.contains(MARKER), "env override did not reach the stub daemon: {response}");
    stub.join().expect("stub join");
}
