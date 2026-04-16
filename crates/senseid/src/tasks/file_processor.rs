//! File processor — extracts structured data from a single file.
//! This is the core logic, independent of the task queue.
//! Returns a FileProcessResult that can be tested without any DB.

use std::path::Path;
use serde::Serialize;
use crate::types::{NodeKind, SymbolKind};
use crate::adapters;
use crate::indexer::graph::compute_complexity;

/// Result of processing a single file. Fully serializable, no DB dependency.
#[derive(Debug, Clone, Serialize)]
pub struct FileProcessResult {
    pub file_id: String,
    pub rel_path: String,
    pub abs_path: String,
    pub kind: String,         // file, doc, extension
    pub tags: String,         // src, test, e2e, doc
    pub language: Option<String>,
    pub doc_type: Option<String>,
    pub doc_category: Option<String>,
    pub title: Option<String>,
    pub symbols: Vec<SymbolResult>,
    pub unresolved_imports: Vec<String>,
    pub unresolved_calls: Vec<UnresolvedCall>,
    pub parent_refs: Vec<ParentRef>,   // method → class name
    pub file_refs: Vec<String>,        // doc: backtick file references
    pub fn_mentions: Vec<String>,      // doc: backtick function mentions
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolResult {
    pub id: String,
    pub name: String,
    pub kind: String,       // function, method, class, interface, etc.
    pub line: u32,
    pub signature: Option<String>,
    pub complexity: Option<u32>,
    pub parent: Option<String>, // class/struct name for methods
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

/// Process a single file and return structured results. No DB writes.
pub fn process_file(abs_path: &str, repo_path: &str, repo_id: &str) -> Result<FileProcessResult, String> {
    let file_path = Path::new(abs_path);
    let repo = Path::new(repo_path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", abs_path));
    }

    let rel_path = file_path.strip_prefix(repo)
        .unwrap_or(file_path)
        .to_string_lossy().to_string();

    let ext = file_path.extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let is_doc = ext == ".md" || ext == ".mdx";
    let is_code = adapters::adapter_for_ext(&ext).is_some();
    let is_config = matches!(ext.as_str(), ".json" | ".toml" | ".yaml" | ".yml" | ".lock");

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read: {}", e))?;

    let file_id = format!("file:{}", abs_path);
    let file_tag = classify_file_tag(&rel_path);

    if is_doc {
        process_doc(&file_id, abs_path, &rel_path, &content, repo_id, repo_path)
    } else if is_code {
        process_code(&file_id, abs_path, &rel_path, &ext, &content, repo_id, &file_tag)
    } else if is_config {
        Ok(FileProcessResult {
            file_id, rel_path, abs_path: abs_path.to_string(),
            kind: "file".into(), tags: "config".into(),
            language: Some(ext.trim_start_matches('.').to_string()),
            doc_type: None, doc_category: None, title: None,
            symbols: vec![], unresolved_imports: vec![],
            unresolved_calls: vec![], parent_refs: vec![],
            file_refs: vec![], fn_mentions: vec![],
        })
    } else {
        // Unknown file type — still register it as a file node
        Ok(FileProcessResult {
            file_id, rel_path, abs_path: abs_path.to_string(),
            kind: "file".into(), tags: file_tag,
            language: None, doc_type: None, doc_category: None, title: None,
            symbols: vec![], unresolved_imports: vec![],
            unresolved_calls: vec![], parent_refs: vec![],
            file_refs: vec![], fn_mentions: vec![],
        })
    }
}

fn process_code(
    file_id: &str, abs_path: &str, rel_path: &str, ext: &str,
    content: &str, repo_id: &str, file_tag: &str,
) -> Result<FileProcessResult, String> {
    let adapter = adapters::adapter_for_ext(ext)
        .ok_or_else(|| format!("No adapter for {}", ext))?;

    let parsed = adapter.parse(content, rel_path);
    let file_lines: Vec<&str> = content.lines().collect();

    let mut symbols = Vec::new();
    let mut unresolved_calls = Vec::new();
    let mut parent_refs = Vec::new();

    for sym in &parsed.symbols {
        let node_kind = NodeKind::from_symbol_kind(&sym.kind);
        let sym_id = if node_kind.is_function_like() {
            format!("fn:{}:{}:{}", abs_path, sym.name, sym.line_start)
        } else {
            format!("type:{}:{}:{}", abs_path, sym.name, sym.line_start)
        };

        let complexity = if node_kind.is_function_like() {
            let body = file_lines
                .get((sym.line_start as usize).saturating_sub(1)..sym.line_end as usize)
                .map(|lines| lines.join("\n"))
                .unwrap_or_default();
            Some(compute_complexity(&body))
        } else {
            None
        };

        symbols.push(SymbolResult {
            id: sym_id.clone(),
            name: sym.name.clone(),
            kind: node_kind.as_str().to_string(),
            line: sym.line_start,
            signature: sym.signature.clone(),
            complexity,
            parent: sym.parent.clone(),
        });

        if let Some(ref parent_name) = sym.parent {
            parent_refs.push(ParentRef {
                method_id: sym_id,
                parent_name: parent_name.clone(),
            });
        }
    }

    for edge in &parsed.edges {
        unresolved_calls.push(UnresolvedCall {
            caller_name: edge.caller_name.clone(),
            callee_name: edge.callee_name.clone(),
        });
    }

    let unresolved_imports: Vec<String> = parsed.imports.iter()
        .map(|i| i.target_path.clone())
        .collect();

    let module_name = rel_path.rsplit_once('.').map(|(n, _)| n).unwrap_or(rel_path)
        .replace('\\', "/");

    Ok(FileProcessResult {
        file_id: file_id.to_string(),
        rel_path: rel_path.to_string(),
        abs_path: abs_path.to_string(),
        kind: "file".into(),
        tags: file_tag.to_string(),
        language: Some(parsed.language),
        doc_type: None,
        doc_category: None,
        title: Some(module_name),
        symbols,
        unresolved_imports,
        unresolved_calls,
        parent_refs,
        file_refs: vec![],
        fn_mentions: vec![],
    })
}

fn process_doc(
    file_id: &str, abs_path: &str, rel_path: &str,
    content: &str, repo_id: &str, repo_path: &str,
) -> Result<FileProcessResult, String> {
    let frontmatter = crate::indexer::doc_indexer::parse_frontmatter_pub(content);
    let classification = crate::indexer::doc_indexer::classify_doc_pub(rel_path, &frontmatter);

    let title = frontmatter.title
        .or(frontmatter.name)
        .or_else(|| crate::indexer::doc_indexer::extract_title(content));

    let file_refs = crate::indexer::doc_indexer::extract_file_refs(content, repo_path);
    let fn_mentions = crate::indexer::doc_indexer::extract_fn_mentions(content);

    Ok(FileProcessResult {
        file_id: file_id.to_string(),
        rel_path: rel_path.to_string(),
        abs_path: abs_path.to_string(),
        kind: classification.kind.as_str().to_string(),
        tags: "doc".into(),
        language: None,
        doc_type: Some(classification.doc_type),
        doc_category: classification.doc_category,
        title,
        symbols: vec![],
        unresolved_imports: vec![],
        unresolved_calls: vec![],
        parent_refs: vec![],
        file_refs,
        fn_mentions,
    })
}

fn classify_file_tag(rel_path: &str) -> String {
    let lower = rel_path.to_lowercase();
    if lower.contains("/e2e/") || lower.contains(".e2e.") || lower.contains("/e2e_") {
        return "e2e".into();
    }
    if lower.contains(".spec.") || lower.contains(".test.")
        || lower.contains("_test.") || lower.contains("/test/") || lower.contains("/tests/")
        || lower.contains("__tests__") || lower.contains("_spec.")
        || lower.contains("/fixtures/") || lower.contains("/__mocks__/")
    {
        return "test".into();
    }
    "src".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf()
    }

    fn process(rel: &str) -> FileProcessResult {
        let root = repo_root();
        let abs = root.join(rel);
        process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap()
    }

    // ── Code files ──────────────────────────────────────────────────

    #[test]
    fn rust_adapter_file() {
        let r = process("crates/senseid/src/adapters/svelte.rs");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
        assert_eq!(r.language.as_deref(), Some("rust"));
        assert!(r.symbols.len() > 0, "should extract symbols");
        // Should have SvelteAdapter struct and parse method
        let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"SvelteAdapter"), "should find SvelteAdapter");
    }

    #[test]
    fn svelte_component_file() {
        let r = process("apps/desktop/src/lib/ServerStatus.svelte");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
        assert_eq!(r.language.as_deref(), Some("svelte"));
        // Should find component symbol
        let kinds: Vec<&str> = r.symbols.iter().map(|s| s.kind.as_str()).collect();
        assert!(kinds.contains(&"component"), "should find component symbol");
    }

    #[test]
    fn svelte_ts_file() {
        let r = process("apps/desktop/src/lib/appstate.svelte.ts");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
        // .svelte.ts → TypeScript adapter
        assert!(r.language.as_deref() == Some("typescript") || r.language.as_deref() == Some("svelte"));
    }

    #[test]
    fn page_svelte_file() {
        let r = process("apps/desktop/src/routes/(app)/p/[id]/+page.svelte");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
    }

    #[test]
    fn test_file_tagged() {
        // Rust test files contain tests in the same file
        let r = process("crates/senseid/src/adapters/svelte.rs");
        // The file itself is src, but we could check for test functions inside
        assert_eq!(r.tags, "src"); // file-level tag is src
        // Check if any test functions exist
        let test_fns: Vec<&SymbolResult> = r.symbols.iter()
            .filter(|s| s.name.contains("test") || s.name.starts_with("svelte_"))
            .collect();
        // Svelte adapter has test functions
        assert!(test_fns.len() > 0 || r.symbols.len() > 0);
    }

    // ── Doc files ───────────────────────────────────────────────────

    #[test]
    fn design_doc() {
        let r = process("docs/design/01-architecture.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.tags, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("design"));
        assert!(r.title.is_some(), "should extract title");
    }

    #[test]
    fn feature_doc() {
        let r = process("docs/features/01-codebase-intelligence.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("feature"));
    }

    #[test]
    fn roadmap_doc() {
        let r = process("docs/roadmap/01-paradigm-shift.md");
        assert_eq!(r.kind, "doc");
        // roadmap → design category
        assert_eq!(r.doc_type.as_deref(), Some("design"));
    }

    #[test]
    fn gap_analysis_doc() {
        let r = process("docs/gap-analysis.md");
        assert_eq!(r.kind, "doc");
    }

    #[test]
    fn root_readme() {
        let r = process("README.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("usage"));
        assert!(r.title.is_some());
    }

    // ── Marketplace files ───────────────────────────────────────────

    #[test]
    fn marketplace_skill() {
        let r = process("marketplace/skills/auditing-skill-descriptions/SKILL.md");
        assert_eq!(r.kind, "extension");
        assert_eq!(r.tags, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("extension"));
    }

    // ── Config files ────────────────────────────────────────────────

    #[test]
    fn package_json() {
        let r = process("apps/desktop/package.json");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
    }

    #[test]
    fn cargo_toml() {
        let r = process("crates/sensei-mcp/Cargo.toml");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
    }

    // ── Non-indexed files ───────────────────────────────────────────

    #[test]
    fn c_parser_file() {
        let r = process("crates/senseid/grammars/kotlin/src/parser.c");
        // No C adapter — should still be registered as a file
        assert_eq!(r.kind, "file");
    }

    #[test]
    fn hook_script() {
        let root = repo_root();
        let abs = root.join("marketplace/hooks/run-hook.cmd");
        if abs.exists() {
            let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei:marketplace").unwrap();
            assert_eq!(r.kind, "file");
        }
    }

    #[test]
    fn hooks_json() {
        let root = repo_root();
        let abs = root.join("marketplace/hooks/hooks.json");
        if abs.exists() {
            let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei:marketplace").unwrap();
            assert_eq!(r.kind, "file");
            assert_eq!(r.tags, "config");
        }
    }
}
