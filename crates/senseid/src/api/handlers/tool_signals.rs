//! Tool-usage signal derivation and Health-tab curation.
//!
//! Turns the flat per-tool call/error/duration/last-used-at stats produced
//! by `pg_store::get_tool_usage_stats` into insight cards for the
//! observatory Insights tab. Two stages:
//!
//! 1. [`derive_signals`] — one signal per tool that deserves one. Persisted
//!    to `sensei.tool_insights` so the per-tool detail pane can look them
//!    up individually.
//! 2. [`curate_insights`] — reduces the flat list to the selective,
//!    actionable set the mockup shows in the Insights strip: every warn and
//!    opportunity survives (they need action per-tool), dormants collapse
//!    to a single summary if more than one exists, and wins collapse to a
//!    single summary if more than one exists. This is what a 50-tool
//!    registry with 40 dormants should render — one dormant card, not 40.
//!
//! Signal vocabulary matches the mockup's `SignalCard variant`:
//! - `warn`        — high traffic + noticeable error rate. Users hit it.
//! - `opportunity` — moderate traffic + noticeable error rate. Room to grow.
//! - `unused`      — no activity in the last month (`unused_days`).
//! - `win`         — high traffic + clean. A workhorse.

use crate::analysis::narration_cache::{FallbackCopy, InsightKind};
use serde::{Deserialize, Serialize};

