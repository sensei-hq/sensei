//! Task queue with dependency tracking and barrier support.
//!
//! Tasks are enqueued with optional depends_on IDs. Blocked tasks
//! automatically become Pending when all dependencies complete.

use super::progress::TaskEvent;
use super::{Task, TaskStatus};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{Mutex, Notify, broadcast};

const DEFAULT_MAX_CONCURRENT_REPOS: usize = 3;

pub struct TaskQueue {
    inner: Mutex<QueueState>,
    notify: Notify,
    next_id: AtomicU64,
    tx: broadcast::Sender<TaskEvent>,
    max_concurrent_repos: std::sync::atomic::AtomicUsize,
    /// When this queue — and therefore this id space — came into existence.
    ///
    /// `next_id` starts at 1 on every construction, so a task id is unique only
    /// WITHIN one daemon session; `activity.task_executions` accumulates across
    /// sessions and will happily hold dozens of unrelated rows for id 1. Any
    /// lookup by task id must therefore be scoped to the session that issued it,
    /// and the queue is the right owner of that boundary because it defines the
    /// id space.
    session_start: chrono::DateTime<chrono::Utc>,
    /// Set once by [`TaskQueue::begin_shutdown`]; never cleared. While set,
    /// [`TaskQueue::next_task`] hands out nothing, so the worker pool drains
    /// down to whatever was already running instead of pulling in new work the
    /// process has no time left to finish.
    ///
    /// It is an atomic rather than a `QueueState` field so [`TaskQueue::
    /// is_shutting_down`] can be a plain `fn` — a shutdown path that has to
    /// `.await` a lock just to ask "are we stopping?" is a shutdown path that
    /// can deadlock against the work it is trying to stop.
    shutting_down: AtomicBool,
}

