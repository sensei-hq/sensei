//! Governance Tier-2 consolidation orchestrator (#19 / G7).
//!
//! Composes the pure Tier-1 → Tier-2 helpers ([`crate::governance`]) with the DB
//! and the gateway: resolve the always-on global rules, skip if unchanged since
//! the last merge (source-hash guard), otherwise ask the model to merge them
//! into one coherent markdown ruleset and store it as a new `proposed` version
//! of the `global` scope. Shared by the manual
//! `POST /api/knowledge/rules/consolidate` endpoint and the scheduled
//! `ConsolidateGovernance` task, so both run the identical pipeline.

use crate::db::pg_store::PgStore;
use gateway::Gateway;

/// The consolidation scope handled today — v1 is global-only; the DDL foresees
/// per-repo / namespace scopes later.
const SCOPE: &str = "global";

/// Outcome of one consolidation attempt.
#[derive(Debug)]
pub enum ConsolidationOutcome {
    /// Nothing to do — the `&'static str` is a short, stable reason.
    Skipped(&'static str),
    /// A new `proposed` consolidated ruleset version was written.
    Created {
        id: uuid::Uuid,
        version: i32,
        model: Option<String>,
        content: String,
    },
}

/// Resolve + (conditionally) merge the global governance ruleset. Returns
/// `Skipped` when there are no rules or the Tier-1 input is unchanged since the
/// last merge; otherwise calls the model and writes a new `proposed` version.
/// A gateway / empty-content failure propagates as `Err` (the caller logs / maps
/// it) — the guard checks never call the model, so an unchanged tick is cheap.
pub async fn consolidate_global_rules(
    pg: &PgStore,
    gateway: &Gateway,
) -> Result<ConsolidationOutcome, String> {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Message, MessageRole, Payload};

    let set = crate::governance::structure_ruleset(pg.resolve_global_rules().await?);
    if set.total == 0 {
        return Ok(ConsolidationOutcome::Skipped("no global rules to merge"));
    }
    let hash = crate::governance::ruleset_source_hash(&set);
    if pg.latest_ruleset_source_hash(SCOPE).await? == Some(hash.clone()) {
        return Ok(ConsolidationOutcome::Skipped("unchanged since last consolidation"));
    }

    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        // Heavy synthesis — pin the seed `reasoning` chain (embedded → ollama →
        // cloud) rather than non-deterministic capability resolution (#76).
        chain: Some("reasoning".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(
                MessageRole::User,
                crate::governance::build_merge_prompt(&set),
            )],
            system: Some("You merge governance rules into a single clean markdown ruleset.".to_string()),
            max_tokens: Some(1500),
            temperature: None,
            tools: Vec::new(),
        },
        budget: None,
        auth: None,
        panel: None,
        consensus: None,
        allow_fallback: true,
        credentials: std::collections::HashMap::new(),
    };
    let resp = gateway
        .execute(&request)
        .await
        .map_err(|e| format!("merge model call failed: {e}"))?;
    let content = match (resp.success, resp.content) {
        (true, Some(c)) if !c.trim().is_empty() => c,
        _ => return Err("merge model returned no content".to_string()),
    };

    let version = pg.next_ruleset_version(SCOPE).await?;
    let id = pg
        .insert_consolidated_ruleset(
            SCOPE,
            version,
            &content,
            &serde_json::json!([]),
            resp.model.as_deref(),
            &hash,
            "proposed",
        )
        .await?;
    Ok(ConsolidationOutcome::Created { id, version, model: resp.model, content })
}

// No unit test here: this orchestrator is thin glue over the pure governance
// helpers (`structure_ruleset` / `ruleset_source_hash` / `build_merge_prompt`,
// all unit-tested in `crate::governance`) plus the DB + gateway. A DB test would
// read GLOBAL rule state that other tests mutate concurrently (the source hash
// can change between snapshot and call, taking it down the model-calling path) —
// inherently flaky in the shared test DB. Both paths (skip when unchanged/empty,
// and create via the gemma4 merge) are verified live end-to-end instead.
