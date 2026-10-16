//! Issue-tracker integrations: Jira first, behind a provider-neutral seam so
//! another tracker (GitLab) is one more adapter, not a rewrite.
//!
//! Spec: `docs/spec/daemon/tracker-jira-linkage.md`. Provider knowledge lives in
//! the adapter modules (`jira`); everything else here is provider-neutral, and
//! every SQL statement lives in `db::pg_store::trackers`.

pub mod http;
pub mod jira;
pub mod sprint;
pub mod status;

/// Why a tracker call failed — the fixed vocabulary persisted in
/// `sensei.trackers.last_error` (enum `sensei.tracker_error`) and returned by the
/// API. Never a raw error string: a transport error carries the URL, and a
/// response body can carry issue text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackerError {
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited,
    HttpError,
    Transport,
    Decode,
    Redirect,
    TokenMissing,
    CredentialStore,
    UnsupportedTracker,
}

impl TrackerError {
    /// Every variant, for exhaustiveness checks against the DDL enum.
    pub const ALL: [TrackerError; 11] = [
        TrackerError::Unauthorized,
        TrackerError::Forbidden,
        TrackerError::NotFound,
        TrackerError::RateLimited,
        TrackerError::HttpError,
        TrackerError::Transport,
        TrackerError::Decode,
        TrackerError::Redirect,
        TrackerError::TokenMissing,
        TrackerError::CredentialStore,
        TrackerError::UnsupportedTracker,
    ];

    /// The `sensei.tracker_error` enum label for this error.
    pub fn code(self) -> &'static str {
        match self {
            TrackerError::Unauthorized => "unauthorized",
            TrackerError::Forbidden => "forbidden",
            TrackerError::NotFound => "not_found",
            TrackerError::RateLimited => "rate_limited",
            TrackerError::HttpError => "http_error",
            TrackerError::Transport => "transport",
            TrackerError::Decode => "decode",
            TrackerError::Redirect => "redirect",
            TrackerError::TokenMissing => "token_missing",
            TrackerError::CredentialStore => "credential_store",
            TrackerError::UnsupportedTracker => "unsupported_tracker",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_code_is_a_label_of_the_ddl_enum_and_vice_versa() {
        let ddl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../database/ddl/enum/sensei/tracker_error.ddl"
        ))
        .expect("tracker_error.ddl exists");
        let body = ddl
            .split_once("as enum (")
            .and_then(|(_, rest)| rest.split_once(");"))
            .map(|(body, _)| body)
            .expect("an `as enum ( ... );` block");
        let labels: Vec<&str> = body
            .lines()
            .filter_map(|l| {
                let l = l.trim_start_matches([',', ' ']);
                l.strip_prefix('\'').and_then(|r| r.split('\'').next())
            })
            .collect();
        let codes: Vec<&str> = TrackerError::ALL.iter().map(|e| e.code()).collect();
        assert_eq!(
            labels, codes,
            "TrackerError and sensei.tracker_error must list the same labels in the same order"
        );
    }

    #[test]
    fn codes_are_snake_case_and_distinct() {
        let mut codes: Vec<&str> = TrackerError::ALL.iter().map(|e| e.code()).collect();
        assert!(codes.iter().all(|c| c.chars().all(|ch| ch.is_ascii_lowercase() || ch == '_')));
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), TrackerError::ALL.len());
    }
}
