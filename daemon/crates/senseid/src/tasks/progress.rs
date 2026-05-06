//! SSE progress events for task queue.

use serde::Serialize;

/// Events broadcast via SSE as tasks transition states.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TaskEvent {
    Queued { task_id: u64 },
    Started { task_id: u64, folder_path: String, kind: String, path: String },
    Completed { task_id: u64, folder_path: String, kind: String },
    Failed { task_id: u64, folder_path: String, kind: String, error: String },
    /// Emitted once when a folder's file tasks are first queued — carries total file count.
    FolderQueued { folder_path: String, files_total: u32 },
}

/// Accumulated progress for a single repo.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RepoProgress {
    pub total: u32,
    pub pending: u32,
    pub running: u32,
    pub completed: u32,
    pub failed: u32,
    pub current_file: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_progress_default() {
        let p = RepoProgress::default();
        assert_eq!(p.total, 0);
        assert_eq!(p.running, 0);
        assert!(p.current_file.is_none());
    }

    #[test]
    fn task_event_serializes() {
        let evt = TaskEvent::Completed { task_id: 1, folder_path: "/code/app".into(), kind: "process_file".into() };
        let json = serde_json::to_string(&evt).unwrap();
        assert!(json.contains("\"event\":\"completed\""));
        assert!(json.contains("\"folder_path\":\"/code/app\""));
    }
}
