//! Relay lane-lease sweeper (slice 1, item 2; queue cutover): every pass
//! expires lapsed lane leases and suspends their lanes' work, ends limit
//! pauses at their resume time, re-offers tasks past the grace back to the
//! pool, releases abandoned review claims, evicts dead lanes' wait holds,
//! nudges everyone about reviews floating too long, hands the oldest ready
//! broker task to the gate, and completes sprint runs whose every task is
//! done. The `relay_lease` schedule owns the cadence (60 s seeded: the
//! 300 s TTL over five); a missing relay schema (pre-convergence databases)
//! warns, never fatals.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::api::events::{LaneChange, LaneReason, RunChange, StateEvent};
use crate::db::pg_store::PgStore;
use crate::db::pg_store::relay::{RelaySweepReport, RelaySweptTask};
use crate::relay::RelayError;
use crate::relay::gate::GateRunner;
use crate::relay::rows::{decision_asked, with_lane_row, with_run_row};

/// One pass at wall-clock now.
pub async fn tick<G: GateRunner>(
    pg: Arc<PgStore>,
    event_tx: &broadcast::Sender<StateEvent>,
    gate: &G,
) -> Result<(), String> {
    match pass(&pg, event_tx, gate, chrono::Utc::now()).await {
        Ok(rep) => {
            let resumed_lanes = rep.resumed_lanes.len() as u64;
            let completed_runs = rep.completed_runs.len() as u64;
            let gate_tasks = rep.gate_tasks.len() as u64;
            let did = rep.expired_leases
                + rep.interrupted_tasks
                + rep.reoffered_tasks
                + resumed_lanes
                + rep.released_reviews
                + rep.evicted_waiters
                + rep.nudged
                + completed_runs
                + gate_tasks;
            if did > 0 {
                tracing::info!(
                    expired = rep.expired_leases,
                    interrupted = rep.interrupted_tasks,
                    reoffered = rep.reoffered_tasks,
                    resumed_lanes,
                    released_reviews = rep.released_reviews,
                    evicted_waiters = rep.evicted_waiters,
                    nudged = rep.nudged,
                    completed_runs,
                    gate_tasks,
                    "relay_lease: swept",
                );
            }
            Ok(())
        }
        Err(crate::relay::RelayError::Internal(detail)) if is_missing_schema(&detail) => {
            Err("relay_lease: relay schema not converged yet".to_string())
        }
        Err(other) => Err(format!("relay_lease: sweep failed: {other:?}")),
    }
}

