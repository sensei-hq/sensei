//! Check whether a TCP port is accepting connections on 127.0.0.1.

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use crate::health::checker::{Checker, CheckOutcome};

pub struct PortChecker {
    pub label: &'static str,
    pub port:  u16,
}

impl PortChecker {
    pub const fn new(label: &'static str, port: u16) -> Self { Self { label, port } }
}

impl Checker for PortChecker {
    fn check(&self) -> CheckOutcome {
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        match TcpStream::connect_timeout(&addr, Duration::from_millis(400)) {
            Ok(_)  => CheckOutcome::ready_no_version(),
            Err(e) => CheckOutcome::failed(
                format!("{} not listening on :{}: {e}", self.label, self.port)
            ),
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
        let c = PortChecker { label: "test", port };
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Ready));
    }
}
