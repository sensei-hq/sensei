//! Federation: push promoted rules to a hive-mind and poll-pull applicable rules
//! back as memories(origin='federated'). The ACP never talks to a hive; senseid
//! owns all outbound calls (spec §4).

use crate::db::pg_store::{InsertMemory, KnowledgeSource, MemoryPushPayload, PgStore};
use hive_protocol::{content_hash, PublishedRule, PullResponse};

/// Build the wire payload for a memory being published. `published_by`/`published_at`
/// are stamped server-side by the hive (the hive overrides them from the API key's
/// member + now()), so we send harmless best-effort placeholders.
pub fn build_published_rule(p: &MemoryPushPayload, origin_repo: Option<&str>) -> PublishedRule {
    PublishedRule {
        content_hash: content_hash(&p.content),
        scope_key: p.scope_key.clone(),
        namespace_slug: p.slug.clone(),
        namespace_name: p.name.clone(),
        rule_type: p.rule_type.clone(),
        title: p.title.clone(),
        content: p.content.clone(),
        impact: p.impact.clone(),
        enforcement: p.enforcement.clone(),
        origin_repo: origin_repo.map(|s| s.to_string()),
        published_by: "senseid".to_string(),       // hive overrides from the API key's member
        published_at: "1970-01-01T00:00:00Z".to_string(), // hive overrides with now()
    }
}

/// Push a just-approved promoted memory to every push-capable source whose
/// namespace matches. No-op unless the memory is origin='promoted' at a shareable
/// scope. Errors are logged, not propagated to the approval path.
pub async fn push_promoted(pg: &PgStore, memory_id: uuid::Uuid) {
    let payload = match pg.memory_push_payload(&memory_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return,
        Err(e) => { tracing::warn!(error = %e, "federation: push payload load failed"); return; }
    };
    if payload.origin != "promoted" { return; }
    let namespace_id = match resolve_memory_namespace_id(pg, &memory_id).await { Some(id) => id, None => return };
    if !matches!(pg.namespace_is_shareable(&namespace_id).await, Ok(true)) { return; }
    let sources = match pg.list_knowledge_sources().await { Ok(s) => s, Err(_) => return };
    let pr = build_published_rule(&payload, None);
    let client = reqwest::Client::new();
    for src in sources.into_iter().filter(|s| s.enabled
        && matches!(s.direction.as_str(), "push" | "both")
        && (s.namespace_id.is_none() || s.namespace_id == Some(namespace_id))) {
        if let Err(e) = push_one(pg, &client, &src, &pr, memory_id).await {
            tracing::warn!(source = %src.name, error = %e, "federation: push failed");
        }
    }
}

async fn resolve_memory_namespace_id(pg: &PgStore, memory_id: &uuid::Uuid) -> Option<uuid::Uuid> {
    let row: Option<(Option<uuid::Uuid>,)> = sqlx_core::query_as::query_as(
        "SELECT namespace_id FROM sensei.memories WHERE id = $1")
        .bind(memory_id).fetch_optional(pg.pool()).await.ok()?;
    row.and_then(|(n,)| n)
}

async fn push_one(
    pg: &PgStore, client: &reqwest::Client, src: &KnowledgeSource,
    pr: &PublishedRule, memory_id: uuid::Uuid,
) -> Result<(), String> {
    let key = crate::gateway_keys::get_key(&src.credential_ref).map_err(|e| e.to_string())?;
    let resp = client.post(format!("{}/v1/rules", src.url.trim_end_matches('/')))
        .bearer_auth(key).json(pr).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() { return Err(format!("hive returned {}", resp.status())); }
    let pubresp: hive_protocol::PublishResponse = resp.json().await.map_err(|e| e.to_string())?;
    let remote_id = uuid::Uuid::parse_str(&pubresp.id).map_err(|e| e.to_string())?;
    pg.upsert_federated_memory(&src.id, &remote_id, &pr.content_hash, Some(&memory_id), pubresp.seq).await?;
    Ok(())
}

/// Result of one pull pass over a source.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct PullStats {
    pub applied: usize, pub tombstoned: usize, pub linked: usize, pub new_cursor: i64,
}