/// A full pass at a fed `now`: the sweep, then the gate for every gate
/// task the sweep handed over (one per pass), each outcome written and
/// announced as `relay_task("gate", <agent task>)`. The gate runs after
/// the sweep's commit; a runner failure is the outcome's to record (the
/// gate task blocks behind an ask), while a hand-over or write the store
/// refuses is the pass's error — the ticker logs it, and the next pass
/// selects the task again.
pub async fn pass<G: GateRunner>(
    pg: &PgStore,
    event_tx: &broadcast::Sender<StateEvent>,
    gate: &G,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<RelaySweepReport, RelayError> {
    let report = sweep_at(pg, event_tx, now).await?;
    for gate_task in &report.gate_tasks {
        let handed = pg.relay_gate_task(*gate_task).await?;
        let outcome = gate.run(&handed.spec.input()).await;
        // The gate took its own time: its outcome is stamped when it landed.
        let written = pg.relay_gate_outcome(*gate_task, &outcome, chrono::Utc::now()).await?;
        tracing::info!(
            %gate_task,
            task = %written.task_id,
            state = ?written.state,
            outcome = ?outcome,
            "relay_lease: gate ran",
        );
        let _ = event_tx.send(StateEvent::relay_task("gate", &written.task_id));
        // A gate that could not run asks the person: Needs you gains it (#418).
        if let Some(ask) = written.asked
            && let Some(event) = decision_asked(pg, &ask, &written.crew_id).await
        {
            let _ = event_tx.send(event);
        }
    }
    Ok(report)
}

/// One pass at a fed `now`, announcing what it changed on `event_tx`:
/// each expired lane (its row shows suspended) and each task it moved.
pub async fn sweep_at(
    pg: &PgStore,
    event_tx: &broadcast::Sender<StateEvent>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<RelaySweepReport, RelayError> {
    let report = pg.relay_sweep_leases(now).await?;
    for event in sweep_events(&report) {
        // Lane and run events carry their Now rows (#418).
        let id = |key: &str| -> Option<uuid::Uuid> {
            event.data.get(key).and_then(|v| v.as_str()).and_then(|id| id.parse().ok())
        };
        let event = match (event.entity.as_str(), id("lane_id"), id("run_id")) {
            ("lane", Some(lane_id), _) => with_lane_row(pg, event, &lane_id).await,
            ("run", _, Some(run_id)) => with_run_row(pg, event, &run_id).await,
            _ => event,
        };
        let _ = event_tx.send(event);
    }
    Ok(report)
}

/// The events a pass's report stands for, in pass order: lanes first
/// (suspended, then resumed), then tasks, then the runs that completed.
/// A gate hand-over is announced by the gate's own outcome, not here.
fn sweep_events(report: &RelaySweepReport) -> Vec<StateEvent> {
    let suspended = report.expired_lanes.iter().map(|(crew_id, lane_id)| {
        StateEvent::relay_lane(LaneChange::Suspended(LaneReason::LeaseExpired), lane_id, crew_id)
    });
    let resumed = report.resumed_lanes.iter().map(|(crew_id, lane_id)| {
        StateEvent::relay_lane(LaneChange::Resumed(LaneReason::LimitPauseEnded), lane_id, crew_id)
    });
    let tasks = report.task_changes.iter().map(|(task_id, change)| {
        let op = match change {
            RelaySweptTask::Interrupted => "interrupt",
            RelaySweptTask::Reoffered => "reoffer",
            RelaySweptTask::ReviewReleased => "release",
        };
        StateEvent::relay_task(op, task_id)
    });
    let runs = report
        .completed_runs
        .iter()
        .map(|(run_id, crew_id)| StateEvent::relay_run(RunChange::Completed, run_id, crew_id));
    suspended.chain(resumed).chain(tasks).chain(runs).collect()
}

fn is_missing_schema(detail: &str) -> bool {
    detail.contains("does not exist") || detail.contains("undefined_table")
}

pub fn spawn<G: GateRunner + 'static>(
    pg: Arc<PgStore>,
    event_tx: broadcast::Sender<StateEvent>,
    gate: Arc<G>,
) {
    tokio::spawn(async move {
        crate::tasks::ticker::run_scheduled(pg.clone(), "relay_lease", move || {
            let pg = pg.clone();
            let event_tx = event_tx.clone();
            let gate = gate.clone();
            async move { tick(pg, &event_tx, gate.as_ref()).await }
        })
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_events_name_every_lane_task_and_run_change() {
        let (crew, lane, resumed, run) = (
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
            uuid::Uuid::now_v7(),
        );
        let tasks: Vec<uuid::Uuid> = (0..3).map(|_| uuid::Uuid::now_v7()).collect();
        let report = RelaySweepReport {
            expired_leases: 1,
            interrupted_tasks: 1,
            reoffered_tasks: 1,
            expired_resource_leases: 0,
            granted_from_queue: 0,
            expired_lanes: vec![(crew, lane)],
            resumed_lanes: vec![(crew, resumed)],
            released_reviews: 1,
            evicted_waiters: 0,
            nudged: 0,
            task_changes: vec![
                (tasks[0], RelaySweptTask::Interrupted),
                (tasks[1], RelaySweptTask::Reoffered),
                (tasks[2], RelaySweptTask::ReviewReleased),
            ],
            completed_runs: vec![(run, crew)],
            gate_tasks: vec![],
        };
        let events: Vec<serde_json::Value> =
            sweep_events(&report).iter().map(|e| serde_json::to_value(e).unwrap()).collect();
        // Lanes (suspended, then resumed), tasks, runs — in pass order.
        assert_eq!(events.len(), 6);
        assert_eq!(
            events[0],
            serde_json::to_value(StateEvent::relay_lane(
                LaneChange::Suspended(LaneReason::LeaseExpired),
                &lane,
                &crew
            ))
            .unwrap()
        );
        assert_eq!(
            events[1],
            serde_json::to_value(StateEvent::relay_lane(
                LaneChange::Resumed(LaneReason::LimitPauseEnded),
                &resumed,
                &crew
            ))
            .unwrap()
        );
        for (event, (task, op)) in
            events[2..5].iter().zip(tasks.iter().zip(["interrupt", "reoffer", "release"]))
        {
            assert_eq!(event["entity"], "task");
            assert_eq!(event["data"]["op"], op);
            assert_eq!(event["data"]["task_id"], task.to_string());
        }
        assert_eq!(
            events[5],
            serde_json::to_value(StateEvent::relay_run(RunChange::Completed, &run, &crew)).unwrap()
        );
    }

    #[test]
    fn an_idle_pass_has_no_events() {
        let report = RelaySweepReport {
            expired_leases: 0,
            interrupted_tasks: 0,
            reoffered_tasks: 0,
            expired_resource_leases: 0,
            granted_from_queue: 0,
            expired_lanes: vec![],
            resumed_lanes: vec![],
            released_reviews: 0,
            evicted_waiters: 0,
            nudged: 0,
            task_changes: vec![],
            completed_runs: vec![],
            gate_tasks: vec![],
        };
        assert!(sweep_events(&report).is_empty());
    }
}
