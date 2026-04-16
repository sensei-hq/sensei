//! Shared types for file processing results.

use serde::Serialize;

/// Result of processing a single file. No DB dependency — pure data.
#[derive(Debug, Clone, Serialize)]
pub struct FileProcessResult {
    pub file_id: String,
    pub rel_path: String,
    pub abs_path: String,
    pub kind: String,             // file, doc, extension
    pub tags: String,             // src, test, e2e, doc, config
    pub language: Option<String>,
    pub doc_type: Option<String>,
    pub doc_category: Option<String>,
    pub title: Option<String>,
    pub symbols: Vec<SymbolResult>,
    pub unresolved_imports: Vec<String>,
    pub unresolved_calls: Vec<UnresolvedCall>,
    pub parent_refs: Vec<ParentRef>,
    pub file_refs: Vec<String>,       // doc: backtick file references
    pub fn_mentions: Vec<String>,     // doc: backtick function mentions
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolResult {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub line: u32,
    pub signature: Option<String>,
    pub complexity: Option<u32>,
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnresolvedCall {
    pub caller_name: String,
    pub callee_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParentRef {
    pub method_id: String,
    pub parent_name: String,
}

impl FileProcessResult {
    /// Create a minimal result for files with no extractable content.
    pub fn minimal(file_id: String, rel_path: String, abs_path: String, kind: &str, tags: &str) -> Self {
        Self {
            file_id, rel_path, abs_path,
            kind: kind.into(), tags: tags.into(),
            language: None, doc_type: None, doc_category: None, title: None,
            symbols: vec![], unresolved_imports: vec![],
            unresolved_calls: vec![], parent_refs: vec![],
            file_refs: vec![], fn_mentions: vec![],
        }
    }
}

/// Classify a file as src, test, e2e, or config based on path patterns.
pub fn classify_file_tag(rel_path: &str, ext: &str) -> String {
    let lower = rel_path.to_lowercase();

    // Config files
    if matches!(ext, "json" | "toml" | "yaml" | "yml") {
        return "config".into();
    }

    // E2E tests
    if lower.contains("/e2e/") || lower.starts_with("e2e/") || lower.contains(".e2e.") || lower.contains("/e2e_") {
        return "e2e".into();
    }

    // Unit/integration tests
    // Check filename starts with test_ (Python convention)
    let filename = lower.rsplit('/').next().unwrap_or(&lower);
    if filename.starts_with("test_") {
        return "test".into();
    }

    if lower.contains(".spec.") || lower.contains(".test.")
        || lower.contains("_test.")
        || lower.contains("/test/") || lower.starts_with("test/")
        || lower.contains("/tests/") || lower.starts_with("tests/")
        || lower.contains("__tests__") || lower.contains("_spec.")
        || lower.contains("/fixtures/") || lower.starts_with("fixtures/")
        || lower.contains("/__mocks__/") || lower.starts_with("__mocks__/")
    {
        return "test".into();
    }

    "src".into()
}
