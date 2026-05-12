//! sensei-logger — structured logging (kavach pattern).
//!
//! Generic `Logger` with pluggable writers:
//! - `PgWriter` — direct PG insert (daemon)
//! - `ApiWriter` — HTTP POST to daemon `/api/logs` (CLI, MCP, app)
//! - `Buffered` — caches until PG is available (bootstrap)
//! - `Noop` — tests
//!
//! Context is a flexible jsonb bag. Callers inject module, method, task_id, etc.

pub mod types;
pub mod writer;
pub mod logger;

pub use types::{LogLevel, LogEntry};
pub use writer::LogWriter;
pub use logger::Logger;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_ordering() {
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn log_level_round_trip() {
        for level in [LogLevel::Error, LogLevel::Warn, LogLevel::Info, LogLevel::Debug, LogLevel::Trace] {
            assert_eq!(LogLevel::parse(level.as_str()), level);
        }
    }

    #[test]
    fn log_level_parse_unknown_defaults_to_error() {
        assert_eq!(LogLevel::parse("unknown"), LogLevel::Error);
    }

    #[test]
    fn with_method_adds_context() {
        let logger = Logger::noop();
        let child = logger.with_method("process_file");
        assert_eq!(child.context["method"], "process_file");
    }

    #[test]
    fn with_context_merges() {
        let logger = Logger::new(LogWriter::noop(), LogLevel::Info, "daemon", "tasks");
        let child = logger.with_context(serde_json::json!({"task_id": 42}));
        assert_eq!(child.context["module"], "tasks");
        assert_eq!(child.context["task_id"], 42);
    }

    #[tokio::test]
    async fn noop_logger_does_not_panic() {
        let logger = Logger::noop();
        logger.info("test", Some(serde_json::json!({"k": "v"}))).await;
        logger.error("err", None, Some(serde_json::json!({"msg": "oops"}))).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    #[test]
    fn log_entry_serializes() {
        let entry = LogEntry {
            level: "info".into(),
            running_on: "daemon".into(),
            logged_at: "2026-05-11T00:00:00Z".into(),
            message: "test".into(),
            context: serde_json::json!({"module": "tasks"}),
            data: Some(serde_json::json!({"items": 5})),
            error: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"level\":\"info\""));
        assert!(json.contains("\"module\":\"tasks\""));
    }
}
