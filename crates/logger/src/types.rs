//! Core types — LogLevel and LogEntry.

use serde_json::Value;

/// Log severity levels (matching kavach: error=1, warn=2, info=3, debug=4, trace=5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 1,
    Warn  = 2,
    Info  = 3,
    Debug = 4,
    Trace = 5,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn  => "warn",
            Self::Info  => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "error" => Self::Error,
            "warn"  => Self::Warn,
            "info"  => Self::Info,
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            _       => Self::Error,
        }
    }
}

/// A single structured log entry (matches public.logs columns).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub level: String,
    pub running_on: String,
    pub logged_at: String,
    pub message: String,
    pub context: Value,
    pub data: Option<Value>,
    pub error: Option<Value>,
}
