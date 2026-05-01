//! LogCollector — session log manager.
//!
//! STUB: All methods are no-ops. The bootstrap-logging session will replace
//! this with the real implementation (file rotation, JSON session writes, etc.).
//! Do not add logic here — wait for the logging session to land.

/// Tauri managed state for bootstrap session logging.
/// Register once via `.manage(LogCollector::new())` in lib.rs.
pub struct LogCollector;

impl LogCollector {
    pub fn new() -> Self {
        Self
    }

    /// Start a new log session. No-op until logging session ships.
    pub fn session_start(&self, _session_id: &str) {}

    /// Append a trace record. No-op until logging session ships.
    pub fn append_trace(&self, _session_id: &str, _trace: &serde_json::Value) {}

    /// End a log session. No-op until logging session ships.
    pub fn session_end(&self, _session_id: &str, _success: bool) {}

    /// Return all stored sessions. Returns empty until logging session ships.
    pub fn get_sessions(&self) -> Vec<serde_json::Value> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_does_not_panic() {
        let _ = LogCollector::new();
    }

    #[test]
    fn stub_methods_are_no_ops() {
        let lc = LogCollector::new();
        lc.session_start("sess-001");
        lc.append_trace("sess-001", &serde_json::json!({"step": "test"}));
        lc.session_end("sess-001", true);
        let sessions = lc.get_sessions();
        assert!(sessions.is_empty(), "stub should return empty sessions");
    }
}
