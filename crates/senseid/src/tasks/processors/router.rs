//! File router — selects the right processor based on file extension.

use std::path::Path;
use super::types::*;
use super::{code, doc, config};

/// Process a single file. Routes to the correct processor by extension.
/// Pure function — no DB, no side effects.
pub fn process_file(abs_path: &str, repo_path: &str, repo_id: &str) -> Result<FileProcessResult, String> {
    let file_path = Path::new(abs_path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", abs_path));
    }

    let repo = Path::new(repo_path);
    let rel_path = file_path.strip_prefix(repo)
        .unwrap_or(file_path)
        .to_string_lossy().to_string();

    let ext = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read: {}", e))?;

    // Route by file type
    match ext {
        // Documents
        "md" | "mdx" => Ok(doc::process(abs_path, &rel_path, &content, repo_id, repo_path)),

        // Config
        "json" | "toml" | "yaml" | "yml" => Ok(config::process(abs_path, &rel_path, ext)),

        // Code — try language adapter
        _ => {
            if let Some(result) = code::process(abs_path, &rel_path, ext, &content, repo_id) {
                Ok(result)
            } else {
                // Unknown file type — register as file node
                let tag = classify_file_tag(&rel_path, ext);
                Ok(FileProcessResult::minimal(
                    format!("file:{}", abs_path),
                    rel_path,
                    abs_path.to_string(),
                    "file",
                    &tag,
                ))
            }
        }
    }
}
