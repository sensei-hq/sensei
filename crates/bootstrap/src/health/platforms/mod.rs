//! Platform implementations of PlatformProvider, and the probes they share.

pub mod macos;
pub mod windows;

use crate::health::checker::Checker;
use crate::health::checkers::port::PortChecker;
use std::time::Duration;

// Port-checker timeouts are sized for the resolve-phase re-check.
// `brew services start postgresql@17` takes 1-5s before the port
// is bound; `senseid start` daemonizes and binds within 1-2s.
// The initial check (retry=false) uses a 400ms single-attempt probe
// so closed ports report failed in ~ms instead of paying the full
// patient deadline (which used to add 13s of wasted wall time).
pub(crate) const POSTGRES_PORT_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const OLLAMA_PORT_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const DAEMON_PORT_TIMEOUT: Duration = Duration::from_secs(3);

/// A port probe: a quick single attempt on the initial check, a patient one
/// on the post-resolver re-check (`retry`).
pub(crate) fn port(
    label: &'static str,
    port: u16,
    retry: bool,
    recheck_timeout: Duration,
) -> Box<dyn Checker> {
    if retry {
        Box::new(PortChecker::with_timeout(label, port, recheck_timeout))
    } else {
        Box::new(PortChecker::new(label, port))
    }
}
