use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, Notify};
use crate::types::IndexProgress;

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexJob {
    pub repo_id: String,
    pub repo_path: String,
    pub force: bool,
    pub status: JobStatus,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Indexing,
    Done,
    Failed,
}

/// Event emitted during indexing (broadcast to SSE subscribers).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type")]
pub enum IndexEvent {
    #[serde(rename = "queued")]
    Queued { repo_id: String, position: usize },
    #[serde(rename = "started")]
    Started { repo_id: String, files_total: u32 },
    #[serde(rename = "progress")]
    Progress(IndexProgress),
    #[serde(rename = "completed")]
    Completed {
        repo_id: String,
        files_indexed: u32,
        functions_indexed: u32,
        types_indexed: u32,
        edges_created: u32,
        duration_ms: u64,
    },
    #[serde(rename = "failed")]
    Failed { repo_id: String, error: String },
}

/// Shared index queue state.
pub struct IndexQueue {
    queue: Mutex<VecDeque<IndexJob>>,
    current: Mutex<Option<IndexJob>>,
    history: Mutex<Vec<IndexJob>>,
    notify: Notify,
    tx: broadcast::Sender<IndexEvent>,
}

impl IndexQueue {
    pub fn new() -> (Arc<Self>, broadcast::Receiver<IndexEvent>) {
        let (tx, rx) = broadcast::channel(256);
        let q = Arc::new(Self {
            queue: Mutex::new(VecDeque::new()),
            current: Mutex::new(None),
            history: Mutex::new(Vec::new()),
            notify: Notify::new(),
            tx,
        });
        (q, rx)
    }

    /// Enqueue an index job. Returns queue position.
    pub async fn enqueue(&self, repo_id: String, repo_path: String, force: bool) -> usize {
        let mut q = self.queue.lock().await;
        // Dedup: don't enqueue if already queued or currently indexing
        if q.iter().any(|j| j.repo_id == repo_id) {
            return q.iter().position(|j| j.repo_id == repo_id).unwrap();
        }
        {
            let cur = self.current.lock().await;
            if cur.as_ref().map_or(false, |j| j.repo_id == repo_id) {
                return 0; // already running
            }
        }
        let pos = q.len();
        q.push_back(IndexJob {
            repo_id: repo_id.clone(),
            repo_path,
            force,
            status: JobStatus::Queued,
        });
        let _ = self.tx.send(IndexEvent::Queued { repo_id, position: pos });
        self.notify.notify_one();
        pos
    }

    /// Get the next job to process (blocks until available).
    pub async fn next_job(&self) -> IndexJob {
        loop {
            {
                let mut q = self.queue.lock().await;
                if let Some(mut job) = q.pop_front() {
                    job.status = JobStatus::Indexing;
                    *self.current.lock().await = Some(job.clone());
                    return job;
                }
            }
            self.notify.notified().await;
        }
    }

    /// Mark current job as done.
    pub async fn mark_done(&self, repo_id: &str) {
        let mut cur = self.current.lock().await;
        if let Some(mut job) = cur.take() {
            if job.repo_id == repo_id {
                job.status = JobStatus::Done;
                self.history.lock().await.push(job);
            }
        }
    }

    /// Mark current job as failed.
    pub async fn mark_failed(&self, repo_id: &str) {
        let mut cur = self.current.lock().await;
        if let Some(mut job) = cur.take() {
            if job.repo_id == repo_id {
                job.status = JobStatus::Failed;
                self.history.lock().await.push(job);
            }
        }
    }

    /// Get a snapshot of the full queue state for the API.
    pub async fn status(&self) -> serde_json::Value {
        let q = self.queue.lock().await;
        let cur = self.current.lock().await;
        let hist = self.history.lock().await;
        serde_json::json!({
            "current": *cur,
            "queued": q.iter().collect::<Vec<_>>(),
            "recent": hist.iter().rev().take(10).collect::<Vec<_>>(),
        })
    }

    /// Get the broadcast sender for emitting progress events.
    pub fn sender(&self) -> &broadcast::Sender<IndexEvent> {
        &self.tx
    }

    /// Subscribe to events.
    pub fn subscribe(&self) -> broadcast::Receiver<IndexEvent> {
        self.tx.subscribe()
    }
}

/// Callback trait for the pipeline to report progress.
pub trait ProgressCallback: Send + Sync {
    fn on_file(&self, progress: IndexProgress);
}

/// Broadcast-based progress reporter.
pub struct BroadcastProgress {
    tx: broadcast::Sender<IndexEvent>,
}

impl BroadcastProgress {
    pub fn new(tx: broadcast::Sender<IndexEvent>) -> Self {
        Self { tx }
    }
}

impl ProgressCallback for BroadcastProgress {
    fn on_file(&self, progress: IndexProgress) {
        let _ = self.tx.send(IndexEvent::Progress(progress));
    }
}

/// No-op progress (for tests / direct calls).
pub struct NoProgress;
impl ProgressCallback for NoProgress {
    fn on_file(&self, _: IndexProgress) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn enqueue_and_dequeue() {
        let (q, _rx) = IndexQueue::new();
        let pos = q.enqueue("repo1".into(), "/path".into(), false).await;
        assert_eq!(pos, 0);

        let job = q.next_job().await;
        assert_eq!(job.repo_id, "repo1");
        assert_eq!(job.status, JobStatus::Indexing);

        q.mark_done("repo1").await;
        let status = q.status().await;
        assert!(status["current"].is_null());
    }

    #[tokio::test]
    async fn dedup_enqueue() {
        let (q, _rx) = IndexQueue::new();
        q.enqueue("repo1".into(), "/path".into(), false).await;
        q.enqueue("repo1".into(), "/path".into(), false).await;
        let status = q.status().await;
        assert_eq!(status["queued"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn events_broadcast() {
        let (q, mut rx) = IndexQueue::new();
        q.enqueue("repo1".into(), "/path".into(), false).await;
        let event = rx.recv().await.unwrap();
        match event {
            IndexEvent::Queued { repo_id, position } => {
                assert_eq!(repo_id, "repo1");
                assert_eq!(position, 0);
            }
            _ => panic!("Expected Queued event"),
        }
    }
}
