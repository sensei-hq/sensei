//! Config file processor — .json, .toml, .yaml, .yml files.
//! Extracts metadata without parsing symbols.

use super::types::*;

/// Process a config file.
pub fn process(abs_path: &str, rel_path: &str, ext: &str) -> FileProcessResult {
    FileProcessResult::minimal(
        format!("file:{}", abs_path),
        rel_path.to_string(),
        abs_path.to_string(),
        "file",
        "config",
    )
}
