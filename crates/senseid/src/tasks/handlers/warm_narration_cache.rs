//! `WarmInsightCopy` task handler — eager narration-cache generation.
//!
//! Pre-generates the mentor-voice copy for pending recommendations at analyzer
//! time (via [`crate::analysis::narration_cache::generate_and_cache`]) so the
//! Insights / Today board reads cached copy on the FIRST view — killing the
//! fallback→warm text transition and keeping inference off the request path.
//! Idempotent: a rec whose copy is already cached is skipped (doesn't count
//! toward the cap), so the pass converges across ticks and re-running is cheap.
//! The recommendation is the primary, most-visible source; memories / patterns /
//! corrections warm lazily via `copy_or_warm` as before.

use super::super::Task;
use super::super::executor::TaskContext;
use crate::analysis::narration_cache::{CopyLimits, generate_and_cache, read_cached_copy};

/// Model calls warmed per tick — bounds the (blocking, sometimes cold) embedded
/// model work under the task watchdog; the next tick warms more, and cached recs
/// are free, so all pending recs converge over a few ticks.
const WARM_CAP: usize = 20;

pub async fn warm_narration_cache(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    let recs = ctx.pg().get_insights_recommendations(None).await.unwrap_or_default();
    let gateway = &ctx.app_state.gateway;
    let mut warmed = 0u32;
    let mut attempts = 0usize;
    for r in &recs {
        if attempts >= WARM_CAP {
            break;
        }
        let (kind, facts, _fallback) = crate::insights::rec_copy_inputs(r);
        if read_cached_copy(ctx.pg(), kind, &facts).await.is_some() {
            continue; // already cached — idempotent; doesn't spend the cap
        }
        attempts += 1;
        // Off-wire, breaker-guarded: a down model returns None fast (no stall).
        if generate_and_cache(ctx.pg(), gateway, kind, &facts, CopyLimits::default())
            .await
            .is_some()
        {
            warmed += 1;
        }
    }
    tracing::info!(
        warmed,
        pending = recs.len(),
        "warm_narration_cache: eager mentor-copy warm for pending recs"
    );
    Ok(warmed)
}