/// Raw tool-usage row as decoded from `sensei.tool_usage_stats`. Only the
/// fields the derivation reads — avg_duration_ms and other stats stay on
/// the raw JSON pass-through for the observatory table.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolUsageRow {
    pub tool_name: String,
    pub call_count: i64,
    pub error_count: i64,
    /// ISO-8601 timestamp of the tool's last observed use.
    pub last_used_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalVariant {
    Win,
    Warn,
    Opportunity,
    Unused,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Signal {
    /// Empty string for aggregate summary cards (e.g. "40 tools dormant").
    pub tool_name: String,
    pub variant: SignalVariant,
    pub title: String,
    pub detail: String,
    /// Optional next-step hint — the small right-aligned button on the
    /// mockup's SignalCard. `None` when the card is informational only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    // ── Raw metrics for the narration-cache facts builder ──────────────────────
    // Carried so [`signal_copy_inputs`] can hand stable facts to the mentor-voice
    // pipeline without re-parsing the rendered title/detail. NOT serialized —
    // the wire shape stays `{ tool_name, variant, title, detail, action? }`.
    /// Total calls (per-tool cards). `0` on summary cards.
    #[serde(skip)]
    pub calls: i64,
    /// Errored calls (per-tool cards). `0` on summary cards.
    #[serde(skip)]
    pub errors: i64,
    /// `errors / calls` (per-tool cards). `0.0` on summary cards.
    #[serde(skip)]
    pub error_rate: f64,
    /// Days since last use (per-tool cards). `0` on summary cards.
    #[serde(skip)]
    pub days_since_last_use: i64,
    /// How many tools this aggregate card rolls up. `0` on per-tool cards.
    #[serde(skip)]
    pub summary_count: i64,
    /// Up-to-three short tool names sampled into an aggregate card. Empty on
    /// per-tool cards.
    #[serde(skip)]
    pub summary_sample: Vec<String>,
}

/// Derivation thresholds. Extracted so tests can pin known-good values
/// and future tuning is one edit.
pub struct SignalThresholds {
    /// A tool with `call_count >= high_traffic_calls` counts as "high traffic".
    pub high_traffic_calls: i64,
    /// A tool with `call_count >= moderate_traffic_calls` counts as
    /// "moderate traffic". Must be smaller than `high_traffic_calls`.
    pub moderate_traffic_calls: i64,
    /// Error rate below this counts as clean. Used to earn a `win`.
    pub clean_error_rate: f64,
    /// Error rate at or above this raises a `warn` / `opportunity`.
    pub high_error_rate: f64,
    /// Days since `last_used_at` before we call a tool `unused`. 30 (a month) —
    /// 14 flagged weekly-cadence tools (e.g. `get_rules`, used per session) as
    /// dormant noise (#98); a full month idle is a truer "is this still needed?".
    pub unused_days: i64,
}

impl Default for SignalThresholds {
    fn default() -> Self {
        Self {
            high_traffic_calls: 50,
            moderate_traffic_calls: 10,
            clean_error_rate: 0.02,
            high_error_rate: 0.05,
            unused_days: 30,
        }
    }
}

/// Derive per-tool signals from a slice of raw usage rows. One signal per
/// tool that deserves one. The output is unsorted — [`curate_insights`]
/// takes care of ordering and roll-up for the Insights strip; the
/// per-tool insights writer stores each row unmodified.
pub fn derive_signals(
    stats: &[ToolUsageRow],
    now: chrono::DateTime<chrono::Utc>,
    t: &SignalThresholds,
) -> Vec<Signal> {
    let mut out: Vec<Signal> = Vec::new();

    for row in stats {
        let tool = row.tool_name.as_str();
        let short = short_name(tool);
        let calls = row.call_count.max(0);
        let errors = row.error_count.max(0);
        let error_rate = if calls == 0 { 0.0 } else { errors as f64 / calls as f64 };
        let last_used = chrono::DateTime::parse_from_rfc3339(&row.last_used_at)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc));
        let days_since_last_use = last_used.map(|last| (now - last).num_days()).unwrap_or(i64::MAX);

        if calls == 0 || days_since_last_use >= t.unused_days {
            let detail = if calls == 0 {
                format!(
                    "Registered but never called. Either wire {short} into a skill or persona, or archive it."
                )
            } else {
                format!(
                    "No calls in the last {days_since_last_use} days ({calls} total). Either wire it into a skill or persona, or archive it."
                )
            };
            out.push(Signal {
                tool_name: tool.into(),
                variant: SignalVariant::Unused,
                title: format!("{short}: dormant"),
                detail,
                action: Some(format!("Trace: why is {short} unused?")),
                calls,
                errors,
                error_rate,
                days_since_last_use,
                summary_count: 0,
                summary_sample: Vec::new(),
            });
            continue;
        }

        if calls >= t.high_traffic_calls && error_rate >= t.high_error_rate {
            out.push(Signal {
                tool_name: tool.into(),
                variant:   SignalVariant::Warn,
                title:     format!("{short}: {rate}% failure rate", rate = pct(error_rate)),
                detail: format!(
                    "{calls} calls, {errors} errored. High-traffic tool with sharp edges — fix these first.",
                ),
                action: Some(format!("Edit tool: {tool}")),
                calls, errors, error_rate, days_since_last_use,
                summary_count: 0, summary_sample: Vec::new(),
            });
            continue;
        }

        if calls >= t.moderate_traffic_calls && error_rate >= t.high_error_rate {
            out.push(Signal {
                tool_name: tool.into(),
                variant: SignalVariant::Opportunity,
                title: format!("{short}: room to improve"),
                detail: format!(
                    "{calls} calls, {rate}% failure. Modest volume — small polish would pay off.",
                    rate = pct(error_rate),
                ),
                action: Some(format!("Edit tool: {tool}")),
                calls,
                errors,
                error_rate,
                days_since_last_use,
                summary_count: 0,
                summary_sample: Vec::new(),
            });
            continue;
        }

        if calls >= t.high_traffic_calls && error_rate <= t.clean_error_rate {
            out.push(Signal {
                tool_name: tool.into(),
                variant: SignalVariant::Win,
                title: format!("{short}: workhorse"),
                detail: format!(
                    "{calls} calls, {rate}% failure rate — well-oiled.",
                    rate = pct(error_rate),
                ),
                action: None,
                calls,
                errors,
                error_rate,
                days_since_last_use,
                summary_count: 0,
                summary_sample: Vec::new(),
            });
            continue;
        }
    }

    out
}

/// Curate a per-tool signal list into the selective set the Health-tab
/// Insights strip renders. Warns and opportunities survive as-is (they
/// need action per-tool). Multiple dormants collapse to a single summary
/// card. Multiple wins collapse to a single summary card. The result is
/// sorted warn → opportunity → unused → win.
///
/// Pure: no DB, no clock. Takes an owned Vec because it re-orders.
pub fn curate_insights(signals: Vec<Signal>) -> Vec<Signal> {
    let mut warns: Vec<Signal> = Vec::new();
    let mut opportunities: Vec<Signal> = Vec::new();
    let mut unused: Vec<Signal> = Vec::new();
    let mut wins: Vec<Signal> = Vec::new();

    for s in signals {
        match s.variant {
            SignalVariant::Warn => warns.push(s),
            SignalVariant::Opportunity => opportunities.push(s),
            SignalVariant::Unused => unused.push(s),
            SignalVariant::Win => wins.push(s),
        }
    }

    let mut out: Vec<Signal> = Vec::new();
    out.append(&mut warns);
    out.append(&mut opportunities);

    if unused.len() > 1 {
        out.push(summarise_unused(&unused));
    } else if let Some(one) = unused.into_iter().next() {
        out.push(one);
    }

    if wins.len() > 1 {
        out.push(summarise_wins(&wins));
    } else if let Some(one) = wins.into_iter().next() {
        out.push(one);
    }

    out
}

