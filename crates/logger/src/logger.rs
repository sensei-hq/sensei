//! Logger — structured logger with context propagation (kavach pattern).

use serde_json::Value;
use std::sync::Arc;
use crate::types::{LogLevel, LogEntry};
use crate::writer::LogWriter;

/// Structured logger with context propagation.
///
/// Create a root logger for a component, then derive child loggers
/// with `with_method()` or `with_context()` to add context fields.
#[derive(Clone)]
pub struct Logger {
    writer: Arc<LogWriter>,
    level: LogLevel,
    /// Which component: "daemon", "cli", "mcp", "app".
    pub running_on: String,
    /// Flexible context bag (module, method, task_id, etc.).
    pub context: Value,
}

impl Logger {
    /// Create a root logger for a component.
    ///
    /// - `writer`: where logs go (PG, HTTP, buffered, or noop)
    /// - `level`: minimum level to log
    /// - `running_on`: "daemon", "cli", "mcp", "app"
    /// - `module`: top-level module ("tasks", "api", "indexer", "gateway")
    pub fn new(
        writer: Arc<LogWriter>,
        level: LogLevel,
        running_on: &str,
        module: &str,
    ) -> Self {
        Self {
            writer,
            level,
            running_on: running_on.to_string(),
            context: serde_json::json!({ "module": module }),
        }
    }

    /// Create a noop logger (for tests).
    pub fn noop() -> Self {
        Self {
            writer: LogWriter::noop(),
            level: LogLevel::Error,
            running_on: "test".to_string(),
            context: serde_json::json!({}),
        }
    }

    /// Derive a child logger with an additional method context.
    pub fn with_method(&self, method: &str) -> Self {
        let mut ctx = self.context.clone();
        if let Some(obj) = ctx.as_object_mut() {
            obj.insert("method".into(), Value::String(method.into()));
        }
        Self {
            writer: self.writer.clone(),
            level: self.level,
            running_on: self.running_on.clone(),
            context: ctx,
        }
    }

    /// Derive a child logger with arbitrary extra context fields.
    pub fn with_context(&self, extra: Value) -> Self {
        let mut ctx = self.context.clone();
        if let (Some(base), Some(extra_obj)) = (ctx.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                base.insert(k.clone(), v.clone());
            }
        }
        Self {
            writer: self.writer.clone(),
            level: self.level,
            running_on: self.running_on.clone(),
            context: ctx,
        }
    }

    // ── Level methods ───────────────────────────────────────────────────

    pub async fn error(&self, message: &str, data: Option<Value>, error: Option<Value>) {
        self.log(LogLevel::Error, message, data, error).await;
    }

    pub async fn warn(&self, message: &str, data: Option<Value>) {
        self.log(LogLevel::Warn, message, data, None).await;
    }

    pub async fn info(&self, message: &str, data: Option<Value>) {
        self.log(LogLevel::Info, message, data, None).await;
    }

    pub async fn debug(&self, message: &str, data: Option<Value>) {
        self.log(LogLevel::Debug, message, data, None).await;
    }

    pub async fn trace_log(&self, message: &str, data: Option<Value>) {
        self.log(LogLevel::Trace, message, data, None).await;
    }

    // ── Core ────────────────────────────────────────────────────────────

    async fn log(&self, level: LogLevel, message: &str, data: Option<Value>, error: Option<Value>) {
        if level > self.level { return; }

        let entry = LogEntry {
            level: level.as_str().to_string(),
            running_on: self.running_on.clone(),
            logged_at: chrono::Utc::now().to_rfc3339(),
            message: message.to_string(),
            context: self.context.clone(),
            data,
            error,
        };

        // Fire-and-forget — logging never blocks the caller.
        let writer = self.writer.clone();
        tokio::spawn(async move {
            writer.write(&entry).await;
        });
    }
}
