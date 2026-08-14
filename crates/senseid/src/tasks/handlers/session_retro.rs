//! Per-session retrospective — deterministic facts → mentor-voice narrative.
//!
//! `activity.sessions.summary` had no producer, so the Sessions screens showed
//! no narrative. This module fills it off the per-session analyzer enrichment
//! tick (`analyze::enrich_session`): the session's already-computed L0 fields
//! (outcome / ftr / corrections / duration / dominant module / tool mix) plus
//! the distinct files/modules touched become a stable `facts` JSON, which is
//! routed through the [`insight_copy`](crate::analysis::insight_copy) chain to
//! produce a short mentor-voice narrative. The DETERMINISTIC facts stay code-
//! owned; only the prose sentence routes through the model.
//!
//! Graceful by construction: [`generate_session_summary`] never errors and
//! always returns a non-empty string — a gateway miss falls back to a plain
//! deterministic one-line summary ([`SessionFacts::fallback_summary`]) so
//! enrichment always has something useful to persist. The write REFRESHES on a
//! facts change (so a re-derivation can correct a now-stale line) but is a no-op
//! when unchanged — see [`crate::db::pg_store::PgStore::set_session_summary`].

use super::analyze::{parent_dir, HookEvent, SessionMetrics};
use crate::analysis::insight_copy::{generate_and_cache, CopyLimits, InsightCopy, InsightKind};
use crate::db::pg_store::PgStore;

/// How many of a session's most-used tools to name in the facts. Ranked by
/// PostToolUse count (the completed calls), so the mix reflects real work.
const TOP_TOOLS_MAX: usize = 3;

/// The deterministic per-session facts the retrospective is built from. Every
/// field is derived from L0 enrichment output — nothing here routes through a
/// model, so the same session always produces the same facts (and thus the same
/// cached copy under `facts_hash`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFacts {
    pub outcome: String,
    pub ftr: bool,
    pub corrections: i32,
    pub turns: i32,
    pub duration_min: i64,
    pub files_touched: usize,
    pub modules_touched: usize,
    pub dominant_module: Option<String>,
    /// `(tool_name, post_count)`, ranked desc, capped at [`TOP_TOOLS_MAX`].
    pub top_tools: Vec<(String, i64)>,
}

impl SessionFacts {
    /// Stable JSON fed to the insight-copy chain (and hashed for the copy
    /// cache). Key order is irrelevant — `facts_hash` canonicalises it.
    pub fn to_facts_json(&self) -> serde_json::Value {
        serde_json::json!({
            "outcome": self.outcome,
            "ftr": self.ftr,
            "corrections": self.corrections,
            "turns": self.turns,
            "duration_min": self.duration_min,
            "files_touched": self.files_touched,
            "modules_touched": self.modules_touched,
            "dominant_module": self.dominant_module,
            "top_tools": self.top_tools.iter()
                .map(|(tool, calls)| serde_json::json!({ "tool": tool, "calls": calls }))
                .collect::<Vec<_>>(),
        })
    }

    /// Deterministic one-line summary — the fallback persisted when the model is
    /// unavailable or its copy fails validation. Always non-empty (names the
    /// outcome at minimum). Reads sensibly for the 0-correction and 0-file cases.
    pub fn fallback_summary(&self) -> String {
        let corrections = match self.corrections {
            0 => "no corrections".to_string(),
            1 => "1 correction".to_string(),
            n => format!("{n} corrections"),
        };
        let scope = if self.files_touched == 0 {
            "no files touched".to_string()
        } else {
            let files = match self.files_touched {
                1 => "1 file".to_string(),
                n => format!("{n} files"),
            };
            let modules = match self.modules_touched {
                1 => "1 module".to_string(),
                n => format!("{n} modules"),
            };
            match self.dominant_module.as_deref() {
                Some(m) => format!("touched {files} across {modules} in {m}"),
                None => format!("touched {files} across {modules}"),
            }
        };
        format!("{scope}; {corrections}; outcome {}", self.outcome)
    }
}