/// Reduce N (>1) dormant signals into a single summary card. Lists up to
/// three tool names in the detail so the reader sees who's dormant
/// without expanding the table.
fn summarise_unused(dormants: &[Signal]) -> Signal {
    let n = dormants.len();
    let names: Vec<&str> = dormants.iter().take(3).map(|s| short_name(&s.tool_name)).collect();
    let sample = names.join(", ");
    let remainder = n.saturating_sub(3);
    let list = if remainder > 0 { format!("{sample} and {remainder} more") } else { sample };
    Signal {
        tool_name: String::new(),
        variant: SignalVariant::Unused,
        title: format!("{n} tools dormant"),
        detail: format!(
            "{list} haven't been called in the last two weeks. Either wire them into a skill or persona, or archive them.",
        ),
        action: Some("Review tool registry".into()),
        calls: 0,
        errors: 0,
        error_rate: 0.0,
        days_since_last_use: 0,
        summary_count: n as i64,
        summary_sample: names.iter().map(|s| s.to_string()).collect(),
    }
}

/// Reduce N (>1) win signals into a single summary card. Cheap
/// confirmation that the toolchain has healthy backbone tools.
fn summarise_wins(wins: &[Signal]) -> Signal {
    let n = wins.len();
    let names: Vec<&str> = wins.iter().take(3).map(|s| short_name(&s.tool_name)).collect();
    let sample = names.join(", ");
    let remainder = n.saturating_sub(3);
    let list = if remainder > 0 { format!("{sample} and {remainder} more") } else { sample };
    Signal {
        tool_name: String::new(),
        variant: SignalVariant::Win,
        title: format!("{n} workhorse tools"),
        detail: format!("{list} are running high-volume with clean error rates."),
        action: None,
        calls: 0,
        errors: 0,
        error_rate: 0.0,
        days_since_last_use: 0,
        summary_count: n as i64,
        summary_sample: names.iter().map(|s| s.to_string()).collect(),
    }
}

/// Compact display name for a tool. Strips the `sensei.` or `mcp__…__`
/// prefix so titles stay readable in a small card. Falls back to the
/// original string when neither prefix is present.
fn short_name(tool: &str) -> &str {
    if let Some(rest) = tool.strip_prefix("sensei.") {
        return rest;
    }
    // Common MCP namespacing: `mcp__plugin_x__tool` or `mcp__x__tool`.
    if let Some(rest) = tool.strip_prefix("mcp__") {
        if let Some(after) = rest.find("__").map(|i| &rest[i + 2..]) {
            return after;
        }
        return rest;
    }
    tool
}

fn pct(f: f64) -> i64 {
    (f * 100.0).round() as i64
}

/// Round an error-rate fraction to two decimals so the narration-cache facts stay
/// stable and human-clean (`0.10`, not `0.10344…`). Two decimals is
/// whole-percent granularity, matching the integer percentage the templates
/// render — a facts_hash cannot drift on inference-noise-sized fraction diffs.
fn round2(f: f64) -> f64 {
    (f * 100.0).round() / 100.0
}

