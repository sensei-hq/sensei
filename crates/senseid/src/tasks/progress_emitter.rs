//! Scan-pipeline progress emitter — translates per-file TaskEvent::Completed
//! events into throttled StateEvent::folder_update SSE events.
//!
//! Listens to the task queue's broadcast channel. For each folder being
//! indexed, maintains an in-memory tracker that counts completed file tasks
//! and emits a StateEvent::folder_update at most every 300ms or after 25
//! files (whichever fires first). When the terminal BuildConnections task
//! for a folder completes, emits a final folder_update with status=Indexed.

use crate::api::events::{StateEvent, ScanFolder, FolderKind, FolderStatus};
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
}

impl FolderTracker {
    fn should_emit(&self, now: Instant) -> bool {
        now.duration_since(self.last_emit_at) >= THROTTLE_DURATION
            || (self.files_completed.saturating_sub(self.last_emit_count)) >= THROTTLE_FILE_DELTA
    }
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

    while let Ok(evt) = task_events.recv().await {
        match evt {
            TaskEvent::FolderQueued { folder_path, files_total } => {
                if let Some(t) = build_tracker(&pg, &folder_path, files_total).await {
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

                    // Was this the last folder for the project? If so, flip the project to active.
                    let all_indexed = uuid::Uuid::parse_str(&project_id_str)
                        .ok()
                        .map(|uid| pg.count_unindexed_folders(uid))
                        .map(|fut| async { fut.await == Ok(0) });
                    if let Some(check) = all_indexed
                        && check.await {
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
    let row = pg.get_repo_by_path(folder_path).await.ok().flatten()?;
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
}
