//! Provider-neutral HTTP policy for tracker adapters: the base-URL rule every
//! connection passes through, and the 429 back-off. (Spec R4.)

use std::time::Duration;

/// A base URL (or board URL) that no adapter may call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidBaseUrl;

/// Back-off when `Retry-After` is absent or unreadable.
pub const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(5);
/// Upper bound on any single 429 wait.
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// Validate `raw` and reduce it to its canonical origin, `"https://" + host`
/// (lowercase, punycode, no port, no trailing slash).
///
/// Builds on [`crate::api::util::require_secure_url`] for parsing and the https
/// rule, then refuses that helper's loopback-`http` exemption, userinfo, IP-literal
/// hosts and any explicit port other than 443. Path, query and fragment are
/// discarded. `host_allowed` is the adapter's own host rule, applied last.
pub fn normalize_base_url(
    raw: &str,
    host_allowed: fn(&str) -> bool,
) -> Result<String, InvalidBaseUrl> {
    let url =
        crate::api::util::require_secure_url(raw.trim(), "base url").map_err(|_| InvalidBaseUrl)?;
    // `require_secure_url` lets loopback through over plain http; trackers never do.
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err(InvalidBaseUrl);
    }
    // `port()` is None for the scheme default, so an explicit `:443` passes.
    if url.port().is_some() {
        return Err(InvalidBaseUrl);
    }
    let host = url.host_str().ok_or(InvalidBaseUrl)?;
    let is_ip = host.starts_with('[') || host.parse::<std::net::IpAddr>().is_ok();
    if is_ip || !host_allowed(host) {
        return Err(InvalidBaseUrl);
    }
    Ok(format!("https://{host}"))
}

/// How long to wait after a 429, from its `Retry-After` header.
///
/// A decimal integer is seconds; an HTTP-date is the time until then (never
/// negative); anything else — absent, negative, overflowing, garbage — is
/// [`DEFAULT_RETRY_AFTER`]. Always clamped to `[0, MAX_RETRY_AFTER]`.
pub fn retry_after(header: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> Duration {
    let Some(h) = header.map(str::trim).filter(|h| !h.is_empty()) else {
        return DEFAULT_RETRY_AFTER;
    };
    if h.bytes().all(|b| b.is_ascii_digit()) {
        return h
            .parse::<u64>()
            .map(|secs| Duration::from_secs(secs).min(MAX_RETRY_AFTER))
            .unwrap_or(DEFAULT_RETRY_AFTER);
    }
    match chrono::DateTime::parse_from_rfc2822(h) {
        Ok(at) => {
            let secs = (at.with_timezone(&chrono::Utc) - now).num_seconds().max(0);
            Duration::from_secs(secs as u64).min(MAX_RETRY_AFTER)
        }
        Err(_) => DEFAULT_RETRY_AFTER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jira(host: &str) -> bool {
        crate::tracker::jira::is_jira_cloud_host(host)
    }

    #[test]
    fn canonical_origin_for_valid_jira_urls() {
        for raw in [
            "https://acme.atlassian.net",
            "https://acme.atlassian.net/",
            "https://acme.atlassian.net/jira/",
            "https://ACME.atlassian.net:443/jira/?x=1#y",
        ] {
            assert_eq!(
                normalize_base_url(raw, jira),
                Ok("https://acme.atlassian.net".into()),
                "{raw}"
            );
        }
    }

    #[test]
    fn rejects_unsafe_or_foreign_urls() {
        for raw in [
            "http://acme.atlassian.net",
            "http://localhost",
            "https://localhost",
            "https://evil.com",
            "https://1.2.3.4",
            "https://[::1]",
            "https://u:p@acme.atlassian.net",
            "https://acme.atlassian.net:8443",
            "https://atlassian.net",
            "https://acme.atlassian.net.evil.com",
            "https://acme.atlassian.net.",
            "ftp://acme.atlassian.net",
            "not a url",
            "",
        ] {
            assert_eq!(
                normalize_base_url(raw, jira),
                Err(InvalidBaseUrl),
                "{raw:?} must be rejected"
            );
        }
    }

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-10-16T10:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn retry_after_seconds_are_clamped() {
        assert_eq!(retry_after(Some("1"), now()), Duration::from_secs(1));
        assert_eq!(retry_after(Some("0"), now()), Duration::ZERO);
        assert_eq!(retry_after(Some("60"), now()), Duration::from_secs(60));
        assert_eq!(retry_after(Some("120"), now()), MAX_RETRY_AFTER);
        assert_eq!(retry_after(Some(" 7 "), now()), Duration::from_secs(7));
    }

    #[test]
    fn retry_after_http_date_is_time_until_then() {
        assert_eq!(
            retry_after(Some("Fri, 16 Oct 2026 10:00:30 GMT"), now()),
            Duration::from_secs(30)
        );
        assert_eq!(
            retry_after(Some("Fri, 16 Oct 2026 09:59:00 GMT"), now()),
            Duration::ZERO,
            "a date in the past waits 0"
        );
        assert_eq!(retry_after(Some("Fri, 16 Oct 2026 11:00:00 GMT"), now()), MAX_RETRY_AFTER);
    }

    #[test]
    fn unreadable_retry_after_falls_back_to_the_default() {
        for h in
            [None, Some(""), Some("-5"), Some("99999999999999999999"), Some("soon"), Some("1.5")]
        {
            assert_eq!(retry_after(h, now()), DEFAULT_RETRY_AFTER, "{h:?}");
        }
    }
}
