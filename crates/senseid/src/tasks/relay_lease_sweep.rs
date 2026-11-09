//! Relay lane-lease sweeper (slice 1, item 2): every pass expires lapsed
//! lane leases, suspends their lanes' work, re-offers tasks past the grace,
//! and re-picks dead reviewers (Q17). The `relay_lease` schedule owns the
//! cadence (60 s seeded: the 300 s TTL over five); a missing relay schema
//! (pre-convergence databases) warns, never fatals.

use std::sync::Arc;

use crate::db::pg_store::PgStore;

/// One pass at wall-clock now.
pub async fn tick(pg: Arc<PgStore>) -> Result<(), String> {
    match pg.relay_sweep_leases(chrono::Utc::now()).await {
        Ok(rep) => {
            if rep.expired_leases
                + rep.interrupted_tasks
                + rep.reoffered_tasks
                + rep.repicked_reviewers
                > 0
            {
                tracing::info!(
                    expired = rep.expired_leases,
                    interrupted = rep.interrupted_tasks,
                    reoffered = rep.reoffered_tasks,
                    repicked = rep.repicked_reviewers,
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

fn is_missing_schema(detail: &str) -> bool {
    detail.contains("does not exist") || detail.contains("undefined_table")
}

pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(async move {
        crate::tasks::ticker::run_scheduled(pg.clone(), "relay_lease", move || {
            let pg = pg.clone();
            async move { tick(pg).await }
        })
        .await;
    });
}
