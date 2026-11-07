//! F1 reds: `sensei --daemon-url` / `SENSEI_DAEMON_URL` must address a
//! non-default port. Each test spins a stub `/relay/now` on an ephemeral port,
//! runs `sensei relay` through the override, and asserts the stub's marker
//! goal prints — which only happens when the CLI reads the stub's 200 body.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

const MARKER: &str = "daemon-url-proof";

fn stub_now() -> (u16, std::thread::JoinHandle<()>) {
    let body = r#"{"lanes":[{"code":"PROBE","role":"contributor","crew_id":"c1"}],"crews":[{"id":"c1","goal":"daemon-url-proof"}]}"#;
    let listener = TcpListener::bind("127.0.0.1:0").expect("stub bind");
    let port = listener.local_addr().expect("stub addr").port();
    let handle = std::thread::spawn(move || {
        for _ in 0..4 {
            let (mut stream, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(_) => return,
            };
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

fn run_relay(configure: &dyn Fn(&mut Command)) -> String {
    let bin = env!("CARGO_BIN_EXE_sensei");
    let mut cmd = Command::new(bin);
    cmd.env_remove("SENSEI_DAEMON_URL");
    configure(&mut cmd);
    let output = cmd.output().expect("run sensei relay");
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
#[ignore = "pending:F1"]
fn cli_flag_override_reaches_ephemeral_port() {
    let (port, _stub) = stub_now();
    let target = format!("127.0.0.1:{port}");
    let out = run_relay(&|cmd| {
        cmd.arg("--daemon-url").arg(&target).arg("relay");
    });
    assert!(out.contains(MARKER), "flag override did not reach the stub daemon: {out}");
}

#[test]
#[ignore = "pending:F1"]
fn cli_env_override_reaches_ephemeral_port() {
    let (port, _stub) = stub_now();
    let target = format!("http://127.0.0.1:{port}");
    let out = run_relay(&|cmd| {
        cmd.env("SENSEI_DAEMON_URL", &target).arg("relay");
    });
    assert!(out.contains(MARKER), "env override did not reach the stub daemon: {out}");
}
