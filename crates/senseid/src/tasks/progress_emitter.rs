//! Scan-pipeline progress emitter — translates per-file TaskEvent::Completed
//! events into throttled StateEvent::folder_update SSE events.
//!
//! Listens to the task queue's broadcast channel. For each folder being
//! indexed, maintains an in-memory tracker that counts completed file tasks
//! and emits a StateEvent::folder_update at most every 300ms or after 25
//! files (whichever fires first). When the terminal BuildConnections task
//! for a folder completes, emits a final folder_update with status=Indexed.

use crate::api::events::{StateEvent, ScanFolder, FolderKind, FolderStatus, ActivityEvent, ActivityLevel};
use crate::tasks::progress::TaskEvent;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

const THROTTLE_DURATION: Duration = Duration::from_millis(300);
const THROTTLE_FILE_DELTA: u32 = 25;

#[derive(Clone)]
struct FolderTracker {
    folder_id:       String,
    project_id:      String,
    folder_name:     String,
    path:            String,
    kind:            FolderKind,
    stack:           Vec<String>,
    files_total:     u32,
    /// Successfully processed + failed file tasks combined. A failed task
    /// has still consumed its slot in the work list, so progress (and the
    /// UI bar) needs to count it toward completion or the bar permanently
    /// undershoots. The breakdown is kept on `files_failed` for future
    /// reporting; the wire `files_completed` is the union.
    files_completed: u32,
    files_failed:    u32,
    last_emit_at:    Instant,
    last_emit_count: u32,
    /// Set the first time a file result (completed or failed) arrives for this
    /// folder, so the queued→indexing transition emits exactly one Process
    /// activity. Without a Process-level event during the (longest) indexing
    /// phase the scan activity stream goes quiet and the `totalElapsed` timer
    /// the UI derives from it freezes (#33).
    announced_indexing: bool,
}

impl FolderTracker {
    fn should_emit(&self, now: Instant) -> bool {
        now.duration_since(self.last_emit_at) >= THROTTLE_DURATION
            || (self.files_completed.saturating_sub(self.last_emit_count)) >= THROTTLE_FILE_DELTA
    }

    /// Returns `true` exactly once — the first call marks the queued→indexing
    /// transition so a single Process activity is emitted when real work starts
    /// on this folder. Idempotent thereafter.
    fn take_indexing_announcement(&mut self) -> bool {
        if self.announced_indexing {
            false
        } else {
            self.announced_indexing = true;
            true
        }
    }
}

/// Per-folder Process activity messages. Kept as pure builders so the wire
/// strings are unit-testable and live in one place.
fn indexing_message(folder_name: &str, files_total: u32) -> String {
    format!("{folder_name} · indexing {files_total} files")
}

fn indexed_message(folder_name: &str, files_completed: u32) -> String {
    format!("{folder_name} · indexed {files_completed} files")
}

/// Elapsed (seconds) since the current scan's first folder was queued. The UI's
/// `totalElapsed` takes the max elapsed across activity events; emitting Process
/// events with this scan-relative clock keeps the displayed timer advancing
/// through the indexing phase instead of freezing at the discovery-phase value.
fn scan_elapsed(scan_start: Option<Instant>) -> f64 {
    scan_start.map(|s| s.elapsed().as_secs_f64()).unwrap_or(0.0)
}

fn scan_folder_from(t: &FolderTracker, status: FolderStatus) -> ScanFolder {
    ScanFolder {
        id: t.folder_id.clone(),
        project_id: t.project_id.clone(),
        name: t.folder_name.clone(),
        path: t.path.clone(),
        kind: t.kind.clone(),
        stack: t.stack.clone(),
        files_total: t.files_total,
        files_completed: t.files_completed,
        status,
    }
}

/// Public entry point — spawns the emitter as a tokio task that lives for the
/// daemon's lifetime. Subscribes to the task queue's TaskEvent stream and
/// publishes throttled StateEvent::folder_update events to the API broadcast
/// channel.
pub fn spawn(
    task_events: broadcast::Receiver<TaskEvent>,
    state_events: broadcast::Sender<StateEvent>,
    pg: Arc<crate::db::pg_store::PgStore>,
) {
    tokio::spawn(run(task_events, state_events, pg));
}