/// Gather the retrospective facts from a session's enrichment output. Pure over
/// `(events, metrics)` — reuses the same `parent_dir` module derivation the
/// analyzer's `dominant_module` uses, so "module" means the same thing here.
pub fn gather_session_facts(events: &[HookEvent], m: &SessionMetrics) -> SessionFacts {
    let file_paths: Vec<&str> = events.iter().filter_map(|e| e.file_path.as_deref()).collect();
    let files: std::collections::HashSet<&str> = file_paths.iter().copied().collect();
    let modules: std::collections::HashSet<String> =
        file_paths.iter().filter_map(|p| parent_dir(p)).collect();
    SessionFacts {
        outcome: m.outcome.to_string(),
        ftr: m.ftr,
        corrections: m.corrections,
        turns: m.turns.len() as i32,
        duration_min: m.duration_ms / 60_000,
        files_touched: files.len(),
        modules_touched: modules.len(),
        dominant_module: m.module.clone(),
        top_tools: top_tools_from_usage(&m.tool_usage, TOP_TOOLS_MAX),
    }
}

/// Rank a session's tools by completed-call (`post`) count, desc; ties break on
/// name so the ordering is deterministic. Tools with no completed call are
/// dropped. Shape of `usage`: `{ "<tool>": { "pre", "post", "failed" } }`.
fn top_tools_from_usage(usage: &serde_json::Value, max: usize) -> Vec<(String, i64)> {
    let Some(obj) = usage.as_object() else { return Vec::new() };
    let mut tools: Vec<(String, i64)> = obj
        .iter()
        .map(|(tool, v)| (tool.clone(), v.get("post").and_then(|p| p.as_i64()).unwrap_or(0)))
        .filter(|(_, post)| *post > 0)
        .collect();
    tools.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    tools.truncate(max);
    tools
}

/// Compose the persisted narrative from the model's `{ title, detail }` copy.
/// Both fields are already voice- and length-validated by `parse_and_validate`,
/// so the common shape is `"title — detail"`. The empty branches are defensive.
pub fn compose_narrative(copy: &InsightCopy) -> String {
    let title = copy.title.trim();
    let detail = copy.detail.trim();
    match (title.is_empty(), detail.is_empty()) {
        (false, false) => format!("{title} — {detail}"),
        (false, true) => title.to_string(),
        (true, false) => detail.to_string(),
        (true, true) => String::new(),
    }
}

