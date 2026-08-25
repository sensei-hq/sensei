//! File router — selects the right processor based on file extension.

use super::types::*;
use super::{code, config, doc};
use std::path::Path;

/// Process a single file. Routes to the correct processor by extension.
/// Pure function — no DB, no side effects.
pub fn process_file(
    abs_path: &str,
    repo_path: &str,
    repo_id: &str,
) -> Result<FileProcessResult, String> {
    let file_path = Path::new(abs_path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", abs_path));
    }

    let repo = Path::new(repo_path);
    let rel_path = file_path.strip_prefix(repo).unwrap_or(file_path).to_string_lossy().to_string();

    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let raw = std::fs::read_to_string(file_path).map_err(|e| format!("Failed to read: {}", e))?;

    // Normalize line endings to LF before any parsing. tree-sitter byte offsets
    // and the per-line / byte buffers used for text extraction must agree; a
    // CRLF file otherwise yields node byte-ranges (counted over the CRLF source)
    // that overshoot a CR-stripped extraction buffer and panic the parser
    // (observed on CRLF-terminated .py files). Avoids the realloc for LF files.
    let content =
        if raw.contains('\r') { raw.replace("\r\n", "\n").replace('\r', "\n") } else { raw };

    // Route by file type
    match ext {
        // Documents
        "md" | "mdx" => Ok(doc::process(abs_path, &rel_path, &content, repo_id, repo_path)),

        // Plain text docs (llms.txt, etc.)
        "txt" => Ok(doc::process(abs_path, &rel_path, &content, repo_id, repo_path)),

        // Config — the ConfigAdapter registry decides which extensions count
        // as config files (currently json / jsonl / toml / yaml / yml).
        e if crate::adapters::config::config_adapter_for_ext(e).is_some() => {
            Ok(config::process(abs_path, &rel_path, ext))
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    /// A CRLF-terminated source file must parse without panicking. Before line
    /// endings were normalized, tree-sitter byte offsets (over the CRLF bytes)
    /// overshot the extraction buffer and panicked on such files.
    #[test]
    fn process_file_handles_crlf_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crlf.py");
        // Function body padded so node byte-ranges are well past a CR-stripped
        // length — the condition that triggered the panic.
        let mut src = String::from("def greet(name):\r\n");
        for i in 0..200 {
            src.push_str(&format!("    x{i} = compute(name, {i})  # pad line\r\n"));
        }
        src.push_str("    return name\r\n");
        std::fs::write(&path, &src).unwrap();

        let result = process_file(&path.to_string_lossy(), dir.path().to_str().unwrap(), "repo");
        let parsed = result.expect("CRLF file should parse, not error");
        assert!(
            parsed.symbols.iter().any(|s| s.name == "greet"),
            "expected the `greet` function symbol, got {:?}",
            parsed.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }
}
