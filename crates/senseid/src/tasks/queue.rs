//! Task queue with dependency tracking and barrier support.
//!
//! Tasks are enqueued with optional depends_on IDs. Blocked tasks
//! automatically become Pending when all dependencies complete.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use serde::Serialize;
use tokio::sync::{Mutex, Notify, broadcast};
use super::{Task, TaskKind, TaskStatus};
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
    repo_running_count: HashMap<String, usize>,
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
                repo_running_count: HashMap::new(),
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

    pub fn max_concurrent_repos(&self) -> usize {
        self.max_concurrent_repos.load(Ordering::SeqCst)
    }

    pub fn sender(&self) -> &broadcast::Sender<TaskEvent> {
        &self.tx
    }

    /// Enqueue a task. Returns the assigned task ID.
    pub async fn enqueue(&self, mut task: Task) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        task.id = id;

        let mut state = self.inner.lock().await;

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
                task.depends_on = unmet;
                state.blocked.push(task);
            }
        } else {
            state.pending.push_back(task);
        }

        let _ = self.tx.send(TaskEvent::Queued { task_id: id });
        drop(state);
        self.notify.notify_one();
        id
    }

    /// Enqueue multiple tasks at once. Returns their IDs.
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
                    let count = state.repo_running_count.get(&t.repo_id).copied().unwrap_or(0);
                    count < max_repos
                });

                if let Some(idx) = pos {
                    let mut task = state.pending.remove(idx).unwrap();
                    task.status = TaskStatus::Running;
                    task.started_at = Some(std::time::Instant::now());

                    *state.repo_running_count.entry(task.repo_id.clone()).or_insert(0) += 1;
                    let task_clone = task.clone();
                    state.running.insert(task.id, task);

                    let _ = self.tx.send(TaskEvent::Started {
                        task_id: task_clone.id,
                        repo_id: task_clone.repo_id.clone(),
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
            if let Some(count) = state.repo_running_count.get_mut(&task.repo_id) {
                *count = count.saturating_sub(1);
                if *count == 0 { state.repo_running_count.remove(&task.repo_id); }
            }

            let repo_id = task.repo_id.clone();
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
                        newly_pending.push(unblocked);
                    }
                }
            }
            for t in newly_pending {
                state.pending.push_back(t);
            }

            let _ = self.tx.send(TaskEvent::Completed {
                task_id,
                repo_id,
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

            if let Some(count) = state.repo_running_count.get_mut(&task.repo_id) {
                *count = count.saturating_sub(1);
                if *count == 0 { state.repo_running_count.remove(&task.repo_id); }
            }

            let repo_id = task.repo_id.clone();
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
                        state.pending.push_back(unblocked);
                    }
                }
            }

            let _ = self.tx.send(TaskEvent::Failed {
                task_id,
                repo_id,
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
            let p = result.entry(t.repo_id.clone()).or_default();
            p.total += 1;
            p.pending += 1;
        }
        for t in &state.blocked {
            let p = result.entry(t.repo_id.clone()).or_default();
            p.total += 1;
            p.pending += 1; // blocked counts as pending from user perspective
        }
        for t in state.running.values() {
            let p = result.entry(t.repo_id.clone()).or_default();
            p.total += 1;
            p.running += 1;
            p.current_file = Some(t.path.clone());
        }
        // Don't include completed in totals (they're done)

        result
    }

    /// Get queue status summary.
    pub async fn status(&self) -> QueueStatus {
        let state = self.inner.lock().await;
        QueueStatus {
            pending: state.pending.len(),
            blocked: state.blocked.len(),
            running: state.running.len(),
            completed: state.completed.len(),
            repos_active: state.repo_running_count.len(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(progress[&t.repo_id].running, 1);
    }
}
