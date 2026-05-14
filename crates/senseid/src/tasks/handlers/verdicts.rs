//! Verdict measurement — recompute FTR deltas for accepted recommendations.

use super::super::executor::TaskContext;
use super::super::Task;

/// For each accepted recommendation with a pending verdict,
/// compute current FTR and assign positive/negative/neutral verdict.
pub async fn measure_verdicts(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    let updated = ctx.pg().measure_pending_verdicts().await
        .map_err(|e| format!("measure_pending_verdicts failed: {}", e))?;

    tracing::info!("measure_verdicts: updated {} recommendations", updated);
    Ok(updated as u32)
}
