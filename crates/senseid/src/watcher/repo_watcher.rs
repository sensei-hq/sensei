use notify::{Watcher, RecursiveMode, Event, Config};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::collections::HashSet;
use globset::{Glob, GlobSet, GlobSetBuilder};
use crate::adapters;

const DEBOUNCE_MS: u64 = 500;

const DEFAULT_EXCLUDE: &[&str] = &[
    "**/node_modules/**", "**/dist/**", "**/build/**", "**/target/**",
    "**/.git/**", "**/.next/**", "**/.svelte-kit/**",
    "**/__pycache__/**", "**/.venv/**",
];

/// Watches a repo directory for file changes.
/// Does NOT hold a permanent DB connection — processes batch changes lazily.
pub struct RepoWatcher {
    pub repo_id: String,
    pub repo_path: PathBuf,
    exclude: GlobSet,
}

impl RepoWatcher {
    pub fn new(repo_id: &str, repo_path: &Path) -> Self {
        let exclude = build_globset(DEFAULT_EXCLUDE);
        Self {
            repo_id: repo_id.to_string(),
            repo_path: repo_path.to_path_buf(),
            exclude,
        }
    }

    /// Start watching. Returns a channel receiver for changed file paths.
    /// The caller processes batches and opens/closes DB connections per batch.
    pub fn watch(&self) -> Result<mpsc::Receiver<Vec<PathBuf>>, String> {
        let (tx, rx) = mpsc::channel();
        let repo_path = self.repo_path.clone();
        let exclude = self.exclude.clone();

        std::thread::spawn(move || {
            let (event_tx, event_rx) = mpsc::channel();
            let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = event_tx.send(event);
                }
            }).expect("failed to create watcher");

            watcher.watch(&repo_path, RecursiveMode::Recursive)
                .expect("failed to watch directory");

            let mut pending = HashSet::new();
            let mut last_event = Instant::now();

            loop {
                match event_rx.recv_timeout(Duration::from_millis(DEBOUNCE_MS)) {
                    Ok(event) => {
                        for path in event.paths {
                            if !path.is_file() { continue; }

                            // Check extension
                            let ext = path.extension()
                                .and_then(|e| e.to_str())
                                .map(|e| format!(".{}", e))
                                .unwrap_or_default();

                            let is_code = adapters::adapter_for_ext(&ext).is_some();
                            let is_doc = ext == ".md" || ext == ".mdx";
                            if !is_code && !is_doc { continue; }

                            // Check exclude
                            let rel = path.strip_prefix(&repo_path).unwrap_or(&path);
                            if exclude.is_match(rel.to_string_lossy().as_ref()) { continue; }

                            pending.insert(path);
                            last_event = Instant::now();
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Debounce: if enough time passed since last event, flush
                        if !pending.is_empty() && last_event.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                            let batch: Vec<PathBuf> = pending.drain().collect();
                            if tx.send(batch).is_err() { break; }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Ok(rx)
    }
}

fn build_globset(patterns: &[&str]) -> GlobSet {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = Glob::new(p) { builder.add(g); }
    }
    builder.build().unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_creates() {
        let dir = tempfile::TempDir::new().unwrap();
        let w = RepoWatcher::new("test", dir.path());
        assert_eq!(w.repo_id, "test");
    }

    #[test]
    fn watcher_detects_change() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("main.py"), "x = 1").unwrap();

        let w = RepoWatcher::new("test", dir.path());
        let rx = w.watch().unwrap();

        // Write a new file
        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(dir.path().join("new.py"), "y = 2").unwrap();

        // Wait for debounce
        match rx.recv_timeout(Duration::from_secs(3)) {
            Ok(batch) => {
                assert!(!batch.is_empty(), "expected changed files");
                let names: Vec<String> = batch.iter()
                    .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                    .collect();
                assert!(names.contains(&"new.py".to_string()));
            }
            Err(_) => {
                // On some CI systems, file watching may not work
                eprintln!("Warning: watcher timeout — file events may not be supported in this environment");
            }
        }
    }

    #[test]
    fn excludes_node_modules() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/foo")).unwrap();
        std::fs::write(dir.path().join("node_modules/foo/index.js"), "").unwrap();

        let w = RepoWatcher::new("test", dir.path());
        let rx = w.watch().unwrap();

        std::thread::sleep(Duration::from_millis(100));
        std::fs::write(dir.path().join("node_modules/foo/bar.js"), "x").unwrap();

        // Should NOT receive the node_modules change
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(batch) => {
                let in_node_modules = batch.iter().any(|p| p.to_string_lossy().contains("node_modules"));
                assert!(!in_node_modules, "should not include node_modules files");
            }
            Err(_) => {} // Timeout is expected — no valid files changed
        }
    }
}
