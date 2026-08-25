//! `ConsolidateGovernance` task handler — the scheduled Tier-2 rule consolidation.
//!
//! Runs the same pipeline as the manual `POST /api/knowledge/rules/consolidate`
//! (via the shared [`crate::analysis::rule_consolidation`] orchestrator) so
//! `sensei.consolidated_rulesets` populates on its own as global rules change,
//! not only when a user clicks consolidate. Source-hash guarded, so a tick with
//! unchanged rules is a cheap no-op (no model call). Returns 1 when a new
//! `proposed` version was written, else 0.

use super::super::Task;
use super::super::executor::TaskContext;
use crate::analysis::rule_consolidation::{ConsolidationOutcome, consolidate_global_rules};

pub async fn consolidate_governance(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    match consolidate_global_rules(ctx.pg(), &ctx.app_state.gateway).await? {
        ConsolidationOutcome::Skipped(reason) => {
            tracing::debug!(reason, "consolidate_governance: skipped");
            Ok(0)
        }
        ConsolidationOutcome::Created { id, version, model, .. } => {
            tracing::info!(%id, version, model = ?model, "consolidate_governance: new proposed ruleset written");
            Ok(1)
        }
    }
}
