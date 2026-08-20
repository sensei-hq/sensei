//! Corrections aggregation handler (#65 step 5). Global: pull every user prompt,
//! keep corrections (regex recall → LLM precision), embed them (lexical fallback),
//! deterministically cluster, summarize each surviving cluster (graceful), and
//! upsert idempotently by signature (then prune stale signatures).

use super::super::executor::TaskContext;
use super::super::Task;
use super::analyze::correction_signal;
use super::corrections_llm::{summarize_cluster, ClusterSummary};
use super::prompt_classify::{classify_batch, PromptClass};
use crate::corrections::{self, parse_cluster_min, parse_similarity_threshold, Cluster, CorrItem, CorrectionRow};

/// Per-batch embedding wall-clock cap — a stalled backend must not wedge the task.
const EMBED_TIMEOUT_SECS: u64 = 30;
/// Expected embedding width (the pinned 384-dim `embed` chain). A different width
/// means a misconfigured chain — fall back to lexical rather than cluster on
/// semantically wrong vectors.
const EMBED_DIM: usize = 384;

/// Pure: assemble upsert rows from clusters + their (aligned) summaries. Drops
/// clusters below `cluster_min`. `summaries[i]` corresponds to `clusters[i]`;
/// `None` ⇒ snippet fallback.
pub fn build_rows(
    clusters: &[Cluster],
    summaries: &[Option<ClusterSummary>],
    items: &[CorrItem],
    cluster_min: usize,
) -> Vec<CorrectionRow> {
    let mut rows = Vec::new();
    for (c, summary) in clusters.iter().zip(summaries.iter()) {
        if c.count < cluster_min {
            continue;
        }
        let (text, suggestion, memory_id) = match summary {
            Some(s) => (s.text.clone(), s.suggestion.clone(), s.memory_id),
            None => (c.representative_text.clone(), None, None),
        };
        let instances = serde_json::Value::Array(
            c.member_idxs
                .iter()
                .filter_map(|&i| items.get(i))
                .map(|it| {
                    serde_json::json!({
                        "project_id": it.project_id,
                        "session_id": it.session_id,
                        "ts": it.ts,
                        "prompt": corrections::normalize(&it.prompt),
                    })
                })
                .collect(),
        );
        rows.push(CorrectionRow {
            signature: c.signature.clone(),
            text,
            suggestion,
            count: c.count as i32,
            project_ids: c.project_ids.clone(),
            last_seen: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(c.last_seen_ms)
                .unwrap_or_default(),
            memory_id,
            instances,
        });
    }
    rows
}

/// Embed each item's normalized prompt via the 384-dim `embed` chain. `None` ⇒ no
/// embed model / failure / count mismatch (caller uses the lexical fallback).
async fn embed_items(ctx: &TaskContext, items: &[CorrItem]) -> Option<Vec<Vec<f32>>> {
    use gateway::types::capability::Capability;
    use gateway::types::request::{InferenceRequest, Payload};
    // Cap each prompt, then pack into token-bounded batches, reusing the
    // node-embed guards. User prompts are unbounded, and a fixed 64-count batch
    // of them sums past the embed encoder's n_ubatch and calls a GGML abort()
    // that kills the daemon — so both the per-input and per-batch token totals
    // must be bounded before the call. `pack_embed_batches` preserves order, so
    // `out` stays aligned with `items`.
    let texts: Vec<String> = items
        .iter()
        .map(|it| super::embed::cap_embed_input(corrections::normalize(&it.prompt)))
        .collect();
    let batches = super::embed::pack_embed_batches(&texts);
    let mut out: Vec<Vec<f32>> = Vec::with_capacity(items.len());
    for batch in &batches {
        let batch_texts: Vec<String> = batch.iter().map(|&i| texts[i].clone()).collect();
        let request = InferenceRequest {
            capability: Capability::TextEmbed,
            model: None,
            router: None,
            chain: Some("embed".to_string()),
            payload: Payload::Embed { texts: batch_texts },
            budget: None,
            auth: None,
            panel: None,
            consensus: None,
            allow_fallback: true,
            credentials: std::collections::HashMap::new(),
        };
        match tokio::time::timeout(
            std::time::Duration::from_secs(EMBED_TIMEOUT_SECS),
            ctx.app_state.gateway.execute(&request),
        )
        .await
        {
            Ok(Ok(resp)) => {
                let embs = resp.embeddings.unwrap_or_default();
                if embs.len() != batch.len() {
                    tracing::warn!(
                        got = embs.len(), expected = batch.len(),
                        "aggregate_corrections: embedding count mismatch — lexical fallback"
                    );
                    return None;
                }
                if embs.iter().any(|e| e.len() != EMBED_DIM) {
                    tracing::warn!("aggregate_corrections: unexpected embedding width — lexical fallback");
                    return None;
                }
                out.extend(embs);
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "aggregate_corrections: embed gateway error — lexical fallback");
                return None;
            }
            Err(_) => {
                tracing::warn!("aggregate_corrections: embed timed out — lexical fallback");
                return None;
            }
        }
    }
    Some(out)
}

