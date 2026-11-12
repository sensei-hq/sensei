//! Relay lane-lease sweeper (slice 1, item 2): every pass expires lapsed
//! lane leases, suspends their lanes' work, re-offers tasks past the grace
//! back to the pool, and completes sprint runs whose every task is done.
//! The `relay_lease` schedule owns the cadence (60 s seeded: the 300 s TTL
//! over five); a missing relay schema (pre-convergence databases) warns,
//! never fatals.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::api::events::{LaneChange, LaneReason, RunChange, StateEvent};
use crate::db::pg_store::PgStore;
use crate::db::pg_store::relay::{RelaySweepReport, RelaySweptTask};
use crate::relay::RelayError;

/// One pass at wall-clock now.
pub async fn tick(
    pg: Arc<PgStore>,
    event_tx: &broadcast::Sender<StateEvent>,
) -> Result<(), String> {
    match sweep_at(&pg, event_tx, chrono::Utc::now()).await {
        Ok(rep) => {
            let completed_runs = rep.completed_runs.len() as u64;
            if rep.expired_leases + rep.interrupted_tasks + rep.reoffered_tasks + completed_runs > 0
            {
                tracing::info!(
                    expired = rep.expired_leases,
                    interrupted = rep.interrupted_tasks,
                    reoffered = rep.reoffered_tasks,
                    completed_runs,
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

/// One pass at a fed `now`, announcing what it changed on `event_tx`:
/// each expired lane (its row shows suspended) and each task it moved.
pub async fn sweep_at(
    pg: &PgStore,
    event_tx: &broadcast::Sender<StateEvent>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<RelaySweepReport, RelayError> {
    let report = pg.relay_sweep_leases(now).await?;
    for event in sweep_events(&report) {
        let _ = event_tx.send(event);
    }
    Ok(report)
}

/// The events a pass's report stands for, in pass order: lanes first,
/// then tasks, then the runs that completed.
fn sweep_events(report: &RelaySweepReport) -> Vec<StateEvent> {
    let lanes = report.expired_lanes.iter().map(|(crew_id, lane_id)| {
        StateEvent::relay_lane(LaneChange::Suspended(LaneReason::LeaseExpired), lane_id, crew_id)
    });
    let tasks = report.task_changes.iter().map(|(task_id, change)| {
        let op = match change {
            RelaySweptTask::Interrupted => "interrupt",
            RelaySweptTask::Reoffered => "reoffer",
        };
        StateEvent::relay_task(op, task_id)
    });
    let runs = report
        .completed_runs
        .iter()
        .map(|(_, crew_id)| StateEvent::relay_run(RunChange::Completed, crew_id));
    lanes.chain(tasks).chain(runs).collect()
}

fn is_missing_schema(detail: &str) -> bool {
    detail.contains("does not exist") || detail.contains("undefined_table")
}

pub fn spawn(pg: Arc<PgStore>, event_tx: broadcast::Sender<StateEvent>) {
    tokio::spawn(async move {
        crate::tasks::ticker::run_scheduled(pg.clone(), "relay_lease", move || {
            let pg = pg.clone();
            let event_tx = event_tx.clone();
            async move { tick(pg, &event_tx).await }
        })
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_events_name_every_lane_task_and_run_change() {
        let (crew, lane, run) = (uuid::Uuid::now_v7(), uuid::Uuid::now_v7(), uuid::Uuid::now_v7());
        let tasks: Vec<uuid::Uuid> = (0..2).map(|_| uuid::Uuid::now_v7()).collect();
        let report = RelaySweepReport {
            expired_leases: 1,
            interrupted_tasks: 1,
            reoffered_tasks: 1,
            expired_resource_leases: 0,
            granted_from_queue: 0,
            expired_lanes: vec![(crew, lane)],
            task_changes: vec![
                (tasks[0], RelaySweptTask::Interrupted),
                (tasks[1], RelaySweptTask::Reoffered),
            ],
            completed_runs: vec![(run, crew)],
        };
        let events: Vec<serde_json::Value> =
            sweep_events(&report).iter().map(|e| serde_json::to_value(e).unwrap()).collect();
        assert_eq!(events.len(), 4);
        assert_eq!(
            events[3],
            serde_json::to_value(StateEvent::relay_run(RunChange::Completed, &crew)).unwrap()
        );
        assert_eq!(
            events[0],
            serde_json::to_value(StateEvent::relay_lane(
                LaneChange::Suspended(LaneReason::LeaseExpired),
                &lane,
                &crew
            ))
            .unwrap()
        );
        for (event, (task, op)) in
            events[1..3].iter().zip(tasks.iter().zip(["interrupt", "reoffer"]))
        {
            assert_eq!(event["entity"], "task");
            assert_eq!(event["data"]["op"], op);
            assert_eq!(event["data"]["task_id"], task.to_string());
        }
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
            task_changes: vec![],
            completed_runs: vec![],
        };
        assert!(sweep_events(&report).is_empty());
    }
}
