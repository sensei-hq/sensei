//! L2 Generator (heuristic tier, F4/#69).
//!
//! Turns the L1 [`derive_signals`](super::analyze::derive_signals) output —
//! rows in `inference.detected_patterns` — into the artifacts the Observatory
//! consumes: **recommendations** (`inference.recommendations`) and **learned
//! memories** (`sensei.memories`, `origin='learned'`).
//!
//! This tier is deterministic: the prompt classifier already refined
//! correction↔principle↔neither at L1, so here we just map each detected
//! pattern to its action. The LLM *consolidation* tier (F5/#70) is separate.
//!
//! Idempotency: the analyzer runs hourly, so the generator must not duplicate.
//! Each artifact records its source pattern — a recommendation via
//! `based_on.patterns` and a memory via `source_id` — and the generator skips a
//! pattern that already produced one.

use crate::tasks::executor::TaskContext;

/// Memory-title cap (chars) for a principle distilled into a learned rule.
const TITLE_MAX: usize = 72;
/// Correction count at/above which the recommendation is high-urgency.
const CORRECTION_HIGH_URGENCY: i32 = 4;
/// Single-session re-edits at/above which a churn hotspot earns a recommendation.
/// L1 detects churn from 5 re-edits (for the UI); only the severe ones get a rec.
const REWORK_REC_MIN_EDITS: i64 = 8;

/// A detected-pattern row as the generator sees it. `folder_label` is the
/// folder's human name/path (for titles); `instances` is the jsonb evidence
/// array L1 wrote. The planner dispatches on `name`, so anti-pattern/folder-id
/// columns aren't carried here.
#[derive(Debug, Clone)]
pub(crate) struct PatternRow {
    pub id: uuid::Uuid,
    pub folder_label: String,
    pub name: String,
    pub instance_count: i32,
    pub instances: serde_json::Value,
}

/// What the generator decides to create for a pattern (pure output).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Planned {
    Recommendation {
        source_pattern: uuid::Uuid,
        title: String,
        why: String,
        impact: Option<String>,
        action_type: String,
        urgency: String,
    },
    Memory {
        source_pattern: uuid::Uuid,
        project_scoped_filter: Option<String>,
        title: String,
        content: String,
        impact: Option<String>,
        mtype: String,
        category: String,
        triage_signal: String,
    },
}

impl Planned {
    pub(crate) fn source_pattern(&self) -> uuid::Uuid {
        match self {
            Planned::Recommendation { source_pattern, .. } => *source_pattern,
            Planned::Memory { source_pattern, .. } => *source_pattern,
        }
    }
}

/// Distil a stated principle prompt into a concise memory title: strip the
/// imperative lead-in ("you should always", "make sure to", …), trim, cap.
fn principle_title(prompt: &str) -> String {
    let lowered = prompt.trim();
    let mut rest = lowered;
    for lead in [
        "you should always",
        "you should",
        "you must always",
        "you must",
        "always make sure to",
        "always make sure",
        "make sure to",
        "make sure",
        "always",
        "never",
        "from now on,",
        "from now on",
        "please always",
        "please",
    ] {
        if rest.len() >= lead.len() && rest[..lead.len()].eq_ignore_ascii_case(lead) {
            rest = rest[lead.len()..].trim_start_matches([' ', ',', ':']);
            break;
        }
    }
    let rest = rest.trim();
    let base = if rest.is_empty() { lowered } else { rest };
    let mut title: String = base.chars().take(TITLE_MAX).collect();
    if base.chars().count() > TITLE_MAX {
        title.push('…');
    }
    // Sentence-case the first character.
    let mut chars = title.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => title,
    }
}

/// Extract the first principle prompt from a `rule-candidates` instances array.
fn first_principle(instances: &serde_json::Value) -> Option<String> {
    instances
        .as_array()?
        .iter()
        .find_map(|i| i.get("prompt").and_then(|p| p.as_str()))
        .map(|s| s.to_string())
}

/// The file a `rework: <file>` churn pattern points at.
fn rework_file(name: &str) -> Option<&str> {
    name.strip_prefix("rework: ").map(str::trim)
}

