//! Code file processor — uses language adapters (TypeScript, Python, Rust, etc.)

use super::types::*;
use crate::languages;
use crate::languages::compute_complexity;
use crate::types::NodeKind;

/// Process a code file via the appropriate language adapter.
/// Returns None if no adapter exists for this extension.
pub fn process(abs_path: &str, rel_path: &str, ext: &str, content: &str, _repo_id: &str) -> Option<FileProcessResult> {
    // Try compound extension first (e.g. .svelte.ts), then simple extension
    let filename = std::path::Path::new(abs_path).file_name()
        .and_then(|n| n.to_str()).unwrap_or("");
    let adapter = if let Some((a, _)) = languages::adapter_for_filename(filename) {
        a
    } else {
        let dotted = format!(".{}", ext);
        languages::adapter_for_ext(&dotted)?
    };

    let parsed = adapter.parse(content, rel_path);
    let file_lines: Vec<&str> = content.lines().collect();
    let file_id = format!("file:{}", abs_path);
    let file_tag = classify_file_tag(rel_path, ext);
    let module_name = rel_path.rsplit_once('.').map(|(n, _)| n).unwrap_or(rel_path)
        .replace('\\', "/");

    let mut symbols = Vec::new();
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
            line_end: sym.line_end,
            signature: sym.signature.clone(),
            docstring: sym.docstring.clone(),
            is_exported: sym.is_exported,
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

    let unresolved_imports: Vec<String> = parsed.imports.iter()
        .map(|i| i.target_path.clone()).collect();

    let unresolved_calls: Vec<UnresolvedCall> = parsed.edges.iter()
        .map(|e| UnresolvedCall {
            caller_name: e.caller_name.clone(),
            callee_name: e.callee_name.clone(),
        }).collect();

    // Parse IR (rich data) alongside the old path
    let ir = languages::parse_to_ir_for_filename(filename, content, rel_path);

    Some(FileProcessResult {
        file_id,
        rel_path: rel_path.to_string(),
        abs_path: abs_path.to_string(),
        kind: "file".into(),
        tags: file_tag,
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
        ir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf()
    }

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn process_code_file(rel: &str) -> FileProcessResult {
        let root = workspace_root();
        let abs = root.join(rel);
        let content = std::fs::read_to_string(&abs).expect(&format!("File not found: {}", abs.display()));
        let ext = abs.extension().and_then(|e| e.to_str()).unwrap_or("");
        process(&abs.to_string_lossy(), rel, ext, &content, "sensei").expect("should process")
    }

    fn process_fixture(rel: &str) -> FileProcessResult {
        let root = fixtures();
        let abs = root.join(rel);
        let content = std::fs::read_to_string(&abs).expect(&format!("Fixture not found: {}", abs.display()));
        let ext = abs.extension().and_then(|e| e.to_str()).unwrap_or("");
        process(&abs.to_string_lossy(), rel, ext, &content, "test").expect("should process")
    }

    #[test]
    fn rust_svelte_adapter() {
        let r = process_code_file("crates/senseid/src/languages/svelte.rs");
        assert_eq!(r.language.as_deref(), Some("rust"));
        assert_eq!(r.tags, "src");

        // Should find SvelteAdapter struct
        let struct_syms: Vec<_> = r.symbols.iter().filter(|s| s.name == "SvelteAdapter").collect();
        assert_eq!(struct_syms.len(), 1, "should find SvelteAdapter");
        assert_eq!(struct_syms[0].kind, "class"); // Rust struct → class kind

        // Should find parse method (impl LanguageAdapter)
        let methods: Vec<_> = r.symbols.iter().filter(|s| s.kind == "method").collect();
        assert!(methods.len() > 0, "should find methods");

        // Should have imports (use statements)
        assert!(r.unresolved_imports.len() > 0, "should have imports");
    }

    #[test]
    fn svelte_component_step_header() {
        let r = process_fixture("code/StepHeader.svelte");
        assert_eq!(r.language.as_deref(), Some("svelte"));

        let components: Vec<_> = r.symbols.iter().filter(|s| s.kind == "component").collect();
        assert_eq!(components.len(), 1, "should find exactly 1 component");
        assert_eq!(components[0].name, "StepHeader");
    }

    #[test]
    fn svelte_ts_appstate() {
        let r = process_fixture("code/appstate.svelte.ts");
        assert!(r.language.as_deref() == Some("typescript") || r.language.as_deref() == Some("javascript"));

        let fns: Vec<_> = r.symbols.iter().filter(|s| s.kind == "function").collect();
        assert!(!fns.is_empty(), "should find functions");
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"resetState"),
            "should find resetState, got {:?}", names);
    }

    #[test]
    fn page_svelte_route() {
        let r = process_fixture("code/+page.svelte");
        assert_eq!(r.language.as_deref(), Some("svelte"));

        let components: Vec<_> = r.symbols.iter().filter(|s| s.kind == "component").collect();
        assert_eq!(components.len(), 1);
    }

    #[test]
    fn rust_file_with_tests_and_src() {
        // Rust files contain both src and test code in the same file
        let r = process_code_file("crates/senseid/src/languages/svelte.rs");

        // Should have both regular functions AND test functions
        let all_fns: Vec<&str> = r.symbols.iter()
            .filter(|s| s.kind == "function" || s.kind == "method")
            .map(|s| s.name.as_str())
            .collect();

        // Rust adapter should find at least the top-level functions (language, parse)
        // Test functions inside `mod tests {}` may or may not be extracted depending on adapter depth
        assert!(all_fns.len() >= 2, "should find at least language and parse, got: {:?}", all_fns);
    }

    #[test]
    fn rust_methods_have_parent() {
        let r = process_code_file("crates/senseid/src/languages/svelte.rs");

        // Methods should have parent refs (from impl blocks)
        let methods_with_parent: Vec<_> = r.symbols.iter()
            .filter(|s| s.kind == "method" && s.parent.is_some())
            .collect();
        // SvelteAdapter impl has methods with parent
        if !methods_with_parent.is_empty() {
            assert!(methods_with_parent.iter().any(|m|
                m.parent.as_deref() == Some("SvelteAdapter") ||
                m.parent.as_deref() == Some("LanguageAdapter")),
                "methods should have SvelteAdapter parent, got {:?}",
                methods_with_parent.iter().map(|m| (&m.name, &m.parent)).collect::<Vec<_>>());
        }

        // parent_refs should match
        assert_eq!(r.parent_refs.len(), methods_with_parent.len());
    }

    #[test]
    fn c_file_with_adapter() {
        let root = workspace_root();
        let abs = root.join("crates/senseid/grammars/kotlin/src/parser.c");
        if !abs.exists() { return; }
        let content = std::fs::read_to_string(&abs).unwrap();
        let result = process(&abs.to_string_lossy(), "crates/senseid/grammars/kotlin/src/parser.c", "c", &content, "sensei");
        // Large generated file — C adapter skips files > 500K chars
        assert!(result.is_some(), "C adapter should handle .c files");
        let r = result.unwrap();
        assert_eq!(r.language.as_deref(), Some("c"));
        // parser.c is 678K lines — should be skipped by size threshold
        assert!(r.symbols.is_empty(), "generated parser.c should have no symbols (too large)");
    }
}

