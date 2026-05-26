//! Check whether a TCP port is accepting connections on 127.0.0.1.
//!
//! Two modes:
//!   - retry=false (default `PortChecker::new`) — single 400ms connect attempt.
//!     Right for the initial health check phase: ECONNREFUSED returns
//!     instantly, so a closed port reports failed in ~ms.
//!   - retry=true (`PortChecker::with_timeout`) — sleep-retry loop up to
//!     the configured deadline. Right for post-resolver rechecks where the
//!     service was just started (postgres takes 1-5s to bind, senseid 1-2s)
//!     and the loop catches the bind the instant it lands.
//!
//! The trap this replaces: the initial check was previously paying the full
//! deadline (5s × N components = 13s of wall time) on closed ports, because
//! the same long-timeout PortChecker was used for both phases.

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use crate::health::checker::{Checker, CheckOutcome};

pub const DEFAULT_PORT_TIMEOUT: Duration = Duration::from_millis(400);

pub struct PortChecker {
    pub label:   &'static str,
    pub port:    u16,
    pub timeout: Duration,
    /// Sleep-loop until the port comes up (or the deadline lapses).
    /// `false` = single attempt; `true` = retry every 250ms until `timeout`.
    pub retry:   bool,
}

impl PortChecker {
    /// Initial-check variant — single 400ms attempt, no retry.
    /// Closed ports report failed immediately on ECONNREFUSED.
    pub const fn new(label: &'static str, port: u16) -> Self {
        Self { label, port, timeout: DEFAULT_PORT_TIMEOUT, retry: false }
    }
    /// Recheck variant — keeps retrying until `timeout` elapses. Use after
    /// a resolver action (brew services start, senseid start) where the
    /// service is legitimately binding and an immediate connect would race.
    pub const fn with_timeout(label: &'static str, port: u16, timeout: Duration) -> Self {
        Self { label, port, timeout, retry: true }
    }
}

impl Checker for PortChecker {
    fn check(&self) -> CheckOutcome {
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        tracing::debug!(check = "port", label = self.label, port = self.port, timeout_ms = self.timeout.as_millis() as u64, retry = self.retry, "probing");

        if !self.retry {
            // Single attempt — closed ports return ECONNREFUSED in ~ms.
            return match TcpStream::connect_timeout(&addr, self.timeout) {
                Ok(_) => {
                    tracing::debug!(check = "port", label = self.label, port = self.port, result = "open", "port reachable");
                    CheckOutcome::ready_no_version()
                }
                Err(e) => {
                    tracing::info!(check = "port", label = self.label, port = self.port, result = "closed", error = %e, "port not listening");
                    CheckOutcome::failed(format!(
                        "{} not listening on :{}: {e}", self.label, self.port,
                    ))
                }
            };
        }

        // Retry loop — sleep 250ms between attempts until the deadline. Each
        // individual connect_timeout uses the SHORT default so a closed port
        // returns fast; only legitimately-starting services exhaust the budget.
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