async fn run(
    mut task_events: broadcast::Receiver<TaskEvent>,
    state_events: broadcast::Sender<StateEvent>,
    pg: Arc<crate::db::pg_store::PgStore>,
) {
    let mut trackers: HashMap<String, FolderTracker> = HashMap::new();
    // Wall-clock start of the current scan batch — set when the first folder is
    // queued, reset when the last folder finishes. Drives the scan-relative
    // elapsed on Process activity events (#33).
    let mut scan_start: Option<Instant> = None;

    while let Ok(evt) = task_events.recv().await {
        match evt {
            TaskEvent::FolderQueued { folder_path, files_total } => {
                if let Some(t) = build_tracker(&pg, &folder_path, files_total).await {
                    if scan_start.is_none() {
                        scan_start = Some(Instant::now());
                    }
                    trackers.insert(folder_path, t);
                }
            }
            // Both Completed and Failed advance the progress denominator. A
            // file task that errored out (parse failure, IO error) still
            // consumed its slot in the work list; without counting it the
            // bar can never reach 100% on a repo where any file fails.
            TaskEvent::Completed { kind, folder_path, .. } if kind == "process_file" => {
                if let Some(t) = trackers.get_mut(&folder_path) {
                    t.files_completed += 1;
                    if t.take_indexing_announcement() {
                        let _ = state_events.send(StateEvent::activity(ActivityEvent::new(
                            ActivityLevel::Process,
                            &indexing_message(&t.folder_name, t.files_total),
                            scan_elapsed(scan_start),
                        )));
                    }
                    let now = Instant::now();
                    if t.should_emit(now) {
                        t.last_emit_at = now;
                        t.last_emit_count = t.files_completed;
                        let _ = state_events.send(StateEvent::folder_update(
                            scan_folder_from(t, FolderStatus::Indexing),
                        ));
                    }
                }
            }
            TaskEvent::Failed { kind, folder_path, .. } if kind == "process_file" => {
                if let Some(t) = trackers.get_mut(&folder_path) {
                    t.files_completed += 1;
                    t.files_failed += 1;
                    if t.take_indexing_announcement() {
                        let _ = state_events.send(StateEvent::activity(ActivityEvent::new(
                            ActivityLevel::Process,
                            &indexing_message(&t.folder_name, t.files_total),
                            scan_elapsed(scan_start),
                        )));
                    }
                    let now = Instant::now();
                    if t.should_emit(now) {
                        t.last_emit_at = now;
                        t.last_emit_count = t.files_completed;
                        let _ = state_events.send(StateEvent::folder_update(
                            scan_folder_from(t, FolderStatus::Indexing),
                        ));
                    }
                }
            }
            TaskEvent::Completed { kind, folder_path, .. } if kind == "build_connections" => {
                if let Some(t) = trackers.remove(&folder_path) {
                    let project_id_str = t.project_id.clone();
                    let _ = state_events.send(StateEvent::folder_update(
                        scan_folder_from(&t, FolderStatus::Indexed),
                    ));
                    let _ = state_events.send(StateEvent::activity(ActivityEvent::new(
                        ActivityLevel::Process,
                        &indexed_message(&t.folder_name, t.files_completed),
                        scan_elapsed(scan_start),
                    )));

                    // Was this the last folder for the project? If so, flip the project to active.
                    let all_indexed = match uuid::Uuid::parse_str(&project_id_str) {
                        Ok(uid) => match pg.count_unindexed_folders(uid).await {
                            Ok(remaining) => Some(remaining == 0),
                            Err(e) => {
                                tracing::warn!(error = %e, project_id = %project_id_str, "count_unindexed_folders failed; skipping project-active check");
                                None
                            }
                        },
                        Err(e) => {
                            tracing::warn!(error = %e, project_id = %project_id_str, "invalid project_id; skipping project-active check");
                            None
                        }
                    };
                    if let Some(check) = all_indexed
                        && check {
                        let _ = state_events.send(StateEvent::project_update(
                            crate::api::events::ScanProject {
                                id: project_id_str.clone(),
                                name: String::new(),
                                status: crate::api::events::ProjectStatus::Active,
                                folders: vec![],
                                auto_detected: true,
                                confidence: crate::api::events::Confidence::High,
                            },
                        ));
                    }
                }
                // Last folder of the batch finished — reset the scan clock so the
                // next scan's Process events start from zero again.
                if trackers.is_empty() {
                    scan_start = None;
                }
            }
            _ => {}
        }
    }
}

fn kind_from_str(s: &str) -> FolderKind {
    match s {
        "workspace_member" => FolderKind::WorkspaceMember,
        "subtree"          => FolderKind::Subtree,
        "sibling"          => FolderKind::Sibling,
        "standalone"       => FolderKind::Standalone,
        _                  => FolderKind::Git,
    }
}

