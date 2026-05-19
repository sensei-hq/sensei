//! Check whether a TCP port is accepting connections on 127.0.0.1.
//!
//! The fast-path timeout (400ms) is right for "is it up right now" — used
//! during the initial check phase. Resolve-phase callers (postgres after
//! `brew services start`, daemon after `senseid start`) need a longer
//! deadline because the service is legitimately starting and an immediate
//! re-check would race it; `with_timeout` lets those checkers wait.

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use crate::health::checker::{Checker, CheckOutcome};

pub const DEFAULT_PORT_TIMEOUT: Duration = Duration::from_millis(400);

pub struct PortChecker {
    pub label:   &'static str,
    pub port:    u16,
    pub timeout: Duration,
}

impl PortChecker {
    /// Fast default (400ms) — fine for already-running services.
    pub const fn new(label: &'static str, port: u16) -> Self {
        Self { label, port, timeout: DEFAULT_PORT_TIMEOUT }
    }
    /// Use when the service was just started and might still be binding.
    pub const fn with_timeout(label: &'static str, port: u16, timeout: Duration) -> Self {
        Self { label, port, timeout }
    }
}

impl Checker for PortChecker {
    fn check(&self) -> CheckOutcome {
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        tracing::debug!(check = "port", label = self.label, port = self.port, timeout_ms = self.timeout.as_millis() as u64, "probing");
        // Retry up to the configured timeout in 250ms slices. Each individual
        // connect_timeout uses the SHORT default so a closed port returns
        // fast; only legitimately-starting services exhaust the budget.
        let deadline = std::time::Instant::now() + self.timeout;
        let start = std::time::Instant::now();
        loop {
            match TcpStream::connect_timeout(&addr, DEFAULT_PORT_TIMEOUT) {
                Ok(_) => {
                    tracing::debug!(check = "port", label = self.label, port = self.port, result = "open", elapsed_ms = start.elapsed().as_millis() as u64, "port reachable");
                    return CheckOutcome::ready_no_version();
                }
                Err(e) => {
                    if std::time::Instant::now() >= deadline {
                        tracing::info!(check = "port", label = self.label, port = self.port, result = "closed", error = %e, "port not listening");
                        return CheckOutcome::failed(format!(
                            "{} not listening on :{}: {e}", self.label, self.port,
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::types::ComponentStatus;

    #[test]
    fn unbound_port_returns_failed() {
        // Port 1 is reliably closed on user machines (privileged, no service binds it).
        let c = PortChecker::new("test", 1);
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Failed));
        assert!(o.detail.as_deref().unwrap().contains("not listening on :1"));
    }

    #[test]
    fn bound_port_returns_ready() {
        // Bind to an ephemeral port and check it.
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("must bind");
        let port = listener.local_addr().unwrap().port();
        // Box the listener into the closure scope so it stays alive.
        let _l = listener;
        let c = PortChecker::new("test", port);
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Ready));
    }
}