/// Pull one source's deltas since its cursor and apply them. Idempotent via the
/// ledger (which is also the echo-guard for rules this daemon pushed). Advances last_seq.
pub async fn pull_source(pg: &PgStore, client: &reqwest::Client, src: &KnowledgeSource)
    -> Result<PullStats, String> {
    let key = crate::gateway_keys::get_key(&src.credential_ref).map_err(|e| e.to_string())?;
    let resp = client.get(format!("{}/v1/rules?since={}", src.url.trim_end_matches('/'), src.last_seq))
        .bearer_auth(key).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() { return Err(format!("hive returned {}", resp.status())); }
    let page: PullResponse = resp.json().await.map_err(|e| e.to_string())?;
    let mut stats = PullStats { new_cursor: page.cursor, ..Default::default() };

    for pulled in &page.rules {
        let remote_id = uuid::Uuid::parse_str(&pulled.id).map_err(|e| e.to_string())?;
        let existing = pg.find_federated_memory(&src.id, &remote_id).await?;
        let tombstoned = pulled.status == "tombstoned";
        match existing {
            Some(link) => {
                if tombstoned {
                    if let Some(mid) = link.memory_id
                        && pg.archive_federated_memory(&mid).await? {
                        stats.tombstoned += 1;
                    }
                } else {
                    stats.linked += 1; // content re-sync of already-known rules = follow-up (#55-adjacent)
                }
                pg.upsert_federated_memory(&src.id, &remote_id, &pulled.rule.content_hash, link.memory_id.as_ref(), pulled.seq).await?;
            }
            None if tombstoned => {
                pg.upsert_federated_memory(&src.id, &remote_id, &pulled.rule.content_hash, None, pulled.seq).await?;
            }
            None => {
                let ns = pg.upsert_namespace(&pulled.rule.scope_key, &pulled.rule.namespace_name, &pulled.rule.namespace_slug).await?;
                let mem = pg.insert_memory(&InsertMemory {
                    project_id: None, scope: "global".into(), scope_filter: None,
                    mtype: pulled.rule.rule_type.clone(),
                    title: pulled.rule.title.clone(), content: pulled.rule.content.clone(),
                    impact: pulled.rule.impact.clone(), tags: vec![], triage_signal: None,
                    status: "active".into(), namespace_id: Some(ns),
                    enforcement: Some(pulled.rule.enforcement.clone()),
                    origin: Some("federated".into()), source_id: Some(src.id),
                }).await?;
                pg.upsert_federated_memory(&src.id, &remote_id, &pulled.rule.content_hash, Some(&mem), pulled.seq).await?;
                stats.applied += 1;
            }
        }
    }
    pg.set_source_cursor(&src.id, page.cursor).await?;
    Ok(stats)
}

/// Spawned background task: every `interval_secs`, pull every pull-capable source.
pub fn run_pull_loop(pg: PgStore, interval_secs: u64) {
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            tick.tick().await;
            let sources = match pg.list_knowledge_sources().await {
                Ok(s) => s, Err(e) => { tracing::warn!(error=%e, "federation: list sources failed"); continue; }
            };
            for src in sources.into_iter().filter(|s| s.enabled && matches!(s.direction.as_str(), "pull" | "both")) {
                match pull_source(&pg, &client, &src).await {
                    Ok(st) if st.applied + st.tombstoned > 0 =>
                        tracing::info!(source=%src.name, applied=st.applied, tombstoned=st.tombstoned, "federation: pulled"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!(source=%src.name, error=%e, "federation: pull failed"),
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pg_store::MemoryPushPayload;

    #[test]
    fn builds_published_rule_with_content_hash_and_namespace_identity() {
        let p = MemoryPushPayload {
            title: "TDD".into(), content: "  Always use TDD  ".into(), impact: None,
            enforcement: "mandatory".into(), rule_type: "convention".into(), origin: "promoted".into(),
            scope_key: "organization".into(), slug: "sensei-hq".into(), name: "Sensei HQ".into(),
        };
        let pr = build_published_rule(&p, Some("sensei/daemon"));
        // content_hash normalizes (trim+lowercase) — matches hive-protocol.
        assert_eq!(pr.content_hash, hive_protocol::content_hash("Always use TDD"));
        assert_eq!(pr.scope_key, "organization");
        assert_eq!(pr.namespace_slug, "sensei-hq");
        assert_eq!(pr.namespace_name, "Sensei HQ");
        assert_eq!(pr.enforcement, "mandatory");
        assert_eq!(pr.rule_type, "convention");
        assert_eq!(pr.origin_repo.as_deref(), Some("sensei/daemon"));
    }
}
