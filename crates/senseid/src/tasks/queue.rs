//! Task queue with dependency tracking and barrier support.
//!
//! Tasks are enqueued with optional depends_on IDs. Blocked tasks
//! automatically become Pending when all dependencies complete.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use serde::Serialize;
use tokio::sync::{Mutex, Notify, broadcast};
use super::{Task, TaskStatus};
use super::progress::TaskEvent;

const DEFAULT_MAX_CONCURRENT_REPOS: usize = 3;

pub struct TaskQueue {
    inner: Mutex<QueueState>,
    notify: Notify,
    next_id: AtomicU64,
    tx: broadcast::Sender<TaskEvent>,
    max_concurrent_repos: std::sync::atomic::AtomicUsize,
}

struct QueueState {
    pending: VecDeque<Task>,
    blocked: Vec<Task>,
    running: HashMap<u64, Task>,
    completed: Vec<Task>,   // last N for history
    /// Track which repos have running tasks (for concurrency limit)
    folder_running_count: HashMap<String, usize>,
    /// Map of task_id → list of task_ids that depend on it
    dependents: HashMap<u64, Vec<u64>>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self::with_max_repos(DEFAULT_MAX_CONCURRENT_REPOS)
    }

    pub fn with_max_repos(max_repos: usize) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            inner: Mutex::new(QueueState {
                pending: VecDeque::new(),
                blocked: Vec::new(),
                running: HashMap::new(),
                completed: Vec::new(),
                folder_running_count: HashMap::new(),
                dependents: HashMap::new(),
            }),
            notify: Notify::new(),
            next_id: AtomicU64::new(1),
            tx,
            max_concurrent_repos: std::sync::atomic::AtomicUsize::new(max_repos),
        }
    }

    pub fn set_max_concurrent_repos(&self, n: usize) {
        self.max_concurrent_repos.store(n, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn max_concurrent_repos(&self) -> usize {
        self.max_concurrent_repos.load(Ordering::SeqCst)
    }

    pub fn sender(&self) -> &broadcast::Sender<TaskEvent> {
        &self.tx
    }

    /// Enqueue a task. Returns the assigned task ID.
    pub async fn enqueue(&self, task: Task) -> u64 {
        let mut state = self.inner.lock().await;
        let id = self.enqueue_locked(&mut state, task);
        drop(state);
        self.notify.notify_one();
        id
    }

    /// Like [`enqueue`], but a no-op returning `None` when a task with the same
    /// `(kind, folder_path, path)` is already pending, blocked, or running — the
    /// single-writer guard (D6e / W5). The dedup check and the insert happen
    /// under **one** lock acquisition, so two concurrent callers can't both slip
    /// a duplicate past the guard (the check-then-enqueue race). Enqueue sites
    /// that must never double-scan a folder/file (ScanRoot / ProcessGitFolder /
    /// ProcessFile) use this instead of [`enqueue`].
    pub async fn enqueue_unique(&self, task: Task) -> Option<u64> {
        let mut state = self.inner.lock().await;
        let matches = |t: &Task| {
            t.kind == task.kind && t.folder_path == task.folder_path && t.path == task.path
        };
        let dup = state.pending.iter().any(matches)
            || state.blocked.iter().any(matches)
            || state.running.values().any(matches);
        if dup {
            return None; // guard drops the lock on return
        }
        let id = self.enqueue_locked(&mut state, task);
        drop(state);
        self.notify.notify_one();
        Some(id)
    }

    /// Shared enqueue body — assign an id, register dependency tracking, place
    /// the task on `blocked` or `pending`, and broadcast `Queued`. Runs with the
    /// queue lock already held so callers ([`enqueue`], [`enqueue_unique`]) can
    /// combine it with a pre-check atomically. The caller notifies waiters after
    /// dropping the lock.
    fn enqueue_locked(&self, state: &mut QueueState, mut task: Task) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        task.id = id;

        // Register dependency tracking
        for dep_id in &task.depends_on {
            state.dependents.entry(*dep_id).or_default().push(id);
        }

        if task.status == TaskStatus::Blocked {
            // Check if deps are already completed
            let unmet: Vec<u64> = task.depends_on.iter()
                .filter(|dep| !state.completed.iter().any(|c| c.id == **dep))
                .copied()
                .collect();
            if unmet.is_empty() {
                task.status = TaskStatus::Pending;
                task.depends_on.clear();
                state.pending.push_back(task);
            } else {
                task.depends_on = unmet.clone();
                // Structured log so a stalled pipeline is diagnosable from the
                // logs — records which task blocked on which and (best-effort)
                // the kinds of the blocking tasks so the reader doesn't have
                // to cross-reference ids by hand (#64). Resolved by scanning
                // blocked / running / pending / completed for each dep id;
                // unresolved ids are reported as `"?"` which is a signal that
                // the dep was already dropped from history.
                let blockers = blocker_summary(state, &unmet);
                tracing::debug!(
                    task_id = id,
                    kind = %task.kind,
                    folder_path = %task.folder_path,
                    path = %task.path,
                    blocked_on = ?blockers,
                    "task blocked on {} dependency(ies)", unmet.len(),
                );
                state.blocked.push(task);
            }
        } else {
            state.pending.push_back(task);
        }

        let _ = self.tx.send(TaskEvent::Queued { task_id: id });
        id
    }

    /// Enqueue multiple tasks at once. Returns their IDs.
    #[allow(dead_code)]
    pub async fn enqueue_batch(&self, tasks: Vec<Task>) -> Vec<u64> {
        let mut ids = Vec::with_capacity(tasks.len());
        for task in tasks {
            ids.push(self.enqueue(task).await);
        }
        ids
    }

    /// Add a dependency to a blocked task after creation.
    /// Used when file tasks are created by folder tasks and need to be
    /// added to the resolve_edges barrier.
    #[allow(dead_code)]
    pub async fn add_dependency(&self, barrier_task_id: u64, new_dep_id: u64) {
        let mut state = self.inner.lock().await;
        // Add to blocked task's depends_on
        for task in &mut state.blocked {
            if task.id == barrier_task_id {
                task.depends_on.push(new_dep_id);
                break;
            }
        }
        // Register reverse mapping
        state.dependents.entry(new_dep_id).or_default().push(barrier_task_id);
    }

    /// Get next runnable task. Blocks until one is available.
    /// Respects MAX_CONCURRENT_REPOS limit.
    pub async fn next_task(&self) -> Task {
        loop {
            {
                let mut state = self.inner.lock().await;
                // Find a pending task whose repo isn't at the concurrency limit
                let max_repos = self.max_concurrent_repos.load(Ordering::SeqCst);
                let pos = state.pending.iter().position(|t| {
                    let count = state.folder_running_count.get(&t.folder_path).copied().unwrap_or(0);
                    count < max_repos
                });

                if let Some(idx) = pos {
                    let mut task = state.pending.remove(idx).unwrap();
                    task.status = TaskStatus::Running;
                    task.started_at = Some(std::time::Instant::now());

                    *state.folder_running_count.entry(task.folder_path.clone()).or_insert(0) += 1;
                    let task_clone = task.clone();
                    state.running.insert(task.id, task);

                    let _ = self.tx.send(TaskEvent::Started {
                        task_id: task_clone.id,
                        folder_path: task_clone.folder_path.clone(),
                        kind: task_clone.kind.to_string(),
                        path: task_clone.path.clone(),
                    });
                    return task_clone;
                }
            }
            self.notify.notified().await;
        }
    }

    /// Mark a task as completed. Unblocks dependents.
    pub async fn complete(&self, task_id: u64) {
        let mut state = self.inner.lock().await;

        if let Some(mut task) = state.running.remove(&task_id) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(std::time::Instant::now());

            // Decrement repo running count
            if let Some(count) = state.folder_running_count.get_mut(&task.folder_path) {
                *count = count.saturating_sub(1);
                if *count == 0 { state.folder_running_count.remove(&task.folder_path); }
            }

            let folder_path = task.folder_path.clone();
            let kind = task.kind.to_string();

            // Keep last 100 completed
            state.completed.push(task);
            if state.completed.len() > 100 {
                state.completed.remove(0);
            }

            // Unblock dependents
            let dependent_ids = state.dependents.remove(&task_id).unwrap_or_default();
            let mut newly_pending = Vec::new();
            for dep_id in dependent_ids {
                if let Some(pos) = state.blocked.iter().position(|t| t.id == dep_id) {
                    state.blocked[pos].depends_on.retain(|d| *d != task_id);
                    if state.blocked[pos].depends_on.is_empty() {
                        let mut unblocked = state.blocked.remove(pos);
                        unblocked.status = TaskStatus::Pending;
                        // Pair with the enqueue-blocked log so a stall that
                        // eventually resolves shows both edges (#64).
                        tracing::debug!(
                            task_id = unblocked.id,
                            kind = %unblocked.kind,
                            folder_path = %unblocked.folder_path,
                            released_by = task_id,
                            "task unblocked",
                        );
                        newly_pending.push(unblocked);
                    }
                }
            }
            for t in newly_pending {
                state.pending.push_back(t);
            }

            let _ = self.tx.send(TaskEvent::Completed {
                task_id,
                folder_path,
                kind,
            });
        }

        drop(state);
        self.notify.notify_waiters();
    }

    /// Mark a task as failed.
    pub async fn fail(&self, task_id: u64, error: String) {
        let mut state = self.inner.lock().await;

        if let Some(mut task) = state.running.remove(&task_id) {
            task.status = TaskStatus::Failed;
            task.error = Some(error.clone());
            task.completed_at = Some(std::time::Instant::now());

            if let Some(count) = state.folder_running_count.get_mut(&task.folder_path) {
                *count = count.saturating_sub(1);
                if *count == 0 { state.folder_running_count.remove(&task.folder_path); }
            }

            let folder_path = task.folder_path.clone();
            let kind = task.kind.to_string();
            state.completed.push(task);

            // Still unblock dependents (they'll see partial data but won't deadlock)
            let dependent_ids = state.dependents.remove(&task_id).unwrap_or_default();
            for dep_id in dependent_ids {
                if let Some(pos) = state.blocked.iter().position(|t| t.id == dep_id) {
                    state.blocked[pos].depends_on.retain(|d| *d != task_id);
                    if state.blocked[pos].depends_on.is_empty() {
                        let mut unblocked = state.blocked.remove(pos);
                        unblocked.status = TaskStatus::Pending;
                        // A dep FAILED but we release the dependent anyway —
                        // note the reason so a partial-data downstream failure
                        // is traceable back to the failed upstream (#64).
                        tracing::debug!(
                            task_id = unblocked.id,
                            kind = %unblocked.kind,
                            folder_path = %unblocked.folder_path,
                            released_by = task_id,
                            released_by_state = "failed",
                            "task unblocked (dep failed — proceeding with partial data)",
                        );
                        state.pending.push_back(unblocked);
                    }
                }
            }

            let _ = self.tx.send(TaskEvent::Failed {
                task_id,
                folder_path,
                kind,
                error,
            });
        }

        drop(state);
        self.notify.notify_waiters();
    }

    /// Get progress counts per repo.
    pub async fn progress(&self) -> HashMap<String, super::progress::RepoProgress> {
        let state = self.inner.lock().await;
        let mut result: HashMap<String, super::progress::RepoProgress> = HashMap::new();

        for t in &state.pending {
            let p = result.entry(t.folder_path.clone()).or_default();
            p.total += 1;
            p.pending += 1;
        }
        for t in &state.blocked {
            let p = result.entry(t.folder_path.clone()).or_default();
            p.total += 1;
            p.pending += 1; // blocked counts as pending from user perspective
        }
        for t in state.running.values() {
            let p = result.entry(t.folder_path.clone()).or_default();
            p.total += 1;
            p.running += 1;
            p.current_file = Some(t.path.clone());
        }
        // Don't include completed in totals (they're done)

        result
    }

    /// True if a task of `kind` is currently pending, blocked, or running.
    /// Used by the reconcile scheduler as an overlap guard so it never stacks a
    /// second `ScanRoot` batch while a scan/reconcile is still in flight.
    pub async fn has_pending_kind(&self, kind: super::TaskKind) -> bool {
        let state = self.inner.lock().await;
        state.pending.iter().any(|t| t.kind == kind)
            || state.blocked.iter().any(|t| t.kind == kind)
            || state.running.values().any(|t| t.kind == kind)
    }

    /// True if a task of `kind` AND `path` is currently pending, blocked, or
    /// running. The per-(kind, path) variant of [`has_pending_kind`] — used by
    /// the advance-run scheduler to de-dup `AdvanceRun` ticks for the same run
    /// (run id rides in `task.path`) so a backed-up queue never piles up
    /// duplicate ticks for one run in the unbounded `VecDeque`.
    pub async fn has_pending_kind_path(&self, kind: super::TaskKind, path: &str) -> bool {
        let state = self.inner.lock().await;
        state.pending.iter().any(|t| t.kind == kind && t.path == path)
            || state.blocked.iter().any(|t| t.kind == kind && t.path == path)
            || state.running.values().any(|t| t.kind == kind && t.path == path)
    }

    /// True if a task of `kind` AND `folder_path` is pending, blocked, or running.
    /// The per-(kind, folder_path) variant — the library-update scheduler guards
    /// `IndexLibrary` on the library id (which rides in `task.folder_path`), NOT the
    /// name in `task.path` (names aren't unique across ecosystems).
    pub async fn has_pending_kind_folder(&self, kind: super::TaskKind, folder_path: &str) -> bool {
        let state = self.inner.lock().await;
        state.pending.iter().any(|t| t.kind == kind && t.folder_path == folder_path)
            || state.blocked.iter().any(|t| t.kind == kind && t.folder_path == folder_path)
            || state.running.values().any(|t| t.kind == kind && t.folder_path == folder_path)
    }

    /// Test-only: a snapshot of every task the queue has seen (pending, blocked,
    /// running, completed) as `(kind, folder_path, path)`. Lets a test assert
    /// the enqueue GRAPH — which kinds were enqueued for which owner
    /// (`folder_path`) — so the #101 one-task-one-owner invariant is locked at
    /// the enqueue layer, not just the final DB. A member must never appear as
    /// the `folder_path` (owner) of a `ProcessFile`, nor get its own
    /// `ProcessGitFolder`.
    #[cfg(test)]
    pub async fn snapshot(&self) -> Vec<(super::TaskKind, String, String)> {
        let s = self.inner.lock().await;
        s.pending.iter()
            .chain(s.blocked.iter())
            .chain(s.running.values())
            .chain(s.completed.iter())
            .map(|t| (t.kind.clone(), t.folder_path.clone(), t.path.clone()))
            .collect()
    }

    /// Get queue status summary.
    pub async fn status(&self) -> QueueStatus {
        let state = self.inner.lock().await;
        QueueStatus {
            pending: state.pending.len(),
            blocked: state.blocked.len(),
            running: state.running.len(),
            completed: state.completed.len(),
            repos_active: state.folder_running_count.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueStatus {
    pub pending: usize,
    pub blocked: usize,
    pub running: usize,
    pub completed: usize,
    pub repos_active: usize,
}

/// Best-effort resolver: for each unmet-dependency id, produce a
/// `"<id>:<kind>"` string by scanning the running / pending / blocked /
/// completed vecs. Unresolved ids come back as `"<id>:?"` so a reader can
/// tell "the dep was dropped from history" from "the dep still exists but
/// with an unknown kind" (the former should be rare — it means the task
/// completed and was already trimmed from the last-100 window). Kept as a
/// free function so the enqueue path can call it with a `&QueueState`
/// borrow that outlives the log macro.
fn blocker_summary(state: &QueueState, deps: &[u64]) -> Vec<String> {
    deps.iter()
        .map(|dep_id| {
            let kind = state.running.values().find(|t| t.id == *dep_id).map(|t| t.kind.to_string())
                .or_else(|| state.pending.iter().find(|t| t.id == *dep_id).map(|t| t.kind.to_string()))
                .or_else(|| state.blocked.iter().find(|t| t.id == *dep_id).map(|t| t.kind.to_string()))
                .or_else(|| state.completed.iter().find(|t| t.id == *dep_id).map(|t| t.kind.to_string()))
                .unwrap_or_else(|| "?".to_string());
            format!("{dep_id}:{kind}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{Task, TaskKind, TaskStatus};

    fn state_with(pending: Vec<Task>, blocked: Vec<Task>, completed: Vec<Task>) -> QueueState {
        let mut s = QueueState {
            pending: VecDeque::new(),
            blocked: Vec::new(),
            running: HashMap::new(),
            completed: Vec::new(),
            folder_running_count: HashMap::new(),
            dependents: HashMap::new(),
        };
        s.pending.extend(pending);
        s.blocked = blocked;
        s.completed = completed;
        s
    }

    fn task_with(id: u64, kind: TaskKind) -> Task {
        let mut t = Task::new(kind, "repo", "");
        t.id = id;
        t
    }

    #[test]
    fn blocker_summary_resolves_kinds_across_queues() {
        // #64: the blocked-task log has to identify blockers by kind, not just
        // by opaque id. Verify the four fallback paths (pending → blocked →
        // completed → unknown) all resolve as expected.
        let state = state_with(
            vec![task_with(11, TaskKind::ProcessFile)],
            vec![task_with(22, TaskKind::ResolveEdges)],
            vec![task_with(33, TaskKind::ProcessGitFolder)],
        );
        let summary = blocker_summary(&state, &[11, 22, 33, 99]);
        assert_eq!(
            summary,
            vec![
                "11:process_file".to_string(),
                "22:resolve_edges".to_string(),
                "33:process_git_folder".to_string(),
                "99:?".to_string(),
            ],
        );
    }

    #[test]
    fn blocker_summary_empty_deps_returns_empty() {
        let state = state_with(vec![], vec![], vec![]);
        assert!(blocker_summary(&state, &[]).is_empty());
    }

    #[tokio::test]
    async fn enqueue_and_dequeue() {
        let q = TaskQueue::new();
        let id = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "file.ts")).await;
        assert!(id > 0);

        let task = q.next_task().await;
        assert_eq!(task.id, id);
        assert_eq!(task.kind, TaskKind::ProcessFile);
        assert_eq!(task.status, TaskStatus::Running);
    }

    #[tokio::test]
    async fn barrier_unblocks_when_deps_complete() {
        let q = TaskQueue::new();

        // Enqueue two file tasks
        let f1 = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        let f2 = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "b.ts")).await;

        // Enqueue barrier that depends on both
        let barrier = q.enqueue(
            Task::new(TaskKind::ResolveEdges, "repo", "")
                .blocked_by(vec![f1, f2])
        ).await;

        // Barrier should be blocked
        let status = q.status().await;
        assert_eq!(status.pending, 2);
        assert_eq!(status.blocked, 1);

        // Process file tasks
        let t1 = q.next_task().await;
        q.complete(t1.id).await;

        // Still blocked (one dep remaining)
        let status = q.status().await;
        assert_eq!(status.blocked, 1);
        assert_eq!(status.pending, 1);

        let t2 = q.next_task().await;
        q.complete(t2.id).await;

        // Barrier should now be pending
        let status = q.status().await;
        assert_eq!(status.blocked, 0);
        assert_eq!(status.pending, 1);

        // Can dequeue barrier
        let bt = q.next_task().await;
        assert_eq!(bt.id, barrier);
        assert_eq!(bt.kind, TaskKind::ResolveEdges);
    }

    #[tokio::test]
    async fn add_dependency_after_creation() {
        let q = TaskQueue::new();

        // Create barrier first with no deps
        let barrier = q.enqueue(
            Task::new(TaskKind::ResolveEdges, "repo", "").blocked_by(vec![])
        ).await;

        // Barrier starts as Pending (no deps)
        // Now add file tasks and wire them as deps
        let f1 = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        q.add_dependency(barrier, f1).await;

        // Barrier should now be blocked... but it was already Pending.
        // This is a design decision: add_dependency only works on Blocked tasks.
        // For dynamic barriers, create with a placeholder dep.
    }

    #[tokio::test]
    async fn failed_task_unblocks_dependents() {
        let q = TaskQueue::new();
        let f1 = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        let _barrier = q.enqueue(
            Task::new(TaskKind::ResolveEdges, "repo", "").blocked_by(vec![f1])
        ).await;

        let t = q.next_task().await;
        q.fail(t.id, "parse error".into()).await;

        // Barrier should be unblocked even though dep failed
        let status = q.status().await;
        assert_eq!(status.pending, 1);
        assert_eq!(status.blocked, 0);
    }

    #[tokio::test]
    async fn sse_events_broadcast() {
        let q = TaskQueue::new();
        let mut rx = q.sender().subscribe();

        let id = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        let evt = rx.recv().await.unwrap();
        assert!(matches!(evt, TaskEvent::Queued { task_id } if task_id == id));

        let task = q.next_task().await;
        let evt = rx.recv().await.unwrap();
        assert!(matches!(evt, TaskEvent::Started { task_id, .. } if task_id == id));

        q.complete(task.id).await;
        let evt = rx.recv().await.unwrap();
        assert!(matches!(evt, TaskEvent::Completed { task_id, .. } if task_id == id));
    }

    #[tokio::test]
    async fn has_pending_kind_path_matches_kind_and_path() {
        let q = TaskQueue::new();
        let run_a = "aaaaaaaa-0000-0000-0000-000000000000";
        let run_b = "bbbbbbbb-0000-0000-0000-000000000000";

        // Empty queue: nothing pending for either run.
        assert!(!q.has_pending_kind_path(TaskKind::AdvanceRun, run_a).await);

        q.enqueue(Task::new(TaskKind::AdvanceRun, "", run_a)).await;

        // Same kind AND path → pending.
        assert!(q.has_pending_kind_path(TaskKind::AdvanceRun, run_a).await);
        // Same kind, different path → not pending (per-run scoping).
        assert!(!q.has_pending_kind_path(TaskKind::AdvanceRun, run_b).await);
        // Same path, different kind → not pending.
        assert!(!q.has_pending_kind_path(TaskKind::ProcessFile, run_a).await);

        // A running (dequeued) AdvanceRun still counts as pending for its run.
        let t = q.next_task().await;
        assert_eq!(t.path, run_a);
        assert!(q.has_pending_kind_path(TaskKind::AdvanceRun, run_a).await,
            "a running tick still blocks a duplicate enqueue");

        // Once completed, it's gone from the active set.
        q.complete(t.id).await;
        assert!(!q.has_pending_kind_path(TaskKind::AdvanceRun, run_a).await);
    }

    #[tokio::test]
    async fn has_pending_kind_folder_matches_kind_and_folder() {
        let q = TaskQueue::new();
        let lib_a = "aaaaaaaa-1111-1111-1111-111111111111";
        let lib_b = "bbbbbbbb-1111-1111-1111-111111111111";
        // IndexLibrary carries the lib id in folder_path, the name in path.
        q.enqueue(Task::new(TaskKind::IndexLibrary, lib_a, "some-lib")).await;
        assert!(q.has_pending_kind_folder(TaskKind::IndexLibrary, lib_a).await, "same kind + folder → pending");
        assert!(!q.has_pending_kind_folder(TaskKind::IndexLibrary, lib_b).await, "different lib id → not pending");
        // Keyed on folder_path (the lib id), NOT the name in path.
        assert!(!q.has_pending_kind_folder(TaskKind::IndexLibrary, "some-lib").await, "the name is not the key");
    }

    #[tokio::test]
    async fn enqueue_unique_dedupes_same_kind_folder_path() {
        // Single-writer guard (D6e / W5): a second enqueue of the same
        // (kind, folder_path, path) while one is still pending or running must
        // be deduped, so two concurrent scans of the same folder can't both run
        // and defeat the graph-idempotency fixes.
        let q = TaskQueue::new();

        // First admit returns an id.
        let id1 = q.enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo")).await;
        assert!(id1.is_some(), "first enqueue_unique admits the task");
        assert_eq!(q.status().await.pending, 1);

        // Duplicate (same kind + folder + path) while pending → skipped, no new row.
        let dup = q.enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo")).await;
        assert!(dup.is_none(), "a duplicate is not enqueued twice");
        assert_eq!(q.status().await.pending, 1, "still exactly one pending");

        // A different path is admitted.
        let other = q.enqueue_unique(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        assert!(other.is_some(), "a different (kind, path) is admitted");
        assert_eq!(q.status().await.pending, 2);

        // A running (dequeued) task still blocks a re-enqueue — the writer is
        // still active, so the guard must include running tasks.
        let running = q.next_task().await;
        let dup_running = q.enqueue_unique(
            Task::new(running.kind.clone(), &running.folder_path, &running.path)
        ).await;
        assert!(dup_running.is_none(), "a running task still dedupes a re-enqueue");
    }

    #[tokio::test]
    async fn enqueue_unique_dedupes_blocked_task() {
        // The guard must match a task parked in `blocked` (waiting on an unmet
        // dep), not only pending/running — else a duplicate slips in while the
        // first task is blocked on a barrier.
        let q = TaskQueue::new();
        // blocked_by a dep id that never completes → the task stays in `blocked`.
        let id1 = q.enqueue_unique(
            Task::new(TaskKind::ProcessGitFolder, "repo", "repo").blocked_by(vec![9999])
        ).await;
        assert!(id1.is_some(), "first (blocked) task is admitted");
        assert_eq!(q.status().await.blocked, 1);
        assert_eq!(q.status().await.pending, 0);

        let dup = q.enqueue_unique(
            Task::new(TaskKind::ProcessGitFolder, "repo", "repo").blocked_by(vec![9999])
        ).await;
        assert!(dup.is_none(), "a duplicate is deduped even while the first is blocked");
        assert_eq!(q.status().await.blocked, 1, "still exactly one blocked task");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn enqueue_unique_race_admits_exactly_one() {
        // Atomicity: the doc claims two concurrent callers can't both slip a
        // duplicate past the guard. Prove it — 50 tasks racing the identical
        // (kind, folder_path, path) must yield exactly ONE admit.
        use std::sync::Arc;
        let q = Arc::new(TaskQueue::new());
        let mut handles = Vec::new();
        for _ in 0..50 {
            let q = q.clone();
            handles.push(tokio::spawn(async move {
                q.enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo")).await
            }));
        }
        let mut admitted = 0usize;
        for h in handles {
            if h.await.unwrap().is_some() { admitted += 1; }
        }
        assert_eq!(admitted, 1, "exactly one concurrent caller is admitted");
        assert_eq!(q.status().await.pending, 1, "queue holds exactly one task");
    }

    #[tokio::test]
    async fn progress_counts() {
        let q = TaskQueue::new();
        q.enqueue(Task::new(TaskKind::ProcessFile, "repo1", "a.ts")).await;
        q.enqueue(Task::new(TaskKind::ProcessFile, "repo1", "b.ts")).await;
        q.enqueue(Task::new(TaskKind::ProcessFile, "repo2", "c.ts")).await;

        let progress = q.progress().await;
        assert_eq!(progress["repo1"].total, 2);
        assert_eq!(progress["repo1"].pending, 2);
        assert_eq!(progress["repo2"].total, 1);

        let t = q.next_task().await;
        let progress = q.progress().await;
        assert_eq!(progress[&t.folder_path].running, 1);
    }
}
