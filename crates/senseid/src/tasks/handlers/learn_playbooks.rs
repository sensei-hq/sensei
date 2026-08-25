//! `LearnPlaybooks` task handler — §9 learning-loop analyzer stage.
//!
//! Global, idempotent: attribute confirmed `playbook_run`s' outcomes from their
//! sessions, aggregate per-(axes×playbook) FTR stats, run the pure
//! [`crate::playbook::learn`] policy (bounded reweight of existing rules +
//! proposed new learned rules), and apply the resulting plan. Safe to re-run
//! every tick — attribution only touches not-yet-attributed rows, reweights
//! are computed off the immutable `base_priority`, and proposals upsert via
//! the learned partial-unique index.

use super::super::Task;
use super::super::executor::TaskContext;

pub async fn learn_playbooks(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    // Fail closed: a transient DB error on any of these reads aborts the tick
    // (the scheduler retries next pass) rather than feeding empty stats/rules
    // into `learn` and persisting spurious reweights/proposals from them.
    let attributed = ctx.pg().attribute_playbook_outcomes().await?;
    let stats = ctx.pg().playbook_combo_stats().await?;
    let rules = ctx.pg().list_playbook_rules().await?;
    let plan = crate::playbook::learn(&stats, &rules);
    let (reweights, proposals) = (plan.reweights.len(), plan.proposals.len());
    if let Err(e) = ctx.pg().apply_learn_plan(&plan).await {
        tracing::warn!(error=%e, "learn_playbooks: apply_learn_plan failed");
    } else {
        tracing::info!(attributed, reweights, proposals, "learn_playbooks: applied");
    }
    Ok((reweights + proposals) as u32)
}