/// Max single-session re-edits recorded on a churn pattern's instances
/// (L1 writes `[{file, max_session_edits, total_edits}]`). 0 if absent.
fn rework_max_edits(instances: &serde_json::Value) -> i64 {
    instances
        .as_array()
        .and_then(|a| a.first())
        .and_then(|i| i.get("max_session_edits"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// Heuristic L2 mapping: detected patterns → recommendations + learned
/// memories. Pure + deterministic. A pattern that doesn't map to anything
/// (unknown name, empty rule-candidate) produces nothing.
pub(crate) fn plan_artifacts(patterns: &[PatternRow]) -> Vec<Planned> {
    let mut out = Vec::new();
    for p in patterns {
        match p.name.as_str() {
            // Rule-candidate: the user stated a durable principle → capture it
            // as a learned memory AND recommend promoting it to an enforced rule.
            "rule-candidates" => {
                let Some(principle) = first_principle(&p.instances) else {
                    continue;
                };
                let title = principle_title(&principle);
                out.push(Planned::Memory {
                    source_pattern: p.id,
                    project_scoped_filter: None,
                    title: title.clone(),
                    content: principle.trim().to_string(),
                    impact: Some(format!(
                        "Stated as a guiding principle across {} prompt(s).",
                        p.instance_count
                    )),
                    mtype: "convention".to_string(),
                    category: "convention".to_string(),
                    triage_signal: "repeat_pattern".to_string(),
                });
                out.push(Planned::Recommendation {
                    source_pattern: p.id,
                    title: format!("Promote learned rule: {title}"),
                    why: format!(
                        "Stated as a principle in {} prompt(s) in {}; promote it to an enforced rule.",
                        p.instance_count, p.folder_label
                    ),
                    impact: None,
                    action_type: "promote_pattern".to_string(),
                    urgency: "medium".to_string(),
                });
            }
            // Correction clustering: recurring fixes in one area → suggest a
            // guard/skill to stop the repeat.
            "correction-prone" => {
                let urgency =
                    if p.instance_count >= CORRECTION_HIGH_URGENCY { "high" } else { "medium" };
                out.push(Planned::Recommendation {
                    source_pattern: p.id,
                    title: format!("Recurring corrections in {}", p.folder_label),
                    why: format!(
                        "{} corrective prompt(s) clustered here — a guard or skill could prevent the repeat.",
                        p.instance_count
                    ),
                    impact: None,
                    action_type: "write_skill".to_string(),
                    urgency: urgency.to_string(),
                });
            }
            // Re-edit churn (file-scoped): recommend an audit only for SEVERE
            // hotspots. L1 flags churn at >= CHURN_MIN_EDITS (5) re-edits so the
            // UI can show every hot file, but a recommendation per ≥5-edit file
            // floods the inbox — only the worst (>= REWORK_REC_MIN_EDITS) earn a rec.
            other if other.starts_with("rework: ") => {
                if rework_max_edits(&p.instances) < REWORK_REC_MIN_EDITS {
                    continue;
                }
                let file = rework_file(other).unwrap_or(other);
                out.push(Planned::Recommendation {
                    source_pattern: p.id,
                    title: format!("High rework: {file}"),
                    why: "Re-edited heavily within a single session — a churn hotspot worth auditing for an unclear spec or missing test.".to_string(),
                    impact: None,
                    action_type: "audit_stale".to_string(),
                    urgency: "medium".to_string(),
                });
            }
            _ => {}
        }
    }
    out
}

/// Handler step: generate recommendations + learned memories for a project from
/// its detected patterns. Idempotent. Returns the number of new artifacts written.
pub async fn generate_for_project(
    ctx: &TaskContext,
    project_id: &uuid::Uuid,
) -> Result<u32, String> {
    let rows: Vec<PatternRow> = ctx
        .pg()
        .get_patterns_for_generation(project_id)
        .await?
        .into_iter()
        .map(|(id, _folder_id, folder_label, name, _is_anti, instance_count, instances_text)| {
            PatternRow {
                id,
                folder_label,
                name,
                instance_count,
                instances: serde_json::from_str(&instances_text)
                    .unwrap_or(serde_json::Value::Array(Vec::new())),
            }
        })
        .collect();
    let planned = plan_artifacts(&rows);
    let mut written = 0u32;

    for item in planned {
        let pattern_id = item.source_pattern();
        match item {
            Planned::Recommendation { title, why, impact, action_type, urgency, .. } => {
                // Skip if a recommendation already cites this pattern.
                if ctx
                    .pg()
                    .recommendation_exists_for_pattern(project_id, &pattern_id)
                    .await
                    .unwrap_or(false)
                {
                    continue;
                }
                let based_on = serde_json::json!({ "patterns": [pattern_id] });
                match ctx
                    .pg()
                    .create_recommendation_full(
                        project_id,
                        &title,
                        &why,
                        impact.as_deref(),
                        &action_type,
                        &urgency,
                        &based_on,
                        None, // heuristic tier — no reasoning trace
                        None, // no ready-to-send prompt
                    )
                    .await
                {
                    Ok(_) => written += 1,
                    Err(e) => {
                        tracing::warn!(error = %e, pattern = %pattern_id, "generate: recommendation insert failed")
                    }
                }
            }
            Planned::Memory {
                project_scoped_filter,
                title,
                content,
                impact,
                mtype,
                category,
                triage_signal,
                ..
            } => {
                // Skip if a learned memory already sources this pattern.
                if ctx.pg().memory_exists_with_source(&pattern_id).await.unwrap_or(false) {
                    continue;
                }
                // Analyzer memories self-anchor into the spine's design/decisions
                // slot from their category/type (project scope — no feature).
                let slot = crate::memory_slot::default_slot(Some(category.as_str()), &mtype);
                let mem = crate::db::pg_store::InsertMemory {
                    project_id: Some(*project_id),
                    scope: "project".to_string(),
                    scope_filter: project_scoped_filter,
                    mtype,
                    title,
                    content,
                    impact,
                    tags: Vec::new(),
                    triage_signal: Some(triage_signal),
                    status: "active".to_string(),
                    namespace_id: None,
                    enforcement: None,
                    origin: Some("learned".to_string()),
                    source_id: Some(pattern_id),
                    spine_slot: Some(slot.as_str().to_string()),
                    feature: None,
                };
                match ctx.pg().insert_memory(&mem).await {
                    Ok(id) => {
                        if let Err(e) = ctx.pg().set_memory_category(&id, &category).await {
                            tracing::warn!(error = %e, memory = %id, "generate: set_memory_category failed");
                        }
                        written += 1;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, pattern = %pattern_id, "generate: memory insert failed")
                    }
                }
            }
        }
    }

    if written > 0 {
        tracing::info!("generate_for_project: {} — {} new artifacts", project_id, written);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, _anti: bool, count: i32, instances: serde_json::Value) -> PatternRow {
        PatternRow {
            id: uuid::Uuid::new_v4(),
            folder_label: "src/api".to_string(),
            name: name.to_string(),
            instance_count: count,
            instances,
        }
    }

    #[test]
    fn principle_title_strips_imperative_and_sentence_cases() {
        assert_eq!(principle_title("you should always run the tests first"), "Run the tests first");
        assert_eq!(principle_title("make sure to use dbd deploy"), "Use dbd deploy");
        assert_eq!(principle_title("never edit generated files"), "Edit generated files"); // lead stripped
        // Falls back to the whole prompt when no lead-in matches.
        assert_eq!(
            principle_title("prefer composition over inheritance"),
            "Prefer composition over inheritance"
        );
    }

    #[test]
    fn principle_title_caps_length() {
        let long = "you should always ".to_string() + &"x".repeat(200);
        let t = principle_title(&long);
        assert!(t.chars().count() <= TITLE_MAX + 1, "got {} chars", t.chars().count());
        assert!(t.ends_with('…'));
    }

    #[test]
    fn rule_candidate_yields_memory_plus_promote_recommendation() {
        let instances = serde_json::json!([
            { "session": "s1", "prompt": "you should always run the tests first" },
            { "session": "s2", "prompt": "you should always run the tests first" },
        ]);
        let planned = plan_artifacts(&[row("rule-candidates", false, 2, instances)]);
        assert_eq!(planned.len(), 2);
        let mem = planned.iter().find(|p| matches!(p, Planned::Memory { .. })).expect("memory");
        let rec =
            planned.iter().find(|p| matches!(p, Planned::Recommendation { .. })).expect("rec");
        match mem {
            Planned::Memory { title, content, mtype, category, triage_signal, .. } => {
                assert_eq!(title, "Run the tests first");
                assert_eq!(content, "you should always run the tests first");
                assert_eq!(mtype, "convention");
                assert_eq!(category, "convention");
                assert_eq!(triage_signal, "repeat_pattern");
            }
            _ => unreachable!(),
        }
        match rec {
            Planned::Recommendation { action_type, title, urgency, .. } => {
                assert_eq!(action_type, "promote_pattern");
                assert!(title.starts_with("Promote learned rule:"));
                assert_eq!(urgency, "medium");
            }
            _ => unreachable!(),
        }
        // Both cite the same source pattern (idempotency key).
        assert_eq!(mem.source_pattern(), rec.source_pattern());
    }

    #[test]
    fn empty_rule_candidate_yields_nothing() {
        assert!(
            plan_artifacts(&[row("rule-candidates", false, 0, serde_json::json!([]))]).is_empty()
        );
    }

    #[test]
    fn correction_prone_urgency_scales_with_count() {
        let inst = serde_json::json!([{ "session": "s", "prompt": "no, revert that" }]);
        let low = plan_artifacts(&[row("correction-prone", true, 2, inst.clone())]);
        let high = plan_artifacts(&[row("correction-prone", true, 5, inst)]);
        match &low[0] {
            Planned::Recommendation { action_type, urgency, title, .. } => {
                assert_eq!(action_type, "write_skill");
                assert_eq!(urgency, "medium");
                assert!(title.contains("src/api"));
            }
            _ => unreachable!(),
        }
        match &high[0] {
            Planned::Recommendation { urgency, .. } => assert_eq!(urgency, "high"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn severe_rework_churn_yields_audit_recommendation() {
        let inst = serde_json::json!([{ "file": "src/api/server.rs", "max_session_edits": 9, "total_edits": 12 }]);
        let planned = plan_artifacts(&[row("rework: src/api/server.rs", true, 1, inst)]);
        assert_eq!(planned.len(), 1);
        match &planned[0] {
            Planned::Recommendation { action_type, title, .. } => {
                assert_eq!(action_type, "audit_stale");
                assert_eq!(title, "High rework: src/api/server.rs");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn marginal_rework_churn_is_skipped() {
        // Detected as churn (>=5) but below the rec threshold (<8) → no inbox noise.
        let inst =
            serde_json::json!([{ "file": "src/x.rs", "max_session_edits": 6, "total_edits": 6 }]);
        assert!(plan_artifacts(&[row("rework: src/x.rs", true, 1, inst)]).is_empty());
    }

    #[test]
    fn unknown_pattern_is_ignored() {
        assert!(
            plan_artifacts(&[row("something-else", false, 3, serde_json::json!([]))]).is_empty()
        );
    }

    // ── DB-backed orchestrator test ──────────────────────────────────────

    use crate::tasks::test_support::make_ctx;

    #[tokio::test]
    async fn generate_writes_memory_and_recommendation_idempotently() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let pid = pg
            .create_project(&format!("_test:gen-{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let root = pg
            .add_watch_root(
                &format!("/_test/gen-root-{}", uuid::Uuid::new_v4()),
                "t",
                &serde_json::json!([]),
            )
            .await
            .unwrap();
        let fid = pg
            .upsert_repo(&root, "gen-repo", &format!("/_test/gen-{}", uuid::Uuid::new_v4()))
            .await
            .unwrap();
        // A session attributes this folder to the project (L1's attribution path).
        let csid = format!("_test-gen-sid-{}", uuid::Uuid::new_v4());
        pg.record_session_event(&csid, &fid, Some(&pid), "claude", true).await.unwrap();

        // A rule-candidate pattern (the user stated a principle) → memory + promote rec.
        let instances = serde_json::json!([{ "session": csid, "prompt": "you should always run the tests first" }]);
        let pat_id = pg
            .upsert_pattern(&pid, Some(&fid), "rule-candidates", false, None, &instances)
            .await
            .unwrap();

        // First run: writes a learned memory + a recommendation.
        let written = generate_for_project(&ctx, &pid).await.unwrap();
        assert!(written >= 2, "expected memory + recommendation, got {written}");
        assert!(
            pg.recommendation_exists_for_pattern(&pid, &pat_id).await.unwrap(),
            "recommendation cites the pattern"
        );
        assert!(
            pg.memory_exists_with_source(&pat_id).await.unwrap(),
            "learned memory sources the pattern"
        );

        // The learned memory carries the distilled title + category.
        let mem: Option<(String, Option<String>, String)> = sqlx_core::query_as::query_as(
            "SELECT title, category::text, origin FROM sensei.memories WHERE source_id = $1",
        )
        .bind(pat_id)
        .fetch_optional(pg.pool())
        .await
        .unwrap();
        let (title, category, origin) = mem.expect("memory row");
        assert_eq!(title, "Run the tests first");
        assert_eq!(category.as_deref(), Some("convention"));
        assert_eq!(origin, "learned");

        // The analyzer's real write path anchors it: category/mtype="convention"
        // → default_slot → "design" (project scope, so no feature).
        let anchor: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
            "SELECT spine_slot::text, feature FROM sensei.memories WHERE source_id = $1",
        )
        .bind(pat_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            anchor.0.as_deref(),
            Some("design"),
            "analyzer memory self-anchors into the design slot"
        );
        assert_eq!(anchor.1, None, "analyzer memories are project-scope — no feature");

        // Second run: idempotent — nothing new.
        let again = generate_for_project(&ctx, &pid).await.unwrap();
        assert_eq!(again, 0, "re-run must not duplicate");

        // cleanup (best-effort)
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM sensei.memories WHERE source_id = $1")
            .bind(pat_id)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM inference.recommendations WHERE project_id = $1")
            .bind(pid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE project_id = $1")
            .bind(pid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE client_session_id = $1")
            .bind(&csid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1")
            .bind(root)
            .execute(pool)
            .await
            .ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1")
            .bind(pid)
            .execute(pool)
            .await
            .ok();
    }
}
