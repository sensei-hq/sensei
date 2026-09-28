//! `AggregateToolInsights` task handler.
//!
//! Reads the flat per-tool call/error/duration stats from
//! `sensei.tool_usage_stats`, runs the shared derivation from
//! `api::handlers::tool_signals`, and materialises one row per tool into
//! `sensei.tool_insights`. Append-only — historical rows are preserved for
//! trend charts; the observatory reader picks the latest row per tool.
//!
//! Scoped as a "global" task (folder/name blank) so a single tick writes
//! the whole cache in one shot.

use super::super::Task;
use super::super::executor::TaskContext;
use crate::analysis::narration_cache::{CopyLimits, generate_and_cache};
use crate::api::handlers::tool_signals::{
    Signal, SignalThresholds, SignalVariant, ToolUsageRow, derive_signals, signal_copy_inputs,
};

/// Verdict-window for the Health tab metrics — 14 days matches the ticket
/// text ("14d trend"). Kept `pub const` so the aggregator + any future
/// per-project override key off the same constant (#84 T2 Slice D).
pub const HEALTH_VERDICT_WINDOW_DAYS: i32 = 14;

pub async fn aggregate_tool_insights(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    // 1. Pull the current per-tool aggregate.
    let raw_rows =
        ctx.pg().get_tool_usage_stats().await.map_err(|e| format!("get_tool_usage_stats: {e}"))?;

    let typed: Vec<ToolUsageRow> =
        raw_rows.iter().filter_map(|v| serde_json::from_value(v.clone()).ok()).collect();

    // 2. Run the derivation with default thresholds.
    let signals = derive_signals(&typed, chrono::Utc::now(), &SignalThresholds::default());
    let signals_by_tool: std::collections::HashMap<&str, &Signal> =
        signals.iter().map(|s| (s.tool_name.as_str(), s)).collect();

    // 2b. Verdict split per tool over the last N days (#84 T2 Slice D / #90).
    // Zero-row tools land with `usedPct=0` in the metrics; explicit is
    // better than an absent field for the frontend variant selector.
    let split_rows = ctx
        .pg()
        .get_verdict_split_per_tool(HEALTH_VERDICT_WINDOW_DAYS)
        .await
        .map_err(|e| format!("get_verdict_split_per_tool: {e}"))?;
    let split_by_tool: std::collections::HashMap<String, (i64, i64, i64)> =
        split_rows.into_iter().map(|(t, u, p, i)| (t, (u, p, i))).collect();

    // 3. Write one row per tool. Every tool from the raw aggregate gets a
    //    row so the reader can find "healthy tools" too — even those with
    //    no active signal card.
    let mut written: u32 = 0;
    for (row, raw) in typed.iter().zip(raw_rows.iter()) {
        let mut metrics = build_metrics(row, raw);
        // Merge in the verdict split. Reach for the sensible zero fallback
        // rather than absent-field so the UI variant selector always
        // finds `usedPct` / `partialPct` / `ignoredPct` keys.
        let (used, partial, ignored) =
            split_by_tool.get(row.tool_name.as_str()).copied().unwrap_or((0, 0, 0));
        let total = used + partial + ignored;
        let (used_pct, partial_pct, ignored_pct) = if total > 0 {
            (
                used as f64 / total as f64,
                partial as f64 / total as f64,
                ignored as f64 / total as f64,
            )
        } else {
            (0.0, 0.0, 0.0)
        };
        if let Some(obj) = metrics.as_object_mut() {
            obj.insert("usedCount".into(), used.into());
            obj.insert("partialCount".into(), partial.into());
            obj.insert("ignoredCount".into(), ignored.into());
            obj.insert("verdictTotal".into(), total.into());
            obj.insert("usedPct".into(), used_pct.into());
            obj.insert("partialPct".into(), partial_pct.into());
            obj.insert("ignoredPct".into(), ignored_pct.into());
            obj.insert("verdictWindowDays".into(), HEALTH_VERDICT_WINDOW_DAYS.into());
        }

        let signal = signals_by_tool.get(row.tool_name.as_str()).copied();
        ctx.pg()
            .insert_tool_insight(&row.tool_name, &metrics, signal)
            .await
            .map_err(|e| format!("insert_tool_insight({}): {e}", row.tool_name))?;
        written += 1;
    }

    // 4. Eager-warm the mentor-voice copy for each per-tool signal so the wire
    //    read (`GET /api/observatory/tool-signals`) is a pure cache hit for warns
    //    and opportunities in steady state. This is OFF the hot path (a scheduled
    //    task, not a request). `generate_and_cache` logs + trips its own 60s
    //    back-off on a gateway failure and never errors — a down model simply
    //    leaves the deterministic template in place, so we continue the tick.
    //    Summary cards are wire-only (produced by curation) and warm on first miss.
    for s in &signals {
        let (kind, facts, _fallback) = signal_copy_inputs(s);
        let _ = generate_and_cache(
            ctx.pg(),
            &ctx.app_state.gateway,
            kind,
            &facts,
            CopyLimits::default(),
        )
        .await;
    }

    tracing::info!(
        written,
        tools = typed.len(),
        signals = signals.len(),
        window_days = HEALTH_VERDICT_WINDOW_DAYS,
        "aggregate_tool_insights: wrote snapshot",
    );
    Ok(written)
}

/// Build the `metrics` JSON body persisted alongside the signal — the raw
/// aggregate row plus a derived `error_rate` so readers don't recompute it.
fn build_metrics(row: &ToolUsageRow, raw: &serde_json::Value) -> serde_json::Value {
    let error_rate =
        if row.call_count > 0 { row.error_count as f64 / row.call_count as f64 } else { 0.0 };
    serde_json::json!({
        "callCount":      row.call_count,
        "errorCount":     row.error_count,
        "errorRate":      error_rate,
        "avgDurationMs":  raw.get("avg_duration_ms").cloned(),
        "lastUsedAt":     row.last_used_at,
    })
}

/// String form of a `SignalVariant`, used by the pg_store writer since it
/// prefers plain SQL strings over the serde enum tag. Kept next to the
/// handler so the same file is the single owner of the variant→string
/// mapping the DB persists.
pub fn variant_str(v: SignalVariant) -> &'static str {
    match v {
        SignalVariant::Warn => "warn",
        SignalVariant::Opportunity => "opportunity",
        SignalVariant::Unused => "unused",
        SignalVariant::Win => "win",
    }
}
