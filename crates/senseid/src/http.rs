//! Outbound HTTP defaults shared by the daemon's clients of remote APIs.

use std::time::Duration;

/// Connect timeout for outbound API calls.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Whole-request timeout for outbound API calls.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// A client builder with bounded connect and total timeouts, so an unreachable or
/// hung remote can never wedge a sequential loop or pile up tasks. Callers add
/// their own policy (e.g. the tracker transport disables redirects).
pub fn bounded_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder().connect_timeout(CONNECT_TIMEOUT).timeout(REQUEST_TIMEOUT)
}
