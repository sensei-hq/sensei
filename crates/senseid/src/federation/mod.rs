//! Federation: push promoted rules to a Dōjō and poll-pull applicable rules
//! back as memories(origin='federated'). The ACP never talks to a dojo; senseid
//! owns all outbound calls (spec §4).
//!
//! Wire target (D1): rules ride the Worker's **tenant path**
//! `{KnowledgeSource.url}/rules` — where `url` is the tenant base
//! `{registry}/v1/t/{origin}/{org}` — authenticated with the per-membership
//! **device token** (Keychain, via `credential_ref`), the SAME plane the
//! artifacts client uses (`dojo/client.rs`). The `dojo_protocol` request/response
//! shapes are unchanged; only the URL + credential kind moved off the retired
//! dojo-mind's global `/v1/rules` + API-key plane.

use crate::db::pg_store::{InsertMemory, KnowledgeSource, MemoryPushPayload, PgStore};
use dojo_protocol::{PublishedRule, PullResponse, content_hash};

/// HTTP client for all federation calls. Bounded connect + total timeouts so a
/// hung/unreachable dojo can never wedge the (sequential) pull loop or pile up
/// push tasks — same liveness discipline as the daemon's other outbound clients.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|e| { tracing::warn!(error = %e, "federation: http client build failed, using default client without timeouts"); reqwest::Client::new() })
}

/// Build the wire payload for a memory being published. `published_by`/`published_at`
/// are stamped server-side by the dojo (the dojo overrides them from the API key's
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
        published_by: "senseid".to_string(), // dojo overrides from the API key's member
        published_at: "1970-01-01T00:00:00Z".to_string(), // dojo overrides with now()
    }
}

/// Push a just-approved promoted memory to every push-capable source whose
/// namespace matches. No-op unless the memory is origin='promoted' at a shareable
/// scope. Errors are logged, not propagated to the approval path.
pub async fn push_promoted(pg: &PgStore, memory_id: uuid::Uuid) {
    let payload = match pg.memory_push_payload(&memory_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!(error = %e, "federation: push payload load failed");
            return;
        }
    };
    if payload.origin != "promoted" {
        return;
    }
    let namespace_id = match resolve_memory_namespace_id(pg, &memory_id).await {
        Some(id) => id,
        None => return,
    };
    match pg.namespace_is_shareable(&namespace_id).await {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            tracing::warn!(error = %e, "federation: namespace shareability check failed");
            return;
        }
    }
    let sources = match pg.list_knowledge_sources().await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "federation: list sources failed");
            return;
        }
    };
    let pr = build_published_rule(&payload, None);
    let client = http_client();
    for src in sources.into_iter().filter(|s| {
        s.enabled
            && matches!(s.direction.as_str(), "push" | "both")
            && (s.namespace_id.is_none() || s.namespace_id == Some(namespace_id))
    }) {
        if let Err(e) = push_one(pg, &client, &src, &pr, memory_id).await {
            tracing::warn!(source = %src.name, error = %e, "federation: push failed");
        }
    }
}

async fn resolve_memory_namespace_id(pg: &PgStore, memory_id: &uuid::Uuid) -> Option<uuid::Uuid> {
    let row: Option<(Option<uuid::Uuid>,)> = match sqlx_core::query_as::query_as(
        "SELECT namespace_id FROM sensei.memories WHERE id = $1",
    )
    .bind(memory_id)
    .fetch_optional(pg.pool())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "federation: resolve namespace query failed");
            return None;
        }
    };
    row.and_then(|(n,)| n)
}

/// The rules endpoint for a source: `{url}/rules`, where `url` is the tenant base
/// `{registry}/v1/t/{origin}/{org}` the Worker mounts rules under (D1). The
/// origin/org already live inside `url` as real path segments (registrar-provided),
/// so there is no `tenant_key` to re-encode here — unlike the artifacts client,
/// which composes `registry_url` + a bare `tenant_key`. Single join point so
/// publish + pull agree on the trailing-slash discipline.
fn rules_url(base: &str) -> String {
    format!("{}/rules", base.trim_end_matches('/'))
}

