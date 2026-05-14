//! Component ID constants — single source of truth for all 10 bootstrap components.
//!
//! Use these constants wherever a component ID string is needed to eliminate
//! scattered string literals and catch typos at compile time.

pub const HOMEBREW:           &str = "homebrew";
pub const POSTGRESQL:         &str = "postgresql";
pub const OLLAMA:             &str = "ollama";
pub const SENSEI:             &str = "sensei";
pub const SENSEID:            &str = "senseid";
pub const SENSEI_MCP:         &str = "sensei_mcp";
pub const POSTGRESQL_SERVICE: &str = "postgresql_service";
pub const OLLAMA_SERVICE:     &str = "ollama_service";
pub const DATABASE:           &str = "database";
pub const DAEMON:             &str = "daemon";