/// Global corrections aggregation. Idempotent; degrades gracefully without models.
pub async fn aggregate_corrections(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    // Config-driven thresholds; constants are the defaults.
    let sim_threshold = parse_similarity_threshold(
        ctx.pg().get_config("corrections.similarity_threshold").await.ok().flatten().as_deref(),
    );
    let cluster_min = parse_cluster_min(
        ctx.pg().get_config("corrections.cluster_min").await.ok().flatten().as_deref(),
    );

    // 1. Pull all user prompts, keep regex-recall corrections.
    let all = ctx.pg().get_all_user_prompts().await?;
    let mut candidates: Vec<CorrItem> = all
        .into_iter()
        .filter(|(_, _, _, _, prompt)| correction_signal(prompt).is_some())
        .map(|(project_id, _pname, session_id, ts, prompt)| CorrItem { project_id, session_id, ts, prompt })
        .collect();

    if candidates.is_empty() {
        let pruned = ctx.pg().delete_corrections_not_in(&[]).await.unwrap_or(0);
        tracing::info!("aggregate_corrections: no corrective prompts; {pruned} pruned");
        return Ok(0);
    }

    // 2. LLM precision pass (graceful): drop regex false positives + principles.
    let texts: Vec<&str> = candidates.iter().map(|c| c.prompt.as_str()).collect();
    let refined = classify_batch(&ctx.app_state.gateway, &texts).await;
    let mut items: Vec<CorrItem> = candidates
        .drain(..)
        .enumerate()
        .filter(|(i, _)| {
            !matches!(
                refined.get(*i).copied().flatten(),
                Some(PromptClass::Principle) | Some(PromptClass::Neither)
            )
        })
        .map(|(_, c)| c)
        .collect();

    // If the LLM confidently rejected every regex candidate, do NOT prune: a
    // single (fallible) classification pass shouldn't wipe existing corrections.
    if items.is_empty() {
        tracing::info!("aggregate_corrections: LLM filtered all candidates — preserving existing rows");
        return Ok(0);
    }

    // Deterministic order: earliest ts seeds each cluster (session tiebreak).
    items.sort_by(|a, b| a.ts.cmp(&b.ts).then_with(|| a.session_id.cmp(&b.session_id)));

    // 3. Cluster (embed → lexical fallback).
    let clusters = match embed_items(ctx, &items).await {
        Some(embeddings) => corrections::cluster(&items, &embeddings, sim_threshold),
        None => {
            tracing::warn!("aggregate_corrections: no embeddings — lexical fallback");
            corrections::lexical_cluster(&items)
        }
    };

    // 4. Per-cluster summary (graceful), only for clusters that will surface.
    let memories = match ctx.pg().get_learned_memories_for_matching().await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "aggregate_corrections: could not load memories — memory_id will be null");
            Vec::new()
        }
    };
    let mut summaries: Vec<Option<ClusterSummary>> = Vec::with_capacity(clusters.len());
    for c in &clusters {
        if c.count < cluster_min {
            summaries.push(None);
            continue;
        }
        let reps: Vec<&str> = c
            .member_idxs
            .iter()
            .take(5)
            .filter_map(|&i| items.get(i))
            .map(|it| it.prompt.as_str())
            .collect();
        summaries.push(summarize_cluster(&ctx.app_state.gateway, &reps, &memories).await);
    }

    // 5. Upsert + prune. `keep` is built from the rows we intend to persist; when
    // genuinely no corrections recur it is empty and the table is cleared.
    let rows = build_rows(&clusters, &summaries, &items, cluster_min);
    // `keep` lists EVERY intended row (even if its upsert transiently fails) so a
    // failed re-upsert never prunes an already-persisted row. `written` counts
    // only the upserts that actually succeeded, for a faithful return/log.
    let mut keep: Vec<String> = Vec::with_capacity(rows.len());
    let mut written = 0u32;
    for row in &rows {
        keep.push(row.signature.clone());
        match ctx.pg().upsert_correction(row).await {
            Ok(_) => written += 1,
            Err(e) => tracing::warn!(error = %e, signature = %row.signature, "aggregate_corrections: upsert failed"),
        }
    }
    let pruned = ctx.pg().delete_corrections_not_in(&keep).await.unwrap_or(0);
    tracing::info!("aggregate_corrections: {written} upserted ({} rows), {pruned} pruned", rows.len());
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(pid: uuid::Uuid, ts: i64, prompt: &str) -> CorrItem {
        CorrItem { project_id: pid, session_id: "s".into(), ts, prompt: prompt.into() }
    }

    #[test]
    fn build_rows_drops_singletons_and_uses_summary_then_snippet() {
        let p = uuid::Uuid::new_v4();
        let items = vec![item(p, 100, "use $state"), item(p, 200, "use $state please"), item(p, 300, "lone")];
        let clusters = vec![
            Cluster {
                signature: "corr-a".into(),
                representative_text: "use state".into(),
                count: 2,
                last_seen_ms: 200,
                project_ids: vec![p],
                member_idxs: vec![0, 1],
            },
            Cluster {
                signature: "corr-b".into(),
                representative_text: "lone".into(),
                count: 1,
                last_seen_ms: 300,
                project_ids: vec![p],
                member_idxs: vec![2],
            },
        ];
        let summaries = vec![
            Some(ClusterSummary { text: "Use $state".into(), suggestion: Some("reinforce".into()), memory_id: None }),
            None,
        ];
        let rows = build_rows(&clusters, &summaries, &items, 2);
        assert_eq!(rows.len(), 1, "singleton dropped");
        assert_eq!(rows[0].signature, "corr-a");
        assert_eq!(rows[0].text, "Use $state", "summary text used");
        assert_eq!(rows[0].count, 2);
        assert_eq!(rows[0].instances.as_array().unwrap().len(), 2);
    }

    #[test]
    fn build_rows_falls_back_to_snippet_without_summary() {
        let p = uuid::Uuid::new_v4();
        let items = vec![item(p, 100, "revert that"), item(p, 200, "revert that please")];
        let clusters = vec![Cluster {
            signature: "corr-c".into(),
            representative_text: "revert that".into(),
            count: 2,
            last_seen_ms: 200,
            project_ids: vec![p],
            member_idxs: vec![0, 1],
        }];
        let rows = build_rows(&clusters, &[None], &items, 2);
        assert_eq!(rows[0].text, "revert that", "snippet fallback");
        assert_eq!(rows[0].suggestion, None);
        assert_eq!(rows[0].memory_id, None);
    }
}
