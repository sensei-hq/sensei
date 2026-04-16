//! Hierarchical task queue for scanning, indexing, and watching.
//!
//! Tasks form a dependency tree:
//!   scan_root → process_repo → process_folder → process_file → resolve_edges → build_connections
//!
//! Barrier tasks (resolve_edges, build_connections) wait for all dependencies to complete.

pub mod queue;
pub mod executor;
pub mod handlers;
pub mod progress;
#[cfg(test)]
mod integration_tests;

use serde::{Serialize, Deserialize};
use std::time::Instant;

// ── Task kinds ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    ScanRoot,
    ProcessRepo,
    ProcessFolder,
    ProcessFile,
    DeleteFile,
    DeleteFolder,
    ResolveEdges,
    ResolveLibs,
    ImportLib,
    BranchSwitch,
    BuildConnections,
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScanRoot => write!(f, "scan_root"),
            Self::ProcessRepo => write!(f, "process_repo"),
            Self::ProcessFolder => write!(f, "process_folder"),
            Self::ProcessFile => write!(f, "process_file"),
            Self::DeleteFile => write!(f, "delete_file"),
            Self::DeleteFolder => write!(f, "delete_folder"),
            Self::ResolveEdges => write!(f, "resolve_edges"),
            Self::ResolveLibs => write!(f, "resolve_libs"),
            Self::ImportLib => write!(f, "import_lib"),
            Self::BranchSwitch => write!(f, "branch_switch"),
            Self::BuildConnections => write!(f, "build_connections"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Blocked,    // has unmet dependencies
    Running,
    Completed,
    Failed,
}

// ── Task ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Task {
    pub id: u64,
    pub kind: TaskKind,
    pub repo_id: String,
    pub path: String,                    // file/folder/root path
    pub parent_task_id: Option<u64>,     // for hierarchy tracking
    pub module_id: Option<String>,       // for process_file: which module this file belongs to
    pub branch: Option<String>,          // git branch name (for branch-aware indexing)
    pub url: Option<String>,             // for import_lib: library docs URL
    pub status: TaskStatus,
    pub depends_on: Vec<u64>,            // won't run until these complete
    pub error: Option<String>,
    pub created_at: Instant,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
}

impl Task {
    pub fn new(kind: TaskKind, repo_id: &str, path: &str) -> Self {
        Self {
            id: 0, // assigned by queue
            kind,
            repo_id: repo_id.to_string(),
            path: path.to_string(),
            parent_task_id: None,
            module_id: None,
            branch: None,
            url: None,
            status: TaskStatus::Pending,
            depends_on: Vec::new(),
            error: None,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_parent(mut self, parent_id: u64) -> Self {
        self.parent_task_id = Some(parent_id);
        self
    }

    pub fn with_module(mut self, module_id: &str) -> Self {
        self.module_id = Some(module_id.to_string());
        self
    }

    pub fn with_branch(mut self, branch: &str) -> Self {
        self.branch = Some(branch.to_string());
        self
    }

    pub fn with_url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn blocked_by(mut self, deps: Vec<u64>) -> Self {
        if !deps.is_empty() {
            self.status = TaskStatus::Blocked;
            self.depends_on = deps;
        }
        self
    }

    pub fn is_runnable(&self) -> bool {
        self.status == TaskStatus::Pending
    }

    pub fn is_barrier(&self) -> bool {
        matches!(self.kind, TaskKind::ResolveEdges | TaskKind::ResolveLibs | TaskKind::BuildConnections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_creation() {
        let t = Task::new(TaskKind::ProcessFile, "myrepo", "/path/to/file.ts");
        assert_eq!(t.kind, TaskKind::ProcessFile);
        assert_eq!(t.repo_id, "myrepo");
        assert_eq!(t.status, TaskStatus::Pending);
        assert!(t.is_runnable());
        assert!(!t.is_barrier());
    }

    #[test]
    fn blocked_task() {
        let t = Task::new(TaskKind::ResolveEdges, "myrepo", "")
            .blocked_by(vec![1, 2, 3]);
        assert_eq!(t.status, TaskStatus::Blocked);
        assert!(!t.is_runnable());
        assert!(t.is_barrier());
        assert_eq!(t.depends_on, vec![1, 2, 3]);
    }

    #[test]
    fn task_with_parent_and_module() {
        let t = Task::new(TaskKind::ProcessFile, "repo", "src/main.ts")
            .with_parent(42)
            .with_module("mod:repo:src");
        assert_eq!(t.parent_task_id, Some(42));
        assert_eq!(t.module_id, Some("mod:repo:src".to_string()));
    }

    #[test]
    fn task_kind_display() {
        assert_eq!(TaskKind::ScanRoot.to_string(), "scan_root");
        assert_eq!(TaskKind::ProcessFile.to_string(), "process_file");
        assert_eq!(TaskKind::ResolveEdges.to_string(), "resolve_edges");
    }
}
