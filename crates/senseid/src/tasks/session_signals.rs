//! The session-signal derivation pass (spec tracker-sdlc-metrics S2): derive the
//! SDLC signals of sessions whose events or catalogue changed. One session at a
//! time, each in its own transaction; a session that fails is recorded and
//! skipped, never blocking the ones after it.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::session_signals::SignalMark;
use crate::sdlc::framework::{StaticOutcome, behavioural, decide, repo_markers};
use crate::sdlc::signals::{DERIVE_VERSION, derive_signals};
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

/// Whether a path is a directory: the marker probe, injected so tests can fail or
/// stall it.
pub type Probe = std::sync::Arc<dyn Fn(&std::path::Path) -> std::io::Result<bool> + Send + Sync>;

/// How long one repository's marker probe may take before it counts as failed (B10).
pub const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// The filesystem probe.
pub fn fs_probe() -> Probe {
    std::sync::Arc::new(|p: &std::path::Path| std::fs::metadata(p).map(|m| m.is_dir()))
}

/// One pass (spec S2 steps 1–3) with the filesystem marker probe. `Err` only when
/// the catalogue can't be read or compiled, or the candidates can't be selected:
/// then no session is touched.
///
/// A deterministic derivation error is recorded against the inputs it failed on,
/// so the session is skipped until they change. A read or write error may be
/// transient, so it is recorded against inputs no session can have (-1) and the
/// session is retried next pass.
pub async fn pass(pg: &PgStore, now_ms: i64, limit: i64) -> Result<PassReport, String> {
    pass_with(pg, now_ms, limit, fs_probe(), PROBE_TIMEOUT).await
}

/// [`pass`] with the marker probe and its timeout given. Each session also gets its
/// SDD framework (blueprint 2026-11-10-ticket-sdd-framework): a repository's markers
/// are probed at most once per pass, off the runtime and under `probe_timeout`.
pub async fn pass_with(
    pg: &PgStore,
    now_ms: i64,
    limit: i64,
    probe: Probe,
    probe_timeout: std::time::Duration,
) -> Result<PassReport, String> {
    let catalogue = pg.command_catalogue().await?;
    let candidates = pg.sessions_needing_signals(&catalogue.hash, limit).await?;
    let mut report = PassReport::default();
    let mut probed: std::collections::HashMap<String, StaticOutcome> = Default::default();
    for c in candidates {
        let mark_of = |through_event_id, event_count| SignalMark {
            session_id: c.session_id,
            through_event_id,
            event_count,
            catalogue_hash: catalogue.hash.clone(),
            derive_version: DERIVE_VERSION,
            error: None,
        };
        let mut mark = mark_of(-1, -1);
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
                let read_mark =
                    mark_of(events.iter().map(|e| e.id).max().unwrap_or(0), events.len() as i32);
                match derive_signals(&events, &catalogue, &folder, now_ms) {
                    Err(e) => {
                        mark = read_mark;
                        Err(e)
                    }
                    Ok(signals) => {
                        let repo = c.repo_path.as_deref();
                        let hit = behavioural(&events, repo);
                        let markers = match (hit, repo) {
                            (None, Some(path)) => {
                                if !probed.contains_key(path) {
                                    let found =
                                        probe_markers(path, Arc::clone(&probe), probe_timeout)
                                            .await;
                                    probed.insert(path.to_string(), found);
                                }
                                &probed[path]
                            }
                            _ => &StaticOutcome::NotProbed,
                        };
                        let detection = decide(hit, markers, c.previous);
                        pg.store_session_signals(&read_mark, &signals, detection).await
                    }
                }
            }
        };
        match outcome {
            Ok(true) => {
                report.derived += 1;
                // B15: after the session's write, on its own; a sync heals a miss.
                if let Err(e) = pg.pin_session_tickets(c.session_id).await {
                    tracing::warn!(session_id = %c.session_id, error = %e, "derive_session_signals: ticket pin failed; the next tracker sync retries it");
                }
            }
            Ok(false) => {
                tracing::debug!(session_id = %c.session_id, "derive_session_signals: session removed mid-pass");
            }
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

/// A repository's markers, probed on a detached thread under `timeout`. An error or
/// a timeout is [`StaticOutcome::Failed`] (B10); the reason is logged. The thread is
/// not the runtime's, so a probe stuck on a dead mount cannot hold shutdown open.
async fn probe_markers(repo: &str, probe: Probe, timeout: std::time::Duration) -> StaticOutcome {
    let path = std::path::PathBuf::from(repo);
    let (tx, rx) = tokio::sync::oneshot::channel();
    let spawned = std::thread::Builder::new().name("sdd-marker-probe".into()).spawn(move || {
        let _ = tx.send(repo_markers(&path, |p| probe(p)));
    });
    let why = match spawned {
        Err(e) => format!("could not start the probe thread: {e}"),
        Ok(_) => match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(found))) => return StaticOutcome::Found(found),
            Ok(Ok(Err(e))) => e,
            Ok(Err(_)) => "the probe thread ended without a result".to_string(),
            Err(_) => format!("timed out after {timeout:?}"),
        },
    };
    tracing::warn!(repo, error = %why, "derive_session_signals: SDD marker probe failed");
    StaticOutcome::Failed
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

    /// A probe stuck on a dead mount must not hold the runtime open at shutdown:
    /// dropping a runtime waits for its blocking tasks, so a SIGTERM would then need
    /// a SIGKILL.
    #[test]
    fn a_hung_probe_does_not_hold_the_runtime_open() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
            let hung: Probe = Arc::new(|_: &std::path::Path| -> std::io::Result<bool> {
                loop {
                    std::thread::park();
                }
            });
            let out =
                rt.block_on(probe_markers("/_fw/hung", hung, std::time::Duration::from_millis(50)));
            drop(rt);
            tx.send(matches!(out, StaticOutcome::Failed)).unwrap();
        });
        assert_eq!(rx.recv_timeout(std::time::Duration::from_secs(5)), Ok(true));
    }
}
