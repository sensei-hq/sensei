//! Consent to READ an assistant's conversation history (#218).
//!
//! Configuring an assistant writes hooks into its settings. That is consent to
//! instrument the agent, and it is a different thing from reading every past
//! conversation it has on disk. Transcripts are the most sensitive data sensei
//! touches — whole conversations, often with code, pasted credentials and client
//! names — so reading them has its own switch, per assistant.
//!
//! ON BY DEFAULT since 2026-10-19 (#280, docs/decisions.md), superseding #218's
//! default of off: the token, cost and session metrics read this history, and an
//! off-by-default left every one of them blank without saying why.
//!
//! Before this, `dispatch()` walked all six adapters unconditionally every 300 s,
//! so turning an assistant off removed its hooks and sensei kept reading its
//! history anyway.
//!
//! One `sensei.config` key per capture source, `transcripts.consent.<source>`,
//! `off` is no; anything else, including absence, is yes.

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
const OFF: &str = "off";

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
        if pg.get_config(&key(source)).await?.as_deref() != Some(OFF) {
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
    pg.set_config(&key(source), if on { ON } else { OFF }).await
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

    /// Absence is yes (decided 2026-10-19, superseding #218's default): reading
    /// history is core to the token, cost and session metrics. An explicit `off`
    /// is no. Mutation that must break this: treat a missing key as off.
    #[tokio::test]
    async fn everything_is_consented_until_someone_says_no() {
        let pg = crate::db::pg_store::PgStore::test_store().await;
        let _gate = CONSENT_KEYS.enter();
        for (s, _) in SOURCES {
            sqlx_core::query::query("DELETE FROM sensei.config WHERE key = $1")
                .bind(key(s))
                .execute(pg.pool())
                .await
                .unwrap();
        }
        let all: HashSet<String> = SOURCES.iter().map(|(s, _)| (*s).to_string()).collect();
        assert_eq!(consented(&pg).await.unwrap(), all, "no key is consent");

        set(&pg, "zed", false).await.unwrap();
        let mut without_zed = all.clone();
        without_zed.remove("zed");
        assert_eq!(consented(&pg).await.unwrap(), without_zed, "off is off");

        set(&pg, "zed", true).await.unwrap();
        assert_eq!(consented(&pg).await.unwrap(), all, "on again");

        assert!(set(&pg, "notepad", true).await.is_err(), "an unknown source is refused");
    }
}
