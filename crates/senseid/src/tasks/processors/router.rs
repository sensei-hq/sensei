//! File router — selects the right processor based on file extension.

use super::types::*;
use super::{config, doc};
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

    // BYTES THEN DECODE, so this agrees with the scan gate. `read_to_string`
    // refuses a UTF-16 file that `classify_unscannable` has already called
    // text, which would skip it here with no reason recorded anywhere.
    let bytes = std::fs::read(file_path).map_err(|e| format!("Failed to read: {}", e))?;
    let raw = match crate::classifiers::decode_source(&bytes) {
        crate::classifiers::Decoded::Text(text) => text,
        other => return Err(format!("Not text ({other:?})")),
    };

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

        // NOT CODE. Every code extension is claimed by `indexer::lang`, whose
        // languages are now all in `PRODUCTION_LANGUAGES`, so `process_file`
        // routes those files to the indexer and returns before reaching here.
        // What arrives is a file this build has no parser for — and a file node
        // with a tag is the honest record of that.
        //
        // There used to be a second code parser behind this arm. It is gone: two
        // producers writing symbols into one set of tables under DIFFERENT fqn
        // schemes is what made every cross-file lookup miss, and the one thing
        // that cannot be fixed while both exist.
        _ => {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A CRLF-terminated file must reach its processor without panicking.
    ///
    /// The router normalises line endings BEFORE dispatching, because
    /// tree-sitter byte offsets are counted over the original bytes and a
    /// CR-stripped extraction buffer is shorter — the overshoot panicked the
    /// parser on CRLF files.
    ///
    /// MARKDOWN, not Kotlin. The property is the ROUTER's, so it needs any
    /// route that still exists, and code is no longer one of them: every code
    /// extension is claimed by `indexer::lang`, so `process_file` sends those
    /// files to the indexer and never reaches here. A `.kt` fixture would now
    /// assert nothing about normalisation and everything about the fallback.
    #[test]
    fn process_file_handles_crlf_without_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crlf.md");
        let mut src = String::from("# Title\r\n\r\n");
        for i in 0..200 {
            src.push_str(&format!("Paragraph {i} with enough text to push the offsets out.\r\n"));
        }
        src.push_str("## Section\r\n\r\nBody.\r\n");
        std::fs::write(&path, &src).unwrap();

        let result = process_file(&path.to_string_lossy(), dir.path().to_str().unwrap(), "repo");
        let parsed = result.expect("a CRLF file should be processed, not error");
        // A doc decomposes into SECTIONS, not symbols — the heading is what
        // survives normalisation, so it is what proves the offsets lined up.
        assert!(
            parsed.sections.iter().any(|s| s.heading.contains("Section")),
            "expected the headings to become sections, got {:?}",
            parsed.sections.iter().map(|s| &s.heading).collect::<Vec<_>>()
        );
        assert_eq!(parsed.title.as_deref(), Some("Title"));
    }
}