async fn push_one(
    pg: &PgStore,
    client: &reqwest::Client,
    src: &KnowledgeSource,
    pr: &PublishedRule,
    memory_id: uuid::Uuid,
) -> Result<(), String> {
    // The Keychain credential is the per-membership device token (D1), attached as
    // the Bearer exactly as before — only the credential kind + URL plane changed.
    let key = crate::gateway_keys::get_key(&src.credential_ref).map_err(|e| e.to_string())?;
    let resp = client
        .post(rules_url(&src.url))
        .bearer_auth(key)
        .json(pr)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("dojo returned {}", resp.status()));
    }
    let pubresp: dojo_protocol::PublishResponse = resp.json().await.map_err(|e| e.to_string())?;
    let remote_id = uuid::Uuid::parse_str(&pubresp.id).map_err(|e| e.to_string())?;
    pg.upsert_federated_memory(
        &src.id,
        &remote_id,
        &pr.content_hash,
        Some(&memory_id),
        pubresp.seq,
    )
    .await?;
    Ok(())
}

/// Result of one pull pass over a source.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct PullStats {
    pub applied: usize,
    pub tombstoned: usize,
    pub linked: usize,
    pub new_cursor: i64,
}

/// Pull one source's deltas since its cursor and apply them. Idempotent via the
/// ledger (which is also the echo-guard for rules this daemon pushed). Advances last_seq.
pub async fn pull_source(
    pg: &PgStore,
    client: &reqwest::Client,
    src: &KnowledgeSource,
) -> Result<PullStats, String> {
    let key = crate::gateway_keys::get_key(&src.credential_ref).map_err(|e| e.to_string())?;
    let resp = client
        .get(rules_url(&src.url))
        .query(&[("since", src.last_seq.to_string())])
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("dojo returned {}", resp.status()));
    }
    let page: PullResponse = resp.json().await.map_err(|e| e.to_string())?;
    let mut stats = PullStats { new_cursor: page.cursor, ..Default::default() };

    for pulled in &page.rules {
        match apply_pulled_rule(pg, src, pulled).await {
            Ok(RuleOutcome::Applied) => stats.applied += 1,
            Ok(RuleOutcome::Tombstoned) => stats.tombstoned += 1,
            Ok(RuleOutcome::Linked) => stats.linked += 1,
            Ok(RuleOutcome::Skipped) => {}
            // Per-rule isolation: a single malformed delta (bad uuid/enum value, a
            // scope_key absent locally, etc.) is logged and skipped — never a poison
            // pill that stalls the source or the rules ordered after it in the page.
            Err(e) => tracing::warn!(rule = %pulled.id, source = %src.name, error = %e,
                "federation: skipping malformed pulled rule"),
        }
    }
    pg.set_source_cursor(&src.id, page.cursor).await?;
    Ok(stats)
}

/// Outcome of applying one pulled delta — drives `PullStats`.
enum RuleOutcome {
    Applied,
    Tombstoned,
    Linked,
    Skipped,
}

/// Apply one pulled delta to the local store. Idempotent via the `federated_memories`
/// ledger (also the echo-guard for rules this daemon pushed). Errors are returned so
/// the caller can isolate a single bad delta without stalling the whole pull.
async fn apply_pulled_rule(
    pg: &PgStore,
    src: &KnowledgeSource,
    pulled: &dojo_protocol::PulledRule,
) -> Result<RuleOutcome, String> {
    let remote_id = uuid::Uuid::parse_str(&pulled.id).map_err(|e| e.to_string())?;
    let existing = pg.find_federated_memory(&src.id, &remote_id).await?;
    let tombstoned = pulled.status == "tombstoned";
    match existing {
        Some(link) => {
            // Staleness guard: skip a non-tombstone delta we've already applied at or
            // beyond this remote seq (defensive against cursor resets / reordering).
            if !tombstoned && pulled.seq <= link.remote_seq {
                return Ok(RuleOutcome::Skipped);
            }
            let outcome = if tombstoned {
                if let Some(mid) = link.memory_id
                    && pg.archive_federated_memory(&mid).await?
                {
                    RuleOutcome::Tombstoned
                } else {
                    RuleOutcome::Skipped
                }
            } else {
                RuleOutcome::Linked // content re-sync of already-known rules = follow-up (#55-adjacent)
            };
            pg.upsert_federated_memory(
                &src.id,
                &remote_id,
                &pulled.rule.content_hash,
                link.memory_id.as_ref(),
                pulled.seq,
            )
            .await?;
            Ok(outcome)
        }
        None if tombstoned => {
            pg.upsert_federated_memory(
                &src.id,
                &remote_id,
                &pulled.rule.content_hash,
                None,
                pulled.seq,
            )
            .await?;
            Ok(RuleOutcome::Skipped)
        }
        None => {
            let ns = pg
                .upsert_namespace(
                    &pulled.rule.scope_key,
                    &pulled.rule.namespace_name,
                    &pulled.rule.namespace_slug,
                )
                .await?;
            let mem = pg
                .insert_memory(&InsertMemory {
                    project_id: None,
                    scope: "global".into(),
                    scope_filter: None,
                    mtype: pulled.rule.rule_type.clone(),
                    title: pulled.rule.title.clone(),
                    content: pulled.rule.content.clone(),
                    impact: pulled.rule.impact.clone(),
                    tags: vec![],
                    triage_signal: None,
                    status: "active".into(),
                    namespace_id: Some(ns),
                    enforcement: Some(pulled.rule.enforcement.clone()),
                    origin: Some("federated".into()),
                    source_id: Some(src.id),
                    spine_slot: None,
                    feature: None,
                })
                .await?;
            pg.upsert_federated_memory(
                &src.id,
                &remote_id,
                &pulled.rule.content_hash,
                Some(&mem),
                pulled.seq,
            )
            .await?;
            Ok(RuleOutcome::Applied)
        }
    }
}