/// Produce the retrospective narrative for one session. EAGER path (mirrors
/// `tool_insights`' warm off the analyzer tick): awaits the insight-copy chain,
/// and on any miss/failure returns the deterministic fallback. Never errors and
/// always returns a non-empty string, so the caller can always persist it.
pub async fn generate_session_summary(
    store: &PgStore,
    gateway: &gateway::Gateway,
    facts: &SessionFacts,
) -> String {
    let facts_json = facts.to_facts_json();
    let narrative = match generate_and_cache(
        store,
        gateway,
        InsightKind::SessionRetrospective,
        &facts_json,
        CopyLimits::default(),
    )
    .await
    {
        Some(copy) => compose_narrative(&copy),
        None => String::new(),
    };
    if narrative.trim().is_empty() {
        facts.fallback_summary()
    } else {
        narrative
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::handlers::analyze::derive_session_metrics;
    use serde_json::json;

    fn facts(files: usize, modules: usize, corrections: i32, outcome: &str) -> SessionFacts {
        SessionFacts {
            outcome: outcome.to_string(),
            ftr: corrections == 0,
            corrections,
            turns: 3,
            duration_min: 12,
            files_touched: files,
            modules_touched: modules,
            dominant_module: Some("src/api".to_string()),
            top_tools: vec![("Edit".to_string(), 4), ("Read".to_string(), 2)],
        }
    }

    #[test]
    fn fallback_reads_sensibly_without_corrections() {
        let s = facts(3, 2, 0, "completed");
        assert_eq!(s.fallback_summary(), "touched 3 files across 2 modules in src/api; no corrections; outcome completed");
    }

    #[test]
    fn fallback_singularises_and_names_corrections() {
        let s = facts(1, 1, 1, "corrected");
        assert_eq!(s.fallback_summary(), "touched 1 file across 1 module in src/api; 1 correction; outcome corrected");
        let many = facts(9, 3, 4, "corrected");
        assert!(many.fallback_summary().contains("4 corrections"), "{}", many.fallback_summary());
    }

    #[test]
    fn fallback_handles_zero_files() {
        let mut s = facts(0, 0, 0, "abandoned");
        s.dominant_module = None;
        assert_eq!(s.fallback_summary(), "no files touched; no corrections; outcome abandoned");
    }

    #[test]
    fn facts_json_carries_all_fields() {
        let s = facts(3, 2, 1, "corrected");
        let v = s.to_facts_json();
        assert_eq!(v["outcome"], json!("corrected"));
        assert_eq!(v["files_touched"], json!(3));
        assert_eq!(v["modules_touched"], json!(2));
        assert_eq!(v["corrections"], json!(1));
        assert_eq!(v["dominant_module"], json!("src/api"));
        assert_eq!(v["top_tools"][0]["tool"], json!("Edit"));
        assert_eq!(v["top_tools"][0]["calls"], json!(4));
    }

    #[test]
    fn top_tools_ranks_by_post_and_drops_zero() {
        let usage = json!({
            "Edit": { "pre": 3, "post": 3, "failed": 0 },
            "Read": { "pre": 5, "post": 5, "failed": 0 },
            "Bash": { "pre": 1, "post": 0, "failed": 0 },
        });
        let top = top_tools_from_usage(&usage, 3);
        assert_eq!(top, vec![("Read".to_string(), 5), ("Edit".to_string(), 3)], "ranked desc, Bash (post=0) dropped");
    }

    #[test]
    fn top_tools_respects_cap() {
        let usage = json!({
            "A": { "post": 1 }, "B": { "post": 2 }, "C": { "post": 3 }, "D": { "post": 4 },
        });
        assert_eq!(top_tools_from_usage(&usage, 2), vec![("D".to_string(), 4), ("C".to_string(), 3)]);
    }

    #[test]
    fn compose_joins_title_and_detail() {
        let copy = InsightCopy { title: "wired the retro generator".to_string(), detail: "clean pass, one correction.".to_string() };
        assert_eq!(compose_narrative(&copy), "wired the retro generator — clean pass, one correction.");
    }

    #[test]
    fn compose_tolerates_empty_field() {
        let only_title = InsightCopy { title: "did the thing".to_string(), detail: "  ".to_string() };
        assert_eq!(compose_narrative(&only_title), "did the thing");
        let neither = InsightCopy { title: String::new(), detail: String::new() };
        assert_eq!(compose_narrative(&neither), "");
    }

    #[test]
    fn gather_counts_distinct_files_and_modules() {
        let ev = |et: &str, tool: Option<&str>, ts: i64, fp: Option<&str>| HookEvent {
            event_type: et.into(),
            tool_name: tool.map(str::to_string),
            ts,
            prompt: None,
            file_path: fp.map(str::to_string),
            tool_failed: false,
            action: None,
        };
        let events = vec![
            ev("UserPromptSubmit", None, 1000, None),
            ev("PostToolUse", Some("Edit"), 1100, Some("src/api/a.rs")),
            ev("PostToolUse", Some("Edit"), 1200, Some("src/api/b.rs")),
            ev("PostToolUse", Some("Read"), 1300, Some("src/api/a.rs")), // dup file, same module
            ev("PostToolUse", Some("Edit"), 1400, Some("src/db/c.rs")),
            ev("Stop", None, 2000, None),
        ];
        let m = derive_session_metrics(&events, &[]).unwrap();
        let f = gather_session_facts(&events, &m);
        assert_eq!(f.files_touched, 3, "a.rs, b.rs, c.rs distinct");
        assert_eq!(f.modules_touched, 2, "src/api and src/db");
        assert_eq!(f.top_tools.first().map(|(t, _)| t.as_str()), Some("Edit"), "Edit is the workhorse (post=3)");
        assert_eq!(f.outcome, "completed");
        assert!(f.ftr);
    }
}
