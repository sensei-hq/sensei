use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tracks dirty (changed) files per repo, fed by file watchers.
/// The queue worker reads and drains the dirty set to trigger incremental re-index.
#[derive(Clone)]
pub struct DirtyTracker {
    inner: Arc<Mutex<DirtyState>>,
}

struct DirtyState {
    /// repo_id → set of absolute paths of changed code files
    code_files: HashMap<String, HashSet<PathBuf>>,
    /// repo_id → set of absolute paths of changed doc files
    doc_files: HashMap<String, HashSet<PathBuf>>,
    /// repo_id → repo_path mapping (needed by worker to know where to index)
    repo_paths: HashMap<String, String>,
}

/// Snapshot of dirty state for a single repo.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirtySnapshot {
    pub repo_id: String,
    pub code_files: Vec<PathBuf>,
    pub doc_files: Vec<PathBuf>,
}

impl DirtyTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DirtyState {
                code_files: HashMap::new(),
                doc_files: HashMap::new(),
                repo_paths: HashMap::new(),
            })),
        }
    }

    /// Register a repo so the tracker knows its path.
    pub async fn register_repo(&self, repo_id: &str, repo_path: &str) {
        let mut state = self.inner.lock().await;
        state.repo_paths.insert(repo_id.to_string(), repo_path.to_string());
    }

    /// Mark files as dirty. Classifies into code vs doc based on extension.
    pub async fn mark_dirty(&self, repo_id: &str, files: Vec<PathBuf>) {
        let mut state = self.inner.lock().await;
        for file in files {
            let ext = file.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if ext == "md" || ext == "mdx" {
                state.doc_files.entry(repo_id.to_string()).or_default().insert(file);
            } else {
                state.code_files.entry(repo_id.to_string()).or_default().insert(file);
            }
        }
    }

    /// Get all repos that have dirty files.
    pub async fn dirty_repos(&self) -> Vec<(String, String, usize)> {
        let state = self.inner.lock().await;
        let mut result = Vec::new();
        for (repo_id, code) in &state.code_files {
            let doc_count = state.doc_files.get(repo_id).map_or(0, |d| d.len());
            let total = code.len() + doc_count;
            if total > 0 {
                if let Some(path) = state.repo_paths.get(repo_id) {
                    result.push((repo_id.clone(), path.clone(), total));
                }
            }
        }
        // Also check repos with only doc changes
        for (repo_id, docs) in &state.doc_files {
            if !docs.is_empty() && !state.code_files.get(repo_id).map_or(false, |c| !c.is_empty()) {
                if let Some(path) = state.repo_paths.get(repo_id) {
                    result.push((repo_id.clone(), path.clone(), docs.len()));
                }
            }
        }
        result
    }

    /// Drain dirty files for a repo and return the snapshot.
    pub async fn drain(&self, repo_id: &str) -> DirtySnapshot {
        let mut state = self.inner.lock().await;
        let code = state.code_files.remove(repo_id).unwrap_or_default();
        let docs = state.doc_files.remove(repo_id).unwrap_or_default();
        DirtySnapshot {
            repo_id: repo_id.to_string(),
            code_files: code.into_iter().collect(),
            doc_files: docs.into_iter().collect(),
        }
    }

    /// Get a read-only snapshot without draining.
    pub async fn peek(&self, repo_id: &str) -> DirtySnapshot {
        let state = self.inner.lock().await;
        let code = state.code_files.get(repo_id).cloned().unwrap_or_default();
        let docs = state.doc_files.get(repo_id).cloned().unwrap_or_default();
        DirtySnapshot {
            repo_id: repo_id.to_string(),
            code_files: code.into_iter().collect(),
            doc_files: docs.into_iter().collect(),
        }
    }

    /// Get dirty status for all repos (for API).
    pub async fn status(&self) -> Vec<DirtySnapshot> {
        let state = self.inner.lock().await;
        let mut all_repos: HashSet<String> = HashSet::new();
        all_repos.extend(state.code_files.keys().cloned());
        all_repos.extend(state.doc_files.keys().cloned());

        all_repos.into_iter().map(|repo_id| {
            let code = state.code_files.get(&repo_id).cloned().unwrap_or_default();
            let docs = state.doc_files.get(&repo_id).cloned().unwrap_or_default();
            DirtySnapshot {
                repo_id,
                code_files: code.into_iter().collect(),
                doc_files: docs.into_iter().collect(),
            }
        }).filter(|s| !s.code_files.is_empty() || !s.doc_files.is_empty())
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mark_and_drain() {
        let tracker = DirtyTracker::new();
        tracker.register_repo("repo1", "/path/repo1").await;

        tracker.mark_dirty("repo1", vec![
            PathBuf::from("/path/repo1/src/main.ts"),
            PathBuf::from("/path/repo1/src/utils.ts"),
            PathBuf::from("/path/repo1/docs/README.md"),
        ]).await;

        let dirty = tracker.dirty_repos().await;
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].2, 3); // 2 code + 1 doc

        let snapshot = tracker.drain("repo1").await;
        assert_eq!(snapshot.code_files.len(), 2);
        assert_eq!(snapshot.doc_files.len(), 1);

        // After drain, should be empty
        let dirty = tracker.dirty_repos().await;
        assert!(dirty.is_empty());
    }

    #[tokio::test]
    async fn peek_does_not_drain() {
        let tracker = DirtyTracker::new();
        tracker.register_repo("repo1", "/path").await;
        tracker.mark_dirty("repo1", vec![PathBuf::from("/path/a.ts")]).await;

        let snap = tracker.peek("repo1").await;
        assert_eq!(snap.code_files.len(), 1);

        // Still dirty after peek
        let dirty = tracker.dirty_repos().await;
        assert_eq!(dirty.len(), 1);
    }
}