/// Spawned background task: every `interval_secs`, pull every pull-capable source.
pub fn run_pull_loop(pg: PgStore, interval_secs: u64) {
    tokio::spawn(async move {
        let client = http_client();
        let mut tick = crate::tasks::ticker::ticker(interval_secs);
        loop {
            tick.tick().await;
            let sources = match pg.list_knowledge_sources().await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error=%e, "federation: list sources failed");
                    continue;
                }
            };
            for src in sources
                .into_iter()
                .filter(|s| s.enabled && matches!(s.direction.as_str(), "pull" | "both"))
            {
                match pull_source(&pg, &client, &src).await {
                    Ok(st) if st.applied + st.tombstoned > 0 => {
                        tracing::info!(source=%src.name, applied=st.applied, tombstoned=st.tombstoned, "federation: pulled")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(source=%src.name, error=%e, "federation: pull failed"),
                }
            }
            // C7: also pull each Dōjō membership's downstream artifacts into the
            // local inbox. Fully guarded — a dojo pull failure (or the membership
            // listing failing) must never break the rules pull above; log + move on.
            match pg.list_dojo_memberships().await {
                Ok(memberships) => {
                    for m in memberships.into_iter().filter(|m| m.enabled) {
                        match crate::collective::inbox::pull_membership(&pg, &m).await {
                            Ok(o) if o.inserted > 0 => {
                                tracing::info!(membership=%m.id, inserted=o.inserted, cursor=o.new_cursor, "dojo: pulled downstream artifacts")
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(membership=%m.id, error=%e, "dojo: downstream pull failed")
                            }
                        }
                    }
                }
                Err(e) => tracing::warn!(error=%e, "dojo: list memberships failed"),
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
            title: "TDD".into(),
            content: "  Always use TDD  ".into(),
            impact: None,
            enforcement: "mandatory".into(),
            rule_type: "convention".into(),
            origin: "promoted".into(),
            scope_key: "organization".into(),
            slug: "sensei-hq".into(),
            name: "Sensei HQ".into(),
        };
        let pr = build_published_rule(&p, Some("sensei/daemon"));
        // content_hash normalizes (trim+lowercase) — matches dojo-protocol.
        assert_eq!(pr.content_hash, dojo_protocol::content_hash("Always use TDD"));
        assert_eq!(pr.scope_key, "organization");
        assert_eq!(pr.namespace_slug, "sensei-hq");
        assert_eq!(pr.namespace_name, "Sensei HQ");
        assert_eq!(pr.enforcement, "mandatory");
        assert_eq!(pr.rule_type, "convention");
        assert_eq!(pr.origin_repo.as_deref(), Some("sensei/daemon"));
    }

    #[tokio::test]
    async fn e2e_daemon_pulls_a_rule_published_on_the_dojo() {
        use crate::db::pg_store::{NewKnowledgeSource, PgStore};
        use axum::{
            Json, Router,
            extract::{Query, State},
            routing::get,
        };
        use dojo_protocol::{PublishedRule, PullResponse, PulledRule, content_hash};
        use std::collections::HashMap;
        use std::sync::Arc;
        let Ok(pg) = PgStore::connect_test().await else {
            return;
        }; // skip if no test DB

        // Seed the `organization` scope used by the pulled rule (sensei_test is empty;
        // production data is seeded via staging.import_scopes — we replicate the one row
        // we need so the namespaces.scope_key FK is satisfiable). Same idiom as the
        // sibling `federated_ledger_and_shareability` test.
        sqlx_core::query::query(
            "INSERT INTO sensei.scopes(key, name, level, shareable)
             VALUES ('organization', 'Organization', 20, true)
             ON CONFLICT (key) DO UPDATE SET shareable = EXCLUDED.shareable",
        )
        .execute(pg.pool())
        .await
        .unwrap();

        // 1. Stand up a tiny axum stub for the Worker's TENANT-PATH rules endpoint
        // `GET /v1/t/{origin}/{org}/rules` on an ephemeral port (D1 — rules now ride
        // the same tenant path + device-token plane as artifacts; mirrors the sibling
        // `pull_artifacts_forwards_since_and_parses_response` stub in dojo/client.rs).
        // The `{origin}`/`{org}` matchers prove the daemon built the tenant URL (a bare
        // `/v1/rules` would 404 here). It respects the `?since=` cursor — echoing
        // `since + 1` as the new cursor proves the daemon forwarded its `last_seq`,
        // and returns exactly one active `PulledRule` (built with `content_hash` so
        // it matches the real publish path). No embedded Postgres, no dojo-mind: the
        // daemon only ever talks HTTP to a source, so an HTTP stub is a faithful peer
        // and sidesteps the embedded-PG flakiness the old in-process dōjō carried.
        let content = format!("e2e federated rule {}", uuid::Uuid::new_v4());
        // The unique content rides as router State so the `'static` handler and the
        // post-condition assertion agree on it (reruns don't collide). Same State
        // pattern as the sibling `relay_post_retries_transient_failures` stub.
        async fn rules(
            State(content): State<Arc<String>>,
            Query(q): Query<HashMap<String, String>>,
        ) -> Json<PullResponse> {
            let since: i64 = q.get("since").and_then(|s| s.parse().ok()).unwrap_or(-1);
            Json(PullResponse {
                rules: vec![PulledRule {
                    id: uuid::Uuid::new_v4().to_string(),
                    seq: since + 1,
                    status: "active".into(),
                    version: 1,
                    rule: PublishedRule {
                        content_hash: content_hash(&content),
                        scope_key: "organization".into(),
                        namespace_slug: "e2e-org".into(),
                        namespace_name: "E2E Org".into(),
                        rule_type: "convention".into(),
                        title: "E2E".into(),
                        content: (*content).clone(),
                        impact: None,
                        enforcement: "mandatory".into(),
                        origin_repo: None,
                        published_by: "x".into(),
                        published_at: "1970-01-01T00:00:00Z".into(),
                    },
                }],
                cursor: since + 1,
            })
        }
        let app = Router::new()
            .route("/v1/t/{origin}/{org}/rules", get(rules))
            .with_state(Arc::new(content.clone()));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        // The source `url` is the tenant base `{registry}/v1/t/{origin}/{org}` the
        // Worker mounts rules under — the daemon appends `/rules` (D1). This is the
        // exact shape a rules KnowledgeSource carries post-switch.
        let dojo_url = format!("http://{addr}/v1/t/github/acme");

        // 2. Register the source on the daemon (device token in the Keychain, row in
        // PG). The stub ignores the bearer, but `pull_source` resolves it, so seed one.
        let client = crate::federation::http_client();
        let cref = format!("dojo-e2e-{}", uuid::Uuid::new_v4());
        crate::gateway_keys::set_key(&cref, "device-token-e2e").unwrap();
        let src_id = pg
            .create_knowledge_source(&NewKnowledgeSource {
                kind: "hive_mind".into(),
                name: "E2E".into(),
                url: dojo_url,
                namespace_id: None,
                credential_ref: cref.clone(),
                direction: "pull".into(),
            })
            .await
            .unwrap();
        let src = pg.get_knowledge_source(&src_id).await.unwrap().unwrap();

        // 3. Pull.
        let stats = pull_source(&pg, &client, &src).await.expect("pull");
        assert_eq!(stats.applied, 1, "one federated memory created");
        assert!(stats.new_cursor > 0);

        // 4. The pulled rule is a federated, active memory with our content.
        let (cnt,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.memories WHERE origin='federated' AND content=$1 AND status='active'")
            .bind(&content).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(cnt, 1);

        // 5. Cleanup (cascade ledger via source delete; remove memory + keychain entry,
        // the namespace the pull created, and the seeded scope row).
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE content=$1")
            .bind(&content)
            .execute(pg.pool())
            .await
            .unwrap();
        pg.delete_knowledge_source(&src_id).await.unwrap();
        let _ = crate::gateway_keys::delete_key(&cref);
        sqlx_core::query::query(
            "DELETE FROM sensei.namespaces WHERE scope_key='organization' AND slug='e2e-org'",
        )
        .execute(pg.pool())
        .await
        .unwrap();
        sqlx_core::query::query("DELETE FROM sensei.scopes WHERE key='organization'")
            .execute(pg.pool())
            .await
            .unwrap();
    }
}
