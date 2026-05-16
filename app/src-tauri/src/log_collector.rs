//! Session-scoped diagnostic log collector.

#![allow(dead_code)] // Public API wired in Task 8 (Tauri commands + managed state)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

/// System metadata captured at session start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os:        String,
    pub arch:      String,
    pub ram_gb:    u64,
    pub cpu_cores: usize,
}

/// A single incremental log entry (app logger).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id:    String,
    pub ts:    String,
    /// "info" | "warn" | "error"
    pub level: String,
    /// "ui" | "api" | "sidecar" | "data_load"
    pub layer: String,
    pub step:  String,
    pub msg:   String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:  Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err:   Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

/// A completed log session — stored on disk as one JSON file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSession {
    pub id:          String,
    pub module:      String,
    pub started_at:  String,
    pub app_version: String,
    pub system_info: SystemInfo,
    pub outcome:     String,
    pub duration_ms: u64,
    /// BootstrapTrace[] or LogEntry[] serialised as JSON values.
    pub traces:      Vec<serde_json::Value>,
}

struct ActiveSession {
    module:        String,
    started_at:    String,
    start_instant: Instant,
    app_version:   String,
    system_info:   SystemInfo,
    entries:       Vec<serde_json::Value>,
    /// 0 = info, 1 = warn, 2 = error
    max_level:     u8,
}

/// Thread-safe log session manager.
/// Writes via temp-file+rename to `{log_dir}/{module}/{session_id}.json` on session end.
pub struct LogCollector {
    sessions: Mutex<HashMap<String, ActiveSession>>,
    log_dir:  PathBuf,
}

impl LogCollector {
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            log_dir,
        }
    }

    /// Start a new incremental session. Returns the generated session ID.
    pub fn start_session(
        &self,
        module: &str,
        app_version: &str,
        system_info: SystemInfo,
    ) -> String {
        let session_id = next_session_id();
        let dir = self.log_dir.join(module);
        self.rotate_if_needed(&dir);

        let session = ActiveSession {
            module:        module.to_string(),
            started_at:    chrono::Utc::now().to_rfc3339(),
            start_instant: Instant::now(),
            app_version:   app_version.to_string(),
            system_info,
            entries:       Vec::new(),
            max_level:     0,
        };

        self.sessions.lock().unwrap().insert(session_id.clone(), session);
        session_id
    }

    /// Append a log entry to an open session. No-op if session_id is unknown.
    pub fn append_entry(&self, session_id: &str, entry: LogEntry) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(s) = sessions.get_mut(session_id) {
            let level = match entry.level.as_str() {
                "error" => 2,
                "warn"  => 1,
                _       => 0,
            };
            if level > s.max_level {
                s.max_level = level;
            }
            match serde_json::to_value(&entry) {
                Ok(val) => s.entries.push(val),
                Err(e) => eprintln!("log_collector: failed to serialise entry: {e}"),
            }
        }
    }

    /// Finalize an incremental session and write it to disk.
    pub fn end_session(&self, session_id: &str) {
        let session = {
            let mut sessions = self.sessions.lock().unwrap();
            match sessions.remove(session_id) {
                Some(s) => s,
                None => {
                    eprintln!("log_collector: session {session_id} not found");
                    return;
                }
            }
        };

        let outcome = match session.max_level {
            2 => "failed",
            1 => "partial",
            _ => "success",
        };
        let duration_ms = session.start_instant.elapsed().as_millis() as u64;

        let log_session = LogSession {
            id:          session_id.to_string(),
            module:      session.module,
            started_at:  session.started_at,
            app_version: session.app_version,
            system_info: session.system_info,
            outcome:     outcome.to_string(),
            duration_ms,
            traces:      session.entries,
        };

        self.write_session(&log_session);
    }

    /// Write a pre-built session using a temp-file+rename approach to avoid partial writes
    /// (used by bootstrap command after run_with_traces).
    pub fn write_session(&self, session: &LogSession) {
        let dir = self.log_dir.join(&session.module);
        self.rotate_if_needed(&dir);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("log_collector: write failed: {e}");
            return;
        }
        let path = dir.join(format!("{}.json", session.id));
        let json = match serde_json::to_string_pretty(session) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("log_collector: write failed: {e}");
                return;
            }
        };
        let tmp_path = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp_path, &json) {
            eprintln!("log_collector: write failed: {e}");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp_path, &path) {
            eprintln!("log_collector: rename failed: {e}");
            let _ = std::fs::remove_file(&tmp_path);
        }
    }

    /// Read all sessions, optionally filtered by module, newest-first.
    pub fn read_sessions(&self, module: Option<&str>) -> Vec<LogSession> {
        let dirs: Vec<PathBuf> = if let Some(m) = module {
            vec![self.log_dir.join(m)]
        } else {
            std::fs::read_dir(&self.log_dir)
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect()
        };

        let mut results = Vec::new();
        for dir in dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            match serde_json::from_str::<LogSession>(&content) {
                                Ok(session) => results.push(session),
                                Err(e) => eprintln!("log_collector: failed to parse session file {path:?}: {e}"),
                            }
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        results
    }

    /// Delete oldest files when module directory has >= 20 JSON files.
    fn rotate_if_needed(&self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();

        if files.len() >= 20 {
            files.sort();  // sess-{n:08x} names are lexicographically ordered by creation sequence
            for file in files.iter().take(files.len() - 19) {
                let _ = std::fs::remove_file(file);
            }
        }
    }
}

