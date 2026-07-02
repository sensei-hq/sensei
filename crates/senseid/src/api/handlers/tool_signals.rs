//! Tool-usage signal derivation.
//!
//! Turns the flat per-tool call/error/duration/last-used-at stats produced
//! by `pg_store::get_tool_usage_stats` into human-facing insight cards for
//! the observatory Instruments Insights tab. Pure logic — no DB reads, no
//! side effects — so the heuristics stay unit-testable and evolve without
//! touching the request/response plumbing.
//!
//! Signal vocabulary matches the mockup's `SignalCard variant`:
//! - `win`         — high traffic + low error rate. This tool is a workhorse.
//! - `warn`        — high traffic + noticeable error rate. Users may hit it.
//! - `opportunity` — moderate traffic + noticeable error rate. Room to improve.
//! - `unused`      — no activity in the last two weeks.

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
    pub tool_name: String,
    pub variant: SignalVariant,
    pub title: String,
    pub detail: String,
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

/// Derive per-tool signals from a slice of raw usage rows. Ordered so the
/// most actionable signals (warn > opportunity > unused > win) surface first
/// — the UI can render them in that order without a client-side sort.
pub fn derive_signals(
    stats: &[ToolUsageRow],
    now: chrono::DateTime<chrono::Utc>,
    t: &SignalThresholds,
) -> Vec<Signal> {
    let mut out: Vec<Signal> = Vec::new();

    for row in stats {
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
            out.push(Signal {
                tool_name: row.tool_name.clone(),
                variant:   SignalVariant::Unused,
                title:     "Dormant tool".into(),
                detail: format!(
                    "No calls in the last {days_since_last_use} day{s} — is this tool still needed?",
                    s = if days_since_last_use == 1 { "" } else { "s" },
                ),
            });
            continue;
        }

        if calls >= t.high_traffic_calls && error_rate >= t.high_error_rate {
            out.push(Signal {
                tool_name: row.tool_name.clone(),
                variant:   SignalVariant::Warn,
                title:     "High-traffic error rate".into(),
                detail: format!(
                    "{calls} calls, {errors} errors ({rate}% failure). Fix the sharp edges here first.",
                    rate = pct(error_rate),
                ),
            });
            continue;
        }

        if calls >= t.moderate_traffic_calls && error_rate >= t.high_error_rate {
            out.push(Signal {
                tool_name: row.tool_name.clone(),
                variant:   SignalVariant::Opportunity,
                title:     "Improvable error rate".into(),
                detail: format!(
                    "{calls} calls, {rate}% failure. Small polish pays off.",
                    rate = pct(error_rate),
                ),
            });
            continue;
        }

        if calls >= t.high_traffic_calls && error_rate <= t.clean_error_rate {
            out.push(Signal {
                tool_name: row.tool_name.clone(),
                variant:   SignalVariant::Win,
                title:     "Workhorse tool".into(),
                detail: format!(
                    "{calls} calls with a {rate}% failure rate — well-oiled.",
                    rate = pct(error_rate),
                ),
            });
            continue;
        }
    }

    // Priority sort — warn > opportunity > unused > win — so the caller
    // renders the most actionable first without another pass.
    out.sort_by_key(|s| match s.variant {
        SignalVariant::Warn        => 0,
        SignalVariant::Opportunity => 1,
        SignalVariant::Unused      => 2,
        SignalVariant::Win         => 3,
    });
    out
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
        let stats = vec![row("cold", 30, 0, 30)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].variant, SignalVariant::Unused);
        assert_eq!(signals[0].tool_name, "cold");
    }

    #[test]
    fn warn_when_high_traffic_and_high_error_rate() {
        // 100 calls · 10% error → warn (high traffic + high error).
        let stats = vec![row("shakey", 100, 10, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Warn);
    }

    #[test]
    fn opportunity_when_moderate_traffic_and_high_error_rate() {
        let stats = vec![row("teetering", 20, 2, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Opportunity);
    }

    #[test]
    fn win_when_high_traffic_and_clean() {
        // 200 calls · 0 errors → win.
        let stats = vec![row("workhorse", 200, 0, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Win);
    }

    #[test]
    fn no_signal_for_light_use_low_error_tools() {
        // A tool used only twice with no errors gets no card.
        let stats = vec![row("meh", 2, 0, 1)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert!(signals.is_empty());
    }

    #[test]
    fn signals_ordered_warn_first_win_last() {
        let stats = vec![
            row("win",         200, 0,   1),
            row("cold",         30, 0,  30),
            row("shakey",      100, 10,  1),
            row("teetering",    20, 2,   1),
        ];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        let variants: Vec<SignalVariant> = signals.iter().map(|s| s.variant).collect();
        assert_eq!(
            variants,
            vec![
                SignalVariant::Warn,
                SignalVariant::Opportunity,
                SignalVariant::Unused,
                SignalVariant::Win
            ],
        );
    }

    #[test]
    fn zero_calls_row_becomes_unused_regardless_of_last_used() {
        let stats = vec![row("empty", 0, 0, 0)];
        let signals = derive_signals(&stats, now(), &SignalThresholds::default());
        assert_eq!(signals[0].variant, SignalVariant::Unused);
    }

    #[test]
    fn serializes_variant_lowercase() {
        let s = Signal {
            tool_name: "x".into(),
            variant: SignalVariant::Warn,
            title: "t".into(),
            detail: "d".into(),
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["variant"], "warn");
    }
}