async fn build_tracker(
    pg: &crate::db::pg_store::PgStore,
    folder_path: &str,
    files_total: u32,
) -> Option<FolderTracker> {
    let row = match pg.get_repo_by_path(folder_path).await {
        Ok(row) => row,
        Err(e) => {
            tracing::warn!(error = %e, folder_path, "get_repo_by_path failed; cannot build progress tracker");
            return None;
        }
    }?;
    let folder_id   = row.get("id")?.as_str()?.to_string();
    // project_id may be null (folder not yet assigned to a project)
    let project_id  = row.get("project_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let folder_name = row.get("name")?.as_str()?.to_string();
    let kind        = row.get("kind")
        .and_then(|v| v.as_str())
        .map(kind_from_str)
        .unwrap_or(FolderKind::Git);
    Some(FolderTracker {
        folder_id,
        project_id,
        folder_name,
        path: folder_path.to_string(),
        kind,
        stack: vec![],
        files_total,
        files_completed: 0,
        files_failed: 0,
        last_emit_at: Instant::now(),
        last_emit_count: 0,
        announced_indexing: false,
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_tracker(files_total: u32, now: Instant) -> FolderTracker {
        FolderTracker {
            folder_id: "f".into(),
            project_id: "p".into(),
            folder_name: "n".into(),
            path: "/p".into(),
            kind: FolderKind::Git,
            stack: vec![],
            files_total,
            files_completed: 0,
            files_failed: 0,
            last_emit_at: now,
            last_emit_count: 0,
            announced_indexing: false,
        }
    }

    /// Drives the throttle by directly mutating a FolderTracker — mirrors what
    /// run() does in the broadcast loop, but without the channel infrastructure.
    fn tick(t: &mut FolderTracker, now: Instant) -> bool {
        t.files_completed += 1;
        if t.should_emit(now) {
            t.last_emit_at = now;
            t.last_emit_count = t.files_completed;
            true
        } else {
            false
        }
    }

    #[test]
    fn emits_after_25_files_within_throttle_window() {
        let t0 = Instant::now();
        let mut t = fresh_tracker(100, t0);
        // First 24 files at the same instant: under the duration threshold AND
        // under the 25-file delta — no emit.
        for _ in 0..24 { assert!(!tick(&mut t, t0)); }
        // 25th file: delta reaches 25 — should emit.
        assert!(tick(&mut t, t0));
    }

    #[test]
    fn emits_after_300ms_elapsed_even_with_few_files() {
        let t0 = Instant::now();
        let mut t = fresh_tracker(100, t0);
        assert!(!tick(&mut t, t0));   // 1 file, 0ms elapsed — no emit
        let t1 = t0 + Duration::from_millis(300);
        assert!(tick(&mut t, t1));    // 2 files, 300ms — emit on time
    }

    #[test]
    fn resets_throttle_window_after_emit() {
        let t0 = Instant::now();
        let mut t = fresh_tracker(100, t0);
        for _ in 0..25 { tick(&mut t, t0); }     // emits at file 25
        // files_completed = 25, last_emit_count = 25.
        for _ in 0..24 { assert!(!tick(&mut t, t0)); }  // 49 files total, delta of 24 — no emit
        assert!(tick(&mut t, t0));                       // 50 files, delta of 25 — emit
    }

    #[test]
    fn indexing_and_indexed_messages_format() {
        assert_eq!(indexing_message("app", 842), "app · indexing 842 files");
        assert_eq!(indexed_message("app", 840), "app · indexed 840 files");
    }

    #[test]
    fn announces_indexing_exactly_once() {
        let mut t = fresh_tracker(10, Instant::now());
        assert!(t.take_indexing_announcement(), "first file announces indexing");
        assert!(!t.take_indexing_announcement(), "subsequent files do not re-announce");
        assert!(!t.take_indexing_announcement());
    }

    #[test]
    fn process_activity_serializes_with_process_level() {
        // Pin the wire contract: the front-end ActivityLevel union includes
        // 'process'; a mismatch here would silently drop the event client-side.
        let evt = ActivityEvent::new(ActivityLevel::Process, &indexing_message("app", 5), 1.5);
        let json = serde_json::to_value(&evt).unwrap();
        assert_eq!(json["level"], "process");
        assert_eq!(json["message"], "app · indexing 5 files");
    }

    #[test]
    fn scan_elapsed_is_zero_when_unset() {
        assert_eq!(scan_elapsed(None), 0.0);
    }
}