fn next_session_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("sess-{n:08x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_sys_info() -> SystemInfo {
        SystemInfo {
            os:        "macOS 15.0".to_string(),
            arch:      "arm64".to_string(),
            ram_gb:    16,
            cpu_cores: 10,
        }
    }

    #[test]
    fn start_append_end_writes_file() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());

        let sid = collector.start_session(
            "wizard",
            "0.1.0",
            make_sys_info(),
        );
        assert!(sid.starts_with("sess-"));

        let entry = LogEntry {
            id:    "e1".to_string(),
            ts:    "2026-05-01T10:00:00Z".to_string(),
            level: "info".to_string(),
            layer: "ui".to_string(),
            step:  "prefs_load".to_string(),
            msg:   "Preferences loaded".to_string(),
            data:  None,
            err:   None,
            stack: None,
        };
        collector.append_entry(&sid, entry);
        collector.end_session(&sid);

        let path = tmp.path().join("wizard").join(format!("{sid}.json"));
        assert!(path.exists(), "session file should be written");
        let content = std::fs::read_to_string(&path).unwrap();
        let session: LogSession = serde_json::from_str(&content).unwrap();
        assert_eq!(session.module, "wizard");
        assert_eq!(session.traces.len(), 1);
        assert_eq!(session.outcome, "success");
    }

    #[test]
    fn error_entry_sets_outcome_failed() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());
        let sid = collector.start_session("wizard", "0.1.0", make_sys_info());

        collector.append_entry(&sid, LogEntry {
            id: "e1".to_string(), ts: "2026-05-01T10:00:00Z".to_string(),
            level: "error".to_string(), layer: "api".to_string(),
            step: "save_prefs".to_string(), msg: "Save failed".to_string(),
            data: None, err: Some("timeout".to_string()), stack: None,
        });

        collector.end_session(&sid);
        let path = tmp.path().join("wizard").join(format!("{sid}.json"));
        let session: LogSession = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(session.outcome, "failed");
    }

    #[test]
    fn write_session_atomic() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());
        let session = LogSession {
            id:          "sess-00000001".to_string(),
            module:      "bootstrap".to_string(),
            started_at:  "2026-05-01T10:00:00Z".to_string(),
            app_version: "0.1.0".to_string(),
            system_info: make_sys_info(),
            outcome:     "success".to_string(),
            duration_ms: 1234,
            traces:      vec![],
        };
        collector.write_session(&session);
        let path = tmp.path().join("bootstrap").join("sess-00000001.json");
        assert!(path.exists());
    }

    #[test]
    fn file_rotation_keeps_at_most_20() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());

        // Write 22 sessions directly using a "fixture-" prefix that cannot be
        // generated by next_session_id(), preventing collision when tests run in parallel.
        let dir = tmp.path().join("bootstrap");
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..22u64 {
            let sid = format!("fixture-{i:08x}");
            let session = LogSession {
                id: sid.clone(), module: "bootstrap".to_string(),
                started_at: format!("2026-05-01T10:00:{i:02}Z"),
                app_version: "0.1.0".to_string(), system_info: make_sys_info(),
                outcome: "success".to_string(), duration_ms: i * 100, traces: vec![],
            };
            let path = dir.join(format!("{sid}.json"));
            std::fs::write(&path, serde_json::to_string(&session).unwrap()).unwrap();
        }

        // Now start a session (which triggers rotation)
        let sid = collector.start_session("bootstrap", "0.1.0", make_sys_info());
        collector.end_session(&sid);

        let count = std::fs::read_dir(&dir).unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
            .count();
        assert_eq!(count, 20, "rotation should keep exactly 20 session files");
    }

    #[test]
    fn read_sessions_returns_newest_first() {
        let tmp = TempDir::new().unwrap();
        let collector = LogCollector::new(tmp.path().to_path_buf());

        for (i, ts) in ["2026-05-01T08:00:00Z", "2026-05-01T10:00:00Z", "2026-05-01T09:00:00Z"]
            .iter()
            .enumerate()
        {
            let sid = format!("sess-0000000{i}");
            collector.write_session(&LogSession {
                id: sid.clone(), module: "bootstrap".to_string(), started_at: ts.to_string(),
                app_version: "0.1.0".to_string(), system_info: make_sys_info(),
                outcome: "success".to_string(), duration_ms: 100, traces: vec![],
            });
        }

        let sessions = collector.read_sessions(Some("bootstrap"));
        assert_eq!(sessions.len(), 3);
        assert!(sessions[0].started_at > sessions[1].started_at);
        assert!(sessions[1].started_at > sessions[2].started_at);
    }
}