/// Map one curated [`Signal`] to the inputs the mentor-voice narration-cache
/// pipeline needs: the [`InsightKind`], the stable `facts` object, and the
/// deterministic [`FallbackCopy`] (the current template text).
///
/// Pure — no DB, no gateway. The same mapping feeds both the wire read
/// (`copy_or_warm` in `observatory::tool_signals`) and the eager warm
/// (`generate_and_cache` in `tasks::tool_insights`) so a warmed row is a guaranteed
/// cache hit on the wire.
///
/// Facts are code-owned and STABLE: they never carry `action`/`variant` (those
/// are code-owned and would poison the `(kind, facts_hash)` cache key). Per-tool
/// cards key on the metrics; summary cards key on `{ count, sample }`.
pub fn signal_copy_inputs(s: &Signal) -> (InsightKind, serde_json::Value, FallbackCopy) {
    let is_summary = s.tool_name.is_empty();
    let kind = match s.variant {
        SignalVariant::Warn => InsightKind::ToolWarn,
        SignalVariant::Opportunity => InsightKind::ToolOpportunity,
        SignalVariant::Unused => {
            if is_summary {
                InsightKind::ToolsDormantSummary
            } else {
                InsightKind::ToolDormant
            }
        }
        SignalVariant::Win => {
            if is_summary {
                InsightKind::ToolsWorkhorseSummary
            } else {
                InsightKind::ToolWorkhorse
            }
        }
    };

    let facts = if is_summary {
        serde_json::json!({
            "count":  s.summary_count,
            "sample": s.summary_sample,
        })
    } else {
        serde_json::json!({
            "short":               short_name(&s.tool_name),
            "tool":                s.tool_name,
            "calls":               s.calls,
            "errors":              s.errors,
            "error_rate":          round2(s.error_rate),
            "days_since_last_use": s.days_since_last_use,
        })
    };

    let fallback = FallbackCopy { title: s.title.clone(), detail: s.detail.clone() };
    (kind, facts, fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    fn row(name: &str, calls: i64, errors: i64, days_ago: i64) -> ToolUsageRow {
        let last = now() - chrono::Duration::days(days_ago);
        ToolUsageRow {
            tool_name: name.into(),
            call_count: calls,
            error_count: errors,
            last_used_at: last.to_rfc3339(),
        }
    }

    #[test]
    fn unused_when_no_activity_in_two_weeks() {
        let stats = vec![row("sensei.cold", 30, 0, 30)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].variant, SignalVariant::Unused);
        assert_eq!(signals[0].tool_name, "sensei.cold");
        assert!(signals[0].title.contains("cold"));
        assert!(signals[0].action.as_deref().unwrap_or("").contains("cold"));
    }

    #[test]
    fn warn_title_includes_tool_name_and_failure_rate() {
        let stats = vec![row("sensei.shakey", 100, 10, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Warn);
        assert!(signals[0].title.contains("shakey"));
        assert!(signals[0].title.contains("10%"));
        assert_eq!(signals[0].action.as_deref(), Some("Edit tool: sensei.shakey"));
    }

    #[test]
    fn opportunity_when_moderate_traffic_and_high_error_rate() {
        let stats = vec![row("sensei.teetering", 20, 2, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Opportunity);
        assert!(signals[0].title.contains("teetering"));
    }

    #[test]
    fn win_when_high_traffic_and_clean() {
        let stats = vec![row("sensei.workhorse", 200, 0, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Win);
        assert!(signals[0].title.contains("workhorse"));
        assert!(signals[0].action.is_none());
    }

    #[test]
    fn no_signal_for_light_use_low_error_tools() {
        let stats = vec![row("sensei.meh", 2, 0, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert!(signals.is_empty());
    }

    #[test]
    fn zero_calls_row_becomes_unused_regardless_of_last_used() {
        let stats = vec![row("sensei.empty", 0, 0, 0)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Unused);
    }

    #[test]
    fn serializes_variant_lowercase_and_optional_action() {
        let s = Signal {
            tool_name: "x".into(),
            variant: SignalVariant::Warn,
            title: "t".into(),
            detail: "d".into(),
            action: Some("do it".into()),
            calls: 100,
            errors: 5,
            error_rate: 0.05,
            days_since_last_use: 1,
            summary_count: 0,
            summary_sample: Vec::new(),
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["variant"], "warn");
        assert_eq!(v["action"], "do it");
        // The raw metric fields are internal to the facts builder — the wire
        // shape stays { tool_name, variant, title, detail, action? }.
        assert!(v.get("calls").is_none(), "calls must not serialize onto the wire");
        assert!(v.get("errors").is_none());
        assert!(v.get("error_rate").is_none());
        assert!(v.get("days_since_last_use").is_none());
        assert!(v.get("summary_count").is_none());
        assert!(v.get("summary_sample").is_none());

        let s2 = Signal { action: None, ..s };
        let v2 = serde_json::to_value(&s2).unwrap();
        assert!(v2.get("action").is_none(), "None action should be omitted");
    }

    // ── narration-cache facts builder (pure) ───────────────────────────────────

    #[test]
    fn signal_copy_inputs_per_tool_warn_maps_and_carries_facts() {
        let stats = vec![row("sensei.shakey", 100, 10, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        let s = &signals[0];
        assert_eq!(s.variant, SignalVariant::Warn);

        let (kind, facts, fb) = signal_copy_inputs(s);
        assert_eq!(kind, InsightKind::ToolWarn);
        // Discriminating facts are present.
        assert_eq!(facts["tool"], "sensei.shakey");
        assert_eq!(facts["short"], "shakey");
        assert_eq!(facts["calls"], 100);
        assert_eq!(facts["errors"], 10);
        assert_eq!(facts["days_since_last_use"], 1);
        assert!(
            (facts["error_rate"].as_f64().unwrap() - 0.10).abs() < 1e-9,
            "error_rate rounded to 2dp"
        );
        // action / variant are code-owned — never in the facts (would poison the key).
        assert!(facts.get("action").is_none(), "action must not appear in facts");
        assert!(facts.get("variant").is_none(), "variant must not appear in facts");
        // Fallback carries the current template verbatim.
        assert_eq!(fb.title, s.title);
        assert_eq!(fb.detail, s.detail);
    }

    #[test]
    fn signal_copy_inputs_variant_to_kind_per_tool() {
        // opportunity → ToolOpportunity
        let opp = derive_signals(
            &[row("sensei.teetering", 20, 2, 1)],
            now(),
            &SignalThresholds::default(),
        );
        assert_eq!(signal_copy_inputs(&opp[0]).0, InsightKind::ToolOpportunity);
        // single win → ToolWorkhorse (per-tool, kept as-is by curate)
        let win = derive_signals(
            &[row("sensei.workhorse", 200, 0, 1)],
            now(),
            &SignalThresholds::default(),
        );
        assert_eq!(signal_copy_inputs(&win[0]).0, InsightKind::ToolWorkhorse);
        // single dormant → ToolDormant
        let cold =
            derive_signals(&[row("sensei.cold", 30, 0, 30)], now(), &SignalThresholds::default());
        assert_eq!(signal_copy_inputs(&cold[0]).0, InsightKind::ToolDormant);
    }

    #[test]
    fn signal_copy_inputs_days_change_alters_facts_hash() {
        use crate::analysis::narration_cache::facts_hash;
        let a =
            derive_signals(&[row("sensei.cold", 30, 0, 30)], now(), &SignalThresholds::default());
        let b =
            derive_signals(&[row("sensei.cold", 30, 0, 40)], now(), &SignalThresholds::default());
        let (ka, fa, _) = signal_copy_inputs(&a[0]);
        let (kb, fb, _) = signal_copy_inputs(&b[0]);
        assert_eq!(ka, kb, "same kind");
        assert_ne!(
            facts_hash(ka, &fa),
            facts_hash(kb, &fb),
            "a changed days_since_last_use must change the cache key (guards stale copy)"
        );
    }

    #[test]
    fn signal_copy_inputs_summary_dormant_uses_summary_kind_and_count() {
        let stats = vec![
            row("sensei.a", 0, 0, 0),
            row("sensei.b", 0, 0, 0),
            row("sensei.c", 0, 0, 0),
            row("sensei.d", 0, 0, 0),
        ];
        let curated = curate_insights(derive_signals(&stats, now(), &SignalThresholds::default()));
        let summary = curated
            .iter()
            .find(|s| s.variant == SignalVariant::Unused && s.tool_name.is_empty())
            .expect("collapsed dormant summary");

        let (kind, facts, fb) = signal_copy_inputs(summary);
        assert_eq!(kind, InsightKind::ToolsDormantSummary);
        assert_eq!(facts["count"], 4);
        assert_eq!(facts["sample"][0], "a", "sample threads the short tool names");
        // Summary facts use { count, sample }, not per-tool metrics.
        assert!(facts.get("calls").is_none());
        assert!(facts.get("tool").is_none());
        assert_eq!(fb.title, summary.title);
        assert_eq!(fb.detail, summary.detail);
    }

    #[test]
    fn signal_copy_inputs_summary_wins_uses_summary_kind() {
        let stats = vec![
            row("sensei.a", 200, 0, 1),
            row("sensei.b", 200, 0, 1),
            row("sensei.c", 200, 0, 1),
        ];
        let curated = curate_insights(derive_signals(&stats, now(), &SignalThresholds::default()));
        let summary = curated
            .iter()
            .find(|s| s.variant == SignalVariant::Win && s.tool_name.is_empty())
            .expect("collapsed win summary");
        let (kind, facts, _fb) = signal_copy_inputs(summary);
        assert_eq!(kind, InsightKind::ToolsWorkhorseSummary);
        assert_eq!(facts["count"], 3);
    }

    #[test]
    fn curate_collapses_multiple_dormants_into_one_summary() {
        let stats = vec![
            row("sensei.a", 0, 0, 0),
            row("sensei.b", 0, 0, 0),
            row("sensei.c", 0, 0, 0),
            row("sensei.d", 0, 0, 0),
        ];
        let raw = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(raw.len(), 4);

        let curated = curate_insights(raw);
        let dormants: Vec<&Signal> =
            curated.iter().filter(|s| s.variant == SignalVariant::Unused).collect();
        assert_eq!(dormants.len(), 1);
        assert!(dormants[0].title.contains("4 tools dormant"));
        assert!(dormants[0].detail.contains("a"));
        assert!(dormants[0].detail.contains("1 more"), "expected 'and 1 more'");
        assert_eq!(dormants[0].tool_name, "");
    }

    #[test]
    fn curate_keeps_single_dormant_as_is() {
        let stats = vec![row("sensei.only", 0, 0, 0)];
        let raw = derive_signals(&stats, now(), &SignalThresholds::default());
        let curated = curate_insights(raw);
        assert_eq!(curated.len(), 1);
        assert_eq!(curated[0].tool_name, "sensei.only");
    }

    #[test]
    fn curate_keeps_all_warns_and_opportunities() {
        let stats = vec![
            row("sensei.a", 100, 10, 1),
            row("sensei.b", 100, 10, 1),
            row("sensei.c", 20, 2, 1),
            row("sensei.d", 20, 2, 1),
        ];
        let curated = curate_insights(derive_signals(&stats, now(), &SignalThresholds::default()));
        let warns = curated.iter().filter(|s| s.variant == SignalVariant::Warn).count();
        let opps = curated.iter().filter(|s| s.variant == SignalVariant::Opportunity).count();
        assert_eq!(warns, 2);
        assert_eq!(opps, 2);
    }

    #[test]
    fn curate_collapses_multiple_wins_into_one_summary() {
        let stats = vec![
            row("sensei.a", 200, 0, 1),
            row("sensei.b", 200, 0, 1),
            row("sensei.c", 200, 0, 1),
        ];
        let curated = curate_insights(derive_signals(&stats, now(), &SignalThresholds::default()));
        let wins: Vec<&Signal> =
            curated.iter().filter(|s| s.variant == SignalVariant::Win).collect();
        assert_eq!(wins.len(), 1);
        assert!(wins[0].title.contains("3 workhorse tools"));
        assert_eq!(wins[0].tool_name, "");
    }

    #[test]
    fn curate_orders_warn_then_opportunity_then_unused_then_win() {
        let stats = vec![
            row("sensei.win", 200, 0, 1),
            row("sensei.cold1", 0, 0, 0),
            row("sensei.cold2", 0, 0, 0),
            row("sensei.warn", 100, 10, 1),
            row("sensei.opp", 20, 2, 1),
        ];
        let curated = curate_insights(derive_signals(&stats, now(), &SignalThresholds::default()));
        let variants: Vec<SignalVariant> = curated.iter().map(|s| s.variant).collect();
        assert_eq!(variants[0], SignalVariant::Warn);
        assert_eq!(variants[1], SignalVariant::Opportunity);
        assert_eq!(variants[2], SignalVariant::Unused);
        assert_eq!(variants[3], SignalVariant::Win);
    }

    #[test]
    fn short_name_strips_sensei_prefix() {
        assert_eq!(short_name("sensei.search"), "search");
        assert_eq!(short_name("mcp__plugin_x__do_it"), "do_it");
        assert_eq!(short_name("plain"), "plain");
    }
}