struct QueueState {
    pending: VecDeque<Task>,
    blocked: Vec<Task>,
    running: HashMap<u64, Task>,
    completed: Vec<Task>, // last N for history
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
            session_start: chrono::Utc::now(),
            tx,
            max_concurrent_repos: std::sync::atomic::AtomicUsize::new(max_repos),
            shutting_down: AtomicBool::new(false),
        }
    }

    /// Stop handing out new work. Tasks already running are left alone to
    /// finish; nothing queued behind them starts.
    ///
    /// Called from the daemon's graceful-shutdown path, so the cost of being
    /// wrong is asymmetric: starting one more task on the way out means a
    /// half-written graph and a connection that outlives the process, while
    /// declining to start one usually costs only a re-run on the next boot.
    ///
    /// USUALLY, not always, and the difference is worth knowing: the reconcile
    /// tick enqueues exactly ONE kind — `ScanRoot` — which fans out to the whole
    /// scan chain, so anything scan-shaped recovers by itself. The kinds reached
    /// another way (embedding, community detection, analysis, library import,
    /// the metric chain) are NOT re-enqueued by it and simply wait for whatever
    /// normally triggers them. That is why the counts below are logged rather
    /// than assumed harmless.
    ///
    /// Takes the queue lock, and that is the point rather than an accident:
    /// `next_task` reads the flag inside the same critical section it uses to
    /// pick a task, so once this returns there is no in-flight selection still
    /// holding a stale `false` that could hand out one last task. A bare atomic
    /// store would leave exactly that window open.
    // `allow` only until `api::server`'s graceful-shutdown path calls this: the
    // queue half of #212 lands ahead of the signal half that drives it.
    #[allow(dead_code)]
    pub async fn begin_shutdown(&self) {
        let state = self.inner.lock().await;
        let already = self.shutting_down.swap(true, Ordering::SeqCst);
        if !already {
            // The counts are the record of what shutdown abandoned — the next
            // boot should re-enqueue them, and if it doesn't, this line is the
            // evidence of what went missing.
            tracing::info!(
                pending = state.pending.len(),
                blocked = state.blocked.len(),
                running = state.running.len(),
                "task queue shutting down: no new work will be handed out; \
                 running tasks are left to finish",
            );
        }
    }

    /// Whether [`begin_shutdown`](Self::begin_shutdown) has been called.
    #[allow(dead_code)]
    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::SeqCst)
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

    /// Like [`enqueue`], but a no-op returning `None` when an already-queued twin
    /// COVERS this request — the single-writer guard (D6e / W5). The check and the
    /// insert happen under **one** lock acquisition, so two concurrent callers
    /// can't both slip a duplicate past the guard (the check-then-enqueue race).
    /// Enqueue sites that must never double-scan a folder/file (ScanRoot /
    /// ProcessGitFolder / ProcessFile) use this instead of [`enqueue`].
    ///
    /// COVERAGE IS NOT IDENTITY, and the difference is `force`. A twin matches on
    /// `(kind, folder_path, path)`, but it only *covers* this request if it is at
    /// least as forceful: a forced pass re-parses everything an unforced one
    /// would, and an unforced pass does not.
    ///
    /// The key used to be the triple alone. Because the reconcile tick keeps an
    /// unforced `ProcessGitFolder` queued for every repository, a user's
    /// `--force` always arrived behind one and was dropped — returning `None`
    /// with no log while `/api/scan` echoed `"forced": true` to the caller.
    /// Measured 2026-09-30: `mark_folder_unparsed`'s log line appeared zero times
    /// in 2 GB of daemon log. No force had ever run.
    pub async fn enqueue_unique(&self, task: Task) -> Option<u64> {
        let mut state = self.inner.lock().await;
        let force = task.force;
        let matches = |t: &Task| {
            t.kind == task.kind && t.folder_path == task.folder_path && t.path == task.path
        };

        if state
            .pending
            .iter()
            .chain(state.blocked.iter())
            .chain(state.running.values())
            .any(|t| matches(t) && (t.force || !force))
        {
            return None; // covered; the guard drops the lock on return
        }

        // Not covered, and `force` is the only reason it isn't. A twin still
        // WAITING can be upgraded in place — cheaper and more correct than a
        // second pass over the same folder, which is the other way to lose.
        let st = &mut *state;
        if let Some(t) = st.pending.iter_mut().chain(st.blocked.iter_mut()).find(|t| matches(t)) {
            t.force = true;
            return Some(t.id);
        }

        // Only a RUNNING twin is left. Its handler already read `force`, so
        // mutating it would change nothing a worker can still see — the forced
        // request has to be admitted as a task of its own.
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
            let unmet: Vec<u64> = task
                .depends_on
                .iter()
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
    /// added to a post-processing barrier.
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
    ///
    /// Once [`begin_shutdown`](Self::begin_shutdown) has been called this parks
    /// forever instead of returning. That is deliberately the SAME shape the
    /// queue already has for "nothing is dispatchable" — a worker whose repo is
    /// at its concurrency cap also parks on `notified()` indefinitely — so no
    /// caller learns a new state. The alternative, returning `Option<Task>`,
    /// would push an `else { break }` into every one of the worker and
    /// scheduler loops that await this, to express something none of them can
    /// act on: there is no useful work for a worker to do after shutdown but
    /// stop, and letting the process exit stops it. The flag is re-read on every
    /// pass, not just on entry, because `complete`/`fail` call `notify_waiters`
    /// and would otherwise let one more task through per in-flight task.
    pub async fn next_task(&self) -> Task {
        loop {
            {
                let mut state = self.inner.lock().await;
                if self.shutting_down.load(Ordering::SeqCst) {
                    // Read under the lock so it orders against `begin_shutdown`.
                    drop(state);
                    self.notify.notified().await;
                    continue;
                }
                // Among the pending tasks whose repo isn't at the concurrency
                // limit, pick the one with the best (lowest) `kind_priority` so
                // the light metric-backfill chain preempts a bulk boot re-index
                // instead of starving behind it. The per-repo cap
                // (`count < max_repos`) is the SAME gate as before — priority
                // only reorders selection among already-startable tasks, it
                // never bypasses the cap. Tie-break by index so FIFO order is
                // preserved *within* a priority band (deterministic).
                let max_repos = self.max_concurrent_repos.load(Ordering::SeqCst);
                let pos = state
                    .pending
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| {
                        let count =
                            state.folder_running_count.get(&t.folder_path).copied().unwrap_or(0);
                        count < max_repos
                    })
                    .min_by_key(|(idx, t)| (kind_priority(&t.kind), *idx))
                    .map(|(idx, _)| idx);

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
                if *count == 0 {
                    state.folder_running_count.remove(&task.folder_path);
                }
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

            let _ = self.tx.send(TaskEvent::Completed { task_id, folder_path, kind });
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
                if *count == 0 {
                    state.folder_running_count.remove(&task.folder_path);
                }
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

            let _ = self.tx.send(TaskEvent::Failed { task_id, folder_path, kind, error });
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

    /// Test-only: every task the queue is holding (pending, blocked, running,
    /// completed) as whole [`Task`]s.
    ///
    /// [`Self::snapshot`] projects this down to `(kind, folder_path, path)`. That
    /// projection was for a long time the ONLY way to look at the queue, and it
    /// discards `force` — which is part of why a dropped force flag stayed
    /// invisible from outside. Anything asserting on task STATE wants this one.
    #[cfg(test)]
    pub async fn snapshot_tasks(&self) -> Vec<Task> {
        let s = self.inner.lock().await;
        s.pending
            .iter()
            .chain(s.blocked.iter())
            .chain(s.running.values())
            .chain(s.completed.iter())
            .cloned()
            .collect()
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
        self.snapshot_tasks().await.into_iter().map(|t| (t.kind, t.folder_path, t.path)).collect()
    }

    /// When this id space began — the lower bound for any `task_executions`
    /// lookup by task id. See the field docs on `session_start`.
    pub fn session_start(&self) -> chrono::DateTime<chrono::Utc> {
        self.session_start
    }

    /// One task by id, wherever it currently sits in the queue.
    ///
    /// Exists for the follow endpoints: a task that has been enqueued but has not
    /// started yet has NO `activity.task_executions` row (the executor writes
    /// that on start), so the durable log alone would 404 a task that genuinely
    /// exists and is about to run — the one answer a follower must never get.
    ///
    /// `completed` is searched too: it is a bounded ring the queue keeps for
    /// status, and finding a task there is strictly better than reporting it
    /// unknown.
    pub async fn find_task(&self, task_id: u64) -> Option<Task> {
        let s = self.inner.lock().await;
        s.pending
            .iter()
            .chain(s.blocked.iter())
            .chain(s.running.values())
            .chain(s.completed.iter())
            .find(|t| t.id == task_id)
            .cloned()
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

/// Scheduling priority of a task kind for [`TaskQueue::next_task`] — LOWER runs
/// first. The metric-backfill chain (`AnalyzeProject` → `ComputeProjectMetrics` →
/// `ComputeGroupMetrics` → `ComputeHealth`) is HIGH priority (`0`) so a light metric
/// compute preempts a bulk boot re-index (heavy per-file / graph tasks) that
/// would otherwise saturate the small worker pool and leave the backfill
/// `pending` indefinitely. Everything else is NORMAL (`1`). Kept as a small,
/// explicit, pure match so the HIGH set is obvious and unit-testable. This only
/// reorders *selection among already-startable tasks*; it never bypasses the
/// per-repo concurrency cap (that gate is applied first in `next_task`).
fn kind_priority(kind: &super::TaskKind) -> u8 {
    // Reads the kind descriptor rather than re-listing the HIGH set here.
    // The list lived in two places (this match and the watchdog tiering) and
    // nothing kept them agreeing.
    if kind.is_high_priority() { 0 } else { 1 }
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
            let kind = state
                .running
                .values()
                .find(|t| t.id == *dep_id)
                .map(|t| t.kind.to_string())
                .or_else(|| {
                    state.pending.iter().find(|t| t.id == *dep_id).map(|t| t.kind.to_string())
                })
                .or_else(|| {
                    state.blocked.iter().find(|t| t.id == *dep_id).map(|t| t.kind.to_string())
                })
                .or_else(|| {
                    state.completed.iter().find(|t| t.id == *dep_id).map(|t| t.kind.to_string())
                })
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
            vec![task_with(22, TaskKind::DetectCommunities)],
            vec![task_with(33, TaskKind::ProcessGitFolder)],
        );
        let summary = blocker_summary(&state, &[11, 22, 33, 99]);
        assert_eq!(
            summary,
            vec![
                "11:process_file".to_string(),
                "22:detect_communities".to_string(),
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
        let barrier = q
            .enqueue(Task::new(TaskKind::DetectCommunities, "repo", "").blocked_by(vec![f1, f2]))
            .await;

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
        assert_eq!(bt.kind, TaskKind::DetectCommunities);
    }

    #[tokio::test]
    async fn add_dependency_after_creation() {
        let q = TaskQueue::new();

        // Create barrier first with no deps
        let barrier =
            q.enqueue(Task::new(TaskKind::DetectCommunities, "repo", "").blocked_by(vec![])).await;

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
        let _barrier = q
            .enqueue(Task::new(TaskKind::DetectCommunities, "repo", "").blocked_by(vec![f1]))
            .await;

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
        assert!(
            q.has_pending_kind_path(TaskKind::AdvanceRun, run_a).await,
            "a running tick still blocks a duplicate enqueue"
        );

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
        assert!(
            q.has_pending_kind_folder(TaskKind::IndexLibrary, lib_a).await,
            "same kind + folder → pending"
        );
        assert!(
            !q.has_pending_kind_folder(TaskKind::IndexLibrary, lib_b).await,
            "different lib id → not pending"
        );
        // Keyed on folder_path (the lib id), NOT the name in path.
        assert!(
            !q.has_pending_kind_folder(TaskKind::IndexLibrary, "some-lib").await,
            "the name is not the key"
        );
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
        let dup_running = q
            .enqueue_unique(Task::new(running.kind.clone(), &running.folder_path, &running.path))
            .await;
        assert!(dup_running.is_none(), "a running task still dedupes a re-enqueue");
    }

    /// A FORCED task must not be swallowed by an unforced twin.
    ///
    /// The dedup key was `(kind, folder_path, path)` — `force` was not in it. The
    /// reconcile tick keeps an unforced `ProcessGitFolder` queued for every repo,
    /// so a user's `sensei scan --force <repo>` always landed behind one and was
    /// dropped: `enqueue_unique` returned `None` and the flag went nowhere.
    ///
    /// Measured 2026-09-30: `"forced rescan: files reopened for parsing"` — the
    /// line `mark_folder_unparsed` logs on EVERY force — appeared zero times in
    /// 2 GB of daemon log, while `/api/scan` was happily echoing `"forced":true`
    /// back to the caller. The echo exists precisely because a silently-ignored
    /// force is the worst outcome; it was being ignored one layer below the echo.
    ///
    /// Mutation that must break this test: drop the `t.force || !force` term from
    /// the coverage predicate, so any twin covers a forced request again.
    #[tokio::test]
    async fn a_forced_task_is_not_dropped_behind_an_unforced_twin() {
        let q = TaskQueue::new();

        let first = q.enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo")).await;
        assert!(first.is_some(), "the unforced task is admitted");

        let forced = q
            .enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo").forced(true))
            .await;
        assert!(forced.is_some(), "a FORCED task is not a duplicate of an unforced one");

        // UPGRADED IN PLACE, not duplicated: running the same folder twice would
        // be the other way to lose, and it doubles the work it was asked to do once.
        assert_eq!(q.status().await.pending, 1, "the twin is upgraded, not duplicated");
        let t = q.next_task().await;
        assert!(t.force, "the task the worker receives must carry the force flag");
    }

    /// The reverse does NOT admit: an unforced request is fully covered by a
    /// forced twin, because a forced pass re-parses everything the unforced one
    /// would have.
    ///
    /// Mutation that must break this test: make the predicate `t.force == force`,
    /// which would let an unforced twin in behind a forced one and scan twice.
    #[tokio::test]
    async fn an_unforced_task_behind_a_forced_twin_is_still_dropped() {
        let q = TaskQueue::new();
        q.enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo").forced(true))
            .await
            .expect("the forced task is admitted");

        let plain = q.enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo")).await;
        assert!(plain.is_none(), "a forced twin already covers an unforced request");
        assert_eq!(q.status().await.pending, 1);

        // And two forced requests still collapse to one.
        let again = q
            .enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo").forced(true))
            .await;
        assert!(again.is_none(), "an identical forced twin is still a duplicate");
        assert_eq!(q.status().await.pending, 1);
    }

    /// A RUNNING twin cannot be upgraded — it already read `force` when the
    /// handler started — so a forced request must be admitted as its own task.
    ///
    /// Mutation that must break this test: upgrade `running` in place and return
    /// `Some(id)`. The mutation of a task already past its force check is
    /// invisible, and the force silently never happens.
    #[tokio::test]
    async fn a_forced_task_behind_a_running_unforced_twin_is_admitted() {
        let q = TaskQueue::new();
        q.enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo"))
            .await
            .expect("admitted");
        let running = q.next_task().await;
        assert!(!running.force, "precondition: the running twin is unforced");

        let forced = q
            .enqueue_unique(Task::new(TaskKind::ProcessGitFolder, "repo", "repo").forced(true))
            .await;
        assert!(forced.is_some(), "a running unforced twin cannot satisfy a forced request");
        assert_eq!(q.status().await.pending, 1, "admitted as a new task, since it cannot upgrade");
    }

    #[tokio::test]
    async fn enqueue_unique_dedupes_blocked_task() {
        // The guard must match a task parked in `blocked` (waiting on an unmet
        // dep), not only pending/running — else a duplicate slips in while the
        // first task is blocked on a barrier.
        let q = TaskQueue::new();
        // blocked_by a dep id that never completes → the task stays in `blocked`.
        let id1 = q
            .enqueue_unique(
                Task::new(TaskKind::ProcessGitFolder, "repo", "repo").blocked_by(vec![9999]),
            )
            .await;
        assert!(id1.is_some(), "first (blocked) task is admitted");
        assert_eq!(q.status().await.blocked, 1);
        assert_eq!(q.status().await.pending, 0);

        let dup = q
            .enqueue_unique(
                Task::new(TaskKind::ProcessGitFolder, "repo", "repo").blocked_by(vec![9999]),
            )
            .await;
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
            if h.await.unwrap().is_some() {
                admitted += 1;
            }
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

    #[test]
    fn kind_priority_ranks_metric_backfill_above_everything_else() {
        // Lock the exact HIGH-priority set (the metric-backfill chain) so a
        // future edit can't silently drop a member into the NORMAL band.
        for k in [
            TaskKind::AnalyzeProject,
            TaskKind::ComputeProjectMetrics,
            TaskKind::ComputeGroupMetrics,
            TaskKind::ComputeHealth,
        ] {
            assert_eq!(kind_priority(&k), 0, "{k} is high priority (metric backfill)");
        }
        // A representative spread of the bulk / barrier / global kinds are NORMAL.
        for k in [
            TaskKind::ScanRoot,
            TaskKind::ProcessGitFolder,
            TaskKind::ProcessFile,
            TaskKind::DetectCommunities,
            TaskKind::DetectCommunities,
            TaskKind::EmbedNodes,
            TaskKind::AggregateCorrections,
        ] {
            assert_eq!(kind_priority(&k), 1, "{k} is normal priority");
        }
    }

    #[tokio::test]
    async fn next_task_prioritizes_metric_backfill_over_bulk_indexing() {
        // Root cause (live): a boot re-index (heavy per-file / graph tasks)
        // saturates the small worker pool, so the light metric-backfill chain
        // sits `pending` indefinitely under pure FIFO. `next_task` must let a
        // high-priority metric compute PREEMPT a bulk index that was enqueued
        // earlier — while preserving FIFO *within* a priority band.
        let q = TaskQueue::new();

        // A bulk re-index task lands FIRST (folder A) …
        q.enqueue(Task::new(TaskKind::ScanRoot, "A", "A")).await;
        // … then two metric computes for folder B (the backfill chain).
        q.enqueue(Task::new(TaskKind::ComputeGroupMetrics, "B", "group1")).await;
        q.enqueue(Task::new(TaskKind::ComputeGroupMetrics, "B", "group2")).await;

        // Priority preempts FIFO: both ComputeGroupMetrics come out before the
        // earlier-enqueued ScanRoot, and the two computes keep enqueue order
        // (FIFO tie-break within the high-priority band).
        let t1 = q.next_task().await;
        assert_eq!(t1.kind, TaskKind::ComputeGroupMetrics);
        assert_eq!(t1.path, "group1", "FIFO preserved within the high-priority band");
        let t2 = q.next_task().await;
        assert_eq!(t2.kind, TaskKind::ComputeGroupMetrics);
        assert_eq!(t2.path, "group2", "second ComputeGroupMetrics keeps enqueue order");
        // … and only then the bulk index.
        let t3 = q.next_task().await;
        assert_eq!(t3.kind, TaskKind::ScanRoot);
    }

    /// `is_shutting_down` is the observable half of the flag: false on a fresh
    /// queue, true once `begin_shutdown` has returned, and it stays true.
    ///
    /// Mutation that must break this test: have `is_shutting_down` return a
    /// constant, or have `begin_shutdown` not store the flag.
    #[tokio::test]
    async fn is_shutting_down_reflects_the_flag() {
        let q = TaskQueue::new();
        assert!(!q.is_shutting_down(), "a fresh queue is not shutting down");

        q.begin_shutdown().await;
        assert!(q.is_shutting_down(), "the flag is set once begin_shutdown returns");

        // Idempotent — SIGTERM arriving twice must not un-set it.
        q.begin_shutdown().await;
        assert!(q.is_shutting_down(), "a second begin_shutdown leaves it set");
    }

    /// (a) Work queued BEFORE shutdown is still handed out, right up to the
    /// moment the flag flips. This is the control arm for the test below: it
    /// proves the task is genuinely startable, so a later refusal to hand it out
    /// can only be the shutdown flag and not a per-repo cap or an empty queue.
    ///
    /// Mutation that must break this test: park in `next_task` unconditionally
    /// (i.e. check nothing), which would make shutdown "work" by never
    /// dispatching anything at all.
    #[tokio::test]
    async fn a_task_queued_before_shutdown_is_still_handed_out() {
        let q = TaskQueue::new();
        let id = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;

        let task =
            tokio::time::timeout(std::time::Duration::from_secs(5), q.next_task()).await.expect(
                "a pending task on a queue that is not shutting down must be handed out at once",
            );
        assert_eq!(task.id, id);
        assert_eq!(task.status, TaskStatus::Running);
        assert!(!q.is_shutting_down(), "handing out work did not flip the flag");
    }

    /// (b) Once `begin_shutdown` has returned, a task that is pending AND
    /// startable is no longer handed out — `next_task` parks instead of
    /// returning it.
    ///
    /// The first `next_task` call is load-bearing: it proves the queue is in a
    /// state where a second dispatch WOULD happen (same folder, 1 of 3 repo
    /// slots used, one task pending), so the timeout that follows measures the
    /// flag and nothing else.
    ///
    /// Mutation that must break this test: drop the `is_shutting_down` check
    /// from `next_task`'s selection block.
    #[tokio::test]
    async fn no_new_task_is_handed_out_once_shutting_down() {
        let q = TaskQueue::new();
        q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "b.ts")).await;

        let first = tokio::time::timeout(std::time::Duration::from_secs(5), q.next_task())
            .await
            .expect("precondition: this queue hands out work");
        assert_eq!(first.path, "a.ts");

        q.begin_shutdown().await;

        // `b.ts` is still pending and still startable — only the flag stands
        // between it and a worker.
        let status = q.status().await;
        assert_eq!(status.pending, 1, "precondition: a startable task is still queued");
        assert_eq!(status.running, 1, "precondition: the repo is below its concurrency cap");

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(250), q.next_task())
                .await
                .is_err(),
            "a shutting-down queue must park rather than start more work",
        );
        assert_eq!(q.status().await.pending, 1, "the parked task was not consumed");
    }

    /// Shutdown stops NEW work; it does not disturb work already running. The
    /// in-flight task still completes, still unblocks its dependents, and the
    /// `notify_waiters` that completion fires must not be a back door through
    /// which a parked worker picks up fresh work.
    ///
    /// Mutation that must break this test: check the flag only on entry to
    /// `next_task` instead of on every pass of its loop, so the wake-up that
    /// `complete` sends lets one more task through.
    #[tokio::test]
    async fn a_running_task_still_finishes_after_begin_shutdown() {
        let q = TaskQueue::new();
        let dep = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        q.enqueue(Task::new(TaskKind::DetectCommunities, "repo", "").blocked_by(vec![dep])).await;

        let running = tokio::time::timeout(std::time::Duration::from_secs(5), q.next_task())
            .await
            .expect("precondition: this queue hands out work");
        assert_eq!(running.id, dep);

        q.begin_shutdown().await;

        // The running task is untouched by shutdown and completes normally …
        q.complete(running.id).await;
        let status = q.status().await;
        assert_eq!(status.running, 0, "the in-flight task finished");
        assert_eq!(status.completed, 1);
        // … including the dependent it releases, which becomes pending …
        assert_eq!(status.blocked, 0, "the dependent was released as usual");
        assert_eq!(status.pending, 1);

        // … but the completion's wake-up must not start it.
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(250), q.next_task())
                .await
                .is_err(),
            "completing a task during shutdown must not hand out the task it unblocked",
        );
    }

    /// The live shape of shutdown: a worker is ALREADY parked in `next_task`
    /// when SIGTERM lands (it is idle — that is what a worker does between
    /// tasks). It must not be handed work by the wake-up that the last
    /// in-flight task's `complete` sends.
    ///
    /// This is the case an entry-only flag check would miss, and the one that
    /// matters: checking on entry stops workers that arrive after shutdown,
    /// but every worker that was idle at SIGTERM is already past that point.
    ///
    /// Mutation that must break this test: hoist the `is_shutting_down` check
    /// out of `next_task`'s loop so it runs once on entry.
    #[tokio::test]
    async fn a_worker_parked_before_shutdown_is_not_woken_with_new_work() {
        use std::sync::Arc;
        let q = Arc::new(TaskQueue::new());

        // One task running, one barrier blocked behind it: nothing dispatchable.
        let dep = q.enqueue(Task::new(TaskKind::ProcessFile, "repo", "a.ts")).await;
        q.enqueue(Task::new(TaskKind::DetectCommunities, "repo", "").blocked_by(vec![dep])).await;
        // Timed, not bare: `next_task` parks forever when it declines to
        // dispatch, so an un-timed call here would hang the whole test binary
        // under a regression instead of failing it.
        let running = tokio::time::timeout(std::time::Duration::from_secs(5), q.next_task())
            .await
            .expect("precondition: this queue hands out work");
        assert_eq!(running.id, dep);

        // A worker goes idle and parks — before any shutdown.
        let mut worker = tokio::spawn({
            let q = q.clone();
            async move { q.next_task().await.id }
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!worker.is_finished(), "precondition: the worker parked, nothing was dispatchable");
        assert!(!q.is_shutting_down(), "precondition: it parked BEFORE shutdown");

        q.begin_shutdown().await;
        // The last in-flight task finishes, releasing the barrier and waking
        // every parked worker.
        q.complete(running.id).await;
        assert_eq!(q.status().await.pending, 1, "the barrier was released and is startable");

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(250), &mut worker).await.is_err(),
            "a worker parked before shutdown must stay parked, not pick up the released barrier",
        );
        worker.abort();
    }

    #[tokio::test]
    async fn next_task_priority_never_bypasses_per_repo_cap() {
        // Priority reorders *selection among startable tasks*; it must NOT
        // bypass the per-repo concurrency cap. A folder already at `max_repos`
        // running is skipped even for a HIGH-priority task, and a startable
        // NORMAL-priority task in another folder is returned instead.
        let q = TaskQueue::with_max_repos(1);

        // Fill folder A to its cap (1 running).
        q.enqueue(Task::new(TaskKind::ProcessGitFolder, "A", "A")).await;
        let running_a = q.next_task().await;
        assert_eq!(running_a.folder_path, "A");

        // A HIGH-priority task for the capped folder A, plus a NORMAL task for
        // the free folder B.
        q.enqueue(Task::new(TaskKind::ComputeGroupMetrics, "A", "group")).await;
        q.enqueue(Task::new(TaskKind::ProcessFile, "B", "b.ts")).await;

        // Folder A is full, so its high-priority task is NOT startable; the
        // startable normal-priority task in folder B is returned instead.
        let next = q.next_task().await;
        assert_eq!(next.folder_path, "B");
        assert_eq!(next.kind, TaskKind::ProcessFile);
    }
}
