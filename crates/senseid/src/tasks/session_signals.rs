//! The session-signal derivation pass (spec tracker-sdlc-metrics S2): derive the
//! SDLC signals of sessions whose events or catalogue changed. One session at a
//! time, each in its own transaction; a session that fails is recorded and
//! skipped, never blocking the ones after it.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::session_signals::SignalMark;
use crate::sdlc::signals::derive_signals;
use crate::tasks::ticker;

/// Sessions taken per pass.
pub const PASS_LIMIT: i64 = 500;

/// What one pass did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PassReport {
    pub derived: usize,
    pub failed: usize,
}

/// Spawn the pass for the daemon's lifetime. Cadence lives in `sensei.schedules`
/// (name `derive_session_signals`).
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(async move {
        let store = pg.clone();
        ticker::run_scheduled(pg, "derive_session_signals", move || {
            let pg = store.clone();
            async move {
                let report = pass(&pg, chrono::Utc::now().timestamp_millis(), PASS_LIMIT).await?;
                if report.derived > 0 {
                    tracing::info!(
                        derived = report.derived,
                        failed = report.failed,
                        "derive_session_signals: derived {} session(s), {} failed",
                        report.derived,
                        report.failed,
                    );
                }
                outcome(&report)
            }
        })
        .await;
    });
}

/// How a pass reports to the schedule: a run in which a session failed is a failed
/// run, so `sensei.schedules` never shows it green.
pub fn outcome(report: &PassReport) -> Result<(), String> {
    match report.failed {
        0 => Ok(()),
        n => Err(format!("{n} of {} session(s) failed", report.derived + n)),
    }
}

/// One pass (spec S2 steps 1–3). `Err` only when the catalogue can't be read or
/// compiled, or the candidates can't be selected: then no session is touched.
///
/// A deterministic derivation error is recorded against the inputs it failed on,
/// so the session is skipped until they change. A read or write error may be
/// transient, so it is recorded against inputs no session can have (-1) and the
/// session is retried next pass.
pub async fn pass(pg: &PgStore, now_ms: i64, limit: i64) -> Result<PassReport, String> {
    let catalogue = pg.command_catalogue().await?;
    let candidates = pg.sessions_needing_signals(&catalogue.hash, limit).await?;
    let mut report = PassReport::default();
    for c in candidates {
        let mut mark = SignalMark {
            session_id: c.session_id,
            through_event_id: -1,
            event_count: -1,
            catalogue_hash: catalogue.hash.clone(),
            error: None,
        };
        let read = async {
            let events = pg.session_events_for_signals(&c.client_session_id).await?;
            let folder = match c.repo_folder_id {
                Some(f) => pg.folder_commands_under(f).await?,
                None => Vec::new(),
            };
            Ok::<_, String>((events, folder))
        };
        let outcome = match read.await {
            Err(e) => Err(e),
            Ok((events, folder)) => {
                let read_mark = SignalMark {
                    session_id: c.session_id,
                    through_event_id: events.iter().map(|e| e.id).max().unwrap_or(0),
                    event_count: events.len() as i32,
                    catalogue_hash: catalogue.hash.clone(),
                    error: None,
                };
                match derive_signals(&events, &catalogue, &folder, now_ms) {
                    Err(e) => {
                        mark = read_mark;
                        Err(e)
                    }
                    Ok(signals) => pg.store_session_signals(&read_mark, &signals).await,
                }
            }
        };
        match outcome {
            Ok(()) => report.derived += 1,
            Err(e) => {
                report.failed += 1;
                tracing::warn!(session_id = %c.session_id, error = %e, "derive_session_signals: session failed");
                mark.error = Some(e);
                if let Err(store) = pg.store_session_signal_failure(&mark).await {
                    tracing::warn!(session_id = %c.session_id, error = %store, "derive_session_signals: could not record the failure");
                }
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pass_with_failures_reports_a_failed_run() {
        assert_eq!(outcome(&PassReport { derived: 3, failed: 0 }), Ok(()));
        let err = outcome(&PassReport { derived: 3, failed: 2 }).unwrap_err();
        assert!(err.contains("2 of 5"), "{err}");
    }
}
