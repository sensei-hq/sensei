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

use super::super::executor::TaskContext;
use super::super::Task;
use crate::api::handlers::tool_signals::{
    derive_signals, Signal, SignalThresholds, SignalVariant, ToolUsageRow,
};

pub async fn aggregate_tool_insights(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    // 1. Pull the current per-tool aggregate.
    let raw_rows = ctx
        .pg()
        .get_tool_usage_stats()
        .await
        .map_err(|e| format!("get_tool_usage_stats: {e}"))?;

    let typed: Vec<ToolUsageRow> = raw_rows
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    // 2. Run the derivation with default thresholds.
    let signals = derive_signals(&typed, chrono::Utc::now(), &SignalThresholds::default());
    let signals_by_tool: std::collections::HashMap<&str, &Signal> = signals
        .iter()
        .map(|s| (s.tool_name.as_str(), s))
        .collect();

    // 3. Write one row per tool. Every tool from the raw aggregate gets a
    //    row so the reader can find "healthy tools" too — even those with
    //    no active signal card.
    let mut written: u32 = 0;
    for (row, raw) in typed.iter().zip(raw_rows.iter()) {
        let metrics = build_metrics(row, raw);
        let signal = signals_by_tool.get(row.tool_name.as_str()).copied();
        ctx.pg()
            .insert_tool_insight(&row.tool_name, &metrics, signal)
            .await
            .map_err(|e| format!("insert_tool_insight({}): {e}", row.tool_name))?;
        written += 1;
    }

    tracing::info!(
        written,
        tools = typed.len(),
        signals = signals.len(),
        "aggregate_tool_insights: wrote snapshot",
    );
    Ok(written)
}

/// Build the `metrics` JSON body persisted alongside the signal — the raw
/// aggregate row plus a derived `error_rate` so readers don't recompute it.
fn build_metrics(row: &ToolUsageRow, raw: &serde_json::Value) -> serde_json::Value {
    let error_rate = if row.call_count > 0 {
        row.error_count as f64 / row.call_count as f64
    } else {
        0.0
    };
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
