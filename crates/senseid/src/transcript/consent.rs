//! Consent to READ an assistant's conversation history (#218).
//!
//! Configuring an assistant writes hooks into its settings. That is consent to
//! instrument the agent, and it is a different thing from reading every past
//! conversation it has on disk. Transcripts are the most sensitive data sensei
//! touches — whole conversations, often with code, pasted credentials and client
//! names — so reading them gets its own yes, per assistant, and DEFAULTS TO NO.
//!
//! Before this, `dispatch()` walked all six adapters unconditionally every 300 s,
//! so turning an assistant off removed its hooks and sensei kept reading its
//! history anyway.
//!
//! One `sensei.config` key per capture source, `transcripts.consent.<source>`,
//! holding `on`. Anything else, including absence, is no.

use std::collections::HashSet;

/// Every capture source an adapter can read, with the name a person knows it by.
///
/// A test holds `adapters()` to this list, so a seventh adapter cannot be added
/// and read without someone deciding what its consent is called.
pub const SOURCES: &[(&str, &str)] = &[
    ("claude_code", "Claude Code"),
    ("zed", "Zed"),
    ("opencode", "opencode"),
    ("cursor", "Cursor"),
    ("vscode", "VS Code"),
    ("copilot_cli", "Copilot CLI"),
];

const ON: &str = "on";

fn key(source: &str) -> String {
    format!("transcripts.consent.{source}")
}

pub fn is_known(source: &str) -> bool {
    SOURCES.iter().any(|(s, _)| *s == source)
}

/// The sources the user has said yes to.
///
/// A FAILED READ IS AN `Err`, and every caller treats it as "read nothing". An
/// unreadable consent is not a yes.
pub async fn consented(pg: &crate::db::pg_store::PgStore) -> Result<HashSet<String>, String> {
    let mut yes = HashSet::new();
    for (source, _) in SOURCES {
        if pg.get_config(&key(source)).await?.as_deref() == Some(ON) {
            yes.insert((*source).to_string());
        }
    }
    Ok(yes)
}

/// Record a yes or a no. An unknown source is refused rather than stored: a key
/// nothing reads would look like consent and govern nothing.
pub async fn set(pg: &crate::db::pg_store::PgStore, source: &str, on: bool) -> Result<(), String> {
    if !is_known(source) {
        return Err(format!("unknown transcript source '{source}'"));
    }
    pg.set_config(&key(source), if on { ON } else { "off" }).await
}

/// Serialises the tests that write these config keys — they share one table.
#[cfg(test)]
pub(crate) static CONSENT_KEYS: crate::tasks::test_support::TestGate =
    crate::tasks::test_support::TestGate::new();

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;

    /// Every adapter's source has a consent entry. Without this, a new adapter
    /// would be filtered out by `dispatch` forever with no switch to turn it on
    /// — or, worse, be added to a path that skips the filter.
    #[test]
    fn every_adapter_has_a_consent_entry() {
        for ad in super::super::adapters() {
            assert!(is_known(ad.source()), "{} has no consent entry", ad.source());
        }
        assert_eq!(super::super::adapters().len(), SOURCES.len());
    }

    /// Absence is no. Mutation that must break this: treat a missing key as on.
    #[tokio::test]
    async fn nothing_is_consented_until_someone_says_yes() {
        let Ok(pg) = crate::db::pg_store::PgStore::connect_test().await else {
            return;
        };
        let _gate = CONSENT_KEYS.enter();
        for (s, _) in SOURCES {
            sqlx_core::query::query("DELETE FROM sensei.config WHERE key = $1")
                .bind(key(s))
                .execute(pg.pool())
                .await
                .unwrap();
        }
        assert!(consented(&pg).await.unwrap().is_empty());

        set(&pg, "zed", true).await.unwrap();
        assert_eq!(consented(&pg).await.unwrap(), HashSet::from(["zed".to_string()]));

        set(&pg, "zed", false).await.unwrap();
        assert!(consented(&pg).await.unwrap().is_empty(), "off is off, not absent-and-ignored");

        assert!(set(&pg, "notepad", true).await.is_err(), "an unknown source is refused");
    }
}
