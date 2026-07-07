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
//! - `unused`      — no activity in the last two weeks.
//! - `win`         — high traffic + clean. A workhorse.

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
    /// Days since `last_used_at` before we call a tool `unused`.
    pub unused_days: i64,
}

impl Default for SignalThresholds {
    fn default() -> Self {
        Self {
            high_traffic_calls:     50,
            moderate_traffic_calls: 10,
            clean_error_rate:       0.02,
            high_error_rate:        0.05,
            unused_days:            14,
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
        let days_since_last_use = last_used
            .map(|last| (now - last).num_days())
            .unwrap_or(i64::MAX);

        if calls == 0 || days_since_last_use >= t.unused_days {
            let detail = if calls == 0 {
                format!("Registered but never called. Either wire {short} into a skill or persona, or archive it.")
            } else {
                format!("No calls in the last {days_since_last_use} days ({calls} total). Either wire it into a skill or persona, or archive it.")
            };
            out.push(Signal {
                tool_name: tool.into(),
                variant:   SignalVariant::Unused,
                title:     format!("{short}: dormant"),
                detail,
                action:    Some(format!("Trace: why is {short} unused?")),
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
            });
            continue;
        }

        if calls >= t.moderate_traffic_calls && error_rate >= t.high_error_rate {
            out.push(Signal {
                tool_name: tool.into(),
                variant:   SignalVariant::Opportunity,
                title:     format!("{short}: room to improve"),
                detail: format!(
                    "{calls} calls, {rate}% failure. Modest volume — small polish would pay off.",
                    rate = pct(error_rate),
                ),
                action: Some(format!("Edit tool: {tool}")),
            });
            continue;
        }

        if calls >= t.high_traffic_calls && error_rate <= t.clean_error_rate {
            out.push(Signal {
                tool_name: tool.into(),
                variant:   SignalVariant::Win,
                title:     format!("{short}: workhorse"),
                detail: format!(
                    "{calls} calls, {rate}% failure rate — well-oiled.",
                    rate = pct(error_rate),
                ),
                action: None,
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
    let mut warns: Vec<Signal>        = Vec::new();
    let mut opportunities: Vec<Signal>= Vec::new();
    let mut unused: Vec<Signal>       = Vec::new();
    let mut wins: Vec<Signal>         = Vec::new();

    for s in signals {
        match s.variant {
            SignalVariant::Warn        => warns.push(s),
            SignalVariant::Opportunity => opportunities.push(s),
            SignalVariant::Unused      => unused.push(s),
            SignalVariant::Win         => wins.push(s),
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
    let names: Vec<&str> = dormants.iter()
        .take(3)
        .map(|s| short_name(&s.tool_name))
        .collect();
    let sample = names.join(", ");
    let remainder = n.saturating_sub(3);
    let list = if remainder > 0 {
        format!("{sample} and {remainder} more")
    } else {
        sample
    };
    Signal {
        tool_name: String::new(),
        variant:   SignalVariant::Unused,
        title:     format!("{n} tools dormant"),
        detail: format!(
            "{list} haven't been called in the last two weeks. Either wire them into a skill or persona, or archive them.",
        ),
        action: Some("Review tool registry".into()),
    }
}

/// Reduce N (>1) win signals into a single summary card. Cheap
/// confirmation that the toolchain has healthy backbone tools.
fn summarise_wins(wins: &[Signal]) -> Signal {
    let n = wins.len();
    let names: Vec<&str> = wins.iter()
        .take(3)
        .map(|s| short_name(&s.tool_name))
        .collect();
    let sample = names.join(", ");
    let remainder = n.saturating_sub(3);
    let list = if remainder > 0 {
        format!("{sample} and {remainder} more")
    } else {
        sample
    };
    Signal {
        tool_name: String::new(),
        variant:   SignalVariant::Win,
        title:     format!("{n} workhorse tools"),
        detail: format!("{list} are running high-volume with clean error rates."),
        action:  None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-07-02T12:00:00Z").unwrap().with_timezone(&chrono::Utc)
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
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["variant"], "warn");
        assert_eq!(v["action"], "do it");

        let s2 = Signal { action: None, ..s };
        let v2 = serde_json::to_value(&s2).unwrap();
        assert!(v2.get("action").is_none(), "None action should be omitted");
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
        let dormants: Vec<&Signal> = curated.iter().filter(|s| s.variant == SignalVariant::Unused).collect();
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
        let wins: Vec<&Signal> = curated.iter().filter(|s| s.variant == SignalVariant::Win).collect();
        assert_eq!(wins.len(), 1);
        assert!(wins[0].title.contains("3 workhorse tools"));
        assert_eq!(wins[0].tool_name, "");
    }

    #[test]
    fn curate_orders_warn_then_opportunity_then_unused_then_win() {
        let stats = vec![
            row("sensei.win",         200, 0,   1),
            row("sensei.cold1",         0, 0,   0),
            row("sensei.cold2",         0, 0,   0),
            row("sensei.warn",        100, 10,  1),
            row("sensei.opp",          20, 2,   1),
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
