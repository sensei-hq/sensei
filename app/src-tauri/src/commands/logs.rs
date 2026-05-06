//! Log commands — session management and session retrieval.
//!
//! The UI logger sends entries via invoke; the sidecar is the sole writer.

use crate::log_collector::{LogCollector, LogEntry, LogSession, SystemInfo};
use tauri::State;

/// Start a new log session. Returns the session ID.
#[tauri::command]
pub fn log_session_start(
    state: State<LogCollector>,
    module: String,
    app_version: String,
    system_info: SystemInfo,
) -> String {
    state.start_session(&module, &app_version, system_info)
}

/// Append a single log entry to an open session. Fire-and-forget.
#[tauri::command]
pub fn log_entry(state: State<LogCollector>, session_id: String, entry: LogEntry) {
    state.append_entry(&session_id, entry);
}

/// Finalize a session and write it to disk.
#[tauri::command]
pub fn log_session_end(state: State<LogCollector>, session_id: String) {
    state.end_session(&session_id);
}

/// Return all log sessions, optionally filtered by module, newest-first.
#[tauri::command]
pub fn get_log_sessions(
    state: State<LogCollector>,
    module: Option<String>,
) -> Vec<LogSession> {
    state.read_sessions(module.as_deref())
}
