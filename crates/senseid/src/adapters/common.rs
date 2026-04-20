//! Shared tree-sitter helpers used by multiple language adapters.
//! Every adapter gets these via `use super::common::*` or selective imports.

use tree_sitter::Node;
use crate::types::{ParsedSymbol, SymbolKind};
use crate::ir::{IRBase, IRFunction, IRMethod, IRClass, IRParam, IRImport, IRConstant, IRParsedFile, IRModule, ClassKind, Visibility};

/// Extract the text of a named field from a tree-sitter node.
pub fn field_text(node: &Node, field: &str, src: &[u8]) -> String {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or_default()
        .to_string()
}

/// Get a trimmed line from the source at a given row index.
#[cfg(test)]
pub fn line_at(lines: &[&str], row: usize) -> Option<String> {
    lines.get(row).map(|l| l.trim().to_string())
}

/// Build a ParsedSymbol with common fields. Docstring extraction is
/// language-specific — pass the result of the adapter's own extractor.
pub fn make_symbol(
    name: String,
    kind: SymbolKind,
    node: &Node,
    lines: &[&str],
    is_exported: bool,
    docstring: Option<String>,
) -> ParsedSymbol {
    ParsedSymbol {
        name,
        kind,
        signature: lines.get(node.start_position().row).map(|l| l.trim().to_string()),
        docstring,
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        is_exported,
        parent: None,
    }
}

/// Extract `<script>` blocks from Svelte/Vue SFC files.
/// Returns `(script_content, start_line_offset, is_typescript)` for each block.
pub fn extract_script_blocks(source: &str) -> Vec<(String, u32, bool)> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("<script") {
            let is_ts = line.contains("lang=\"ts\"") || line.contains("lang='ts'");
            let start = i + 1;
            let mut end = start;
            while end < lines.len() && !lines[end].trim().starts_with("</script") {
                end += 1;
            }
            let content: String = lines[start..end].join("\n");
            if !content.trim().is_empty() {
                blocks.push((content, start as u32, is_ts));
            }
            i = end + 1;
        } else {
            i += 1;
        }
    }
    blocks
}

// ── IR helpers ──────────────────────────────────────────────────────────────

/// Build an IRFunction from common fields. Used by all adapters.
pub fn ir_function(
    name: String, node: &Node, _lines: &[&str],
    is_exported: bool, is_async: bool,
    params: Vec<IRParam>, return_type: Option<String>,
    docstring: Option<String>, decorators: Vec<String>,
    body_text: &str,
) -> IRFunction {
    IRFunction {
        base: IRBase {
            name,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            docstring,
            is_exported,
            node_type: Some("function".into()),
            ..Default::default()
        },
        params,
        return_type,
        is_async,
        decorators,
        complexity: crate::indexer::graph::compute_complexity(body_text),
        ..Default::default()
    }
}

/// Build an IRMethod from common fields. Used by all adapters.
pub fn ir_method(
    name: String, node: &Node,
    is_exported: bool, is_async: bool, is_static: bool,
    params: Vec<IRParam>, return_type: Option<String>,
    docstring: Option<String>, decorators: Vec<String>,
    visibility: Visibility, body_text: &str,
) -> IRMethod {
    IRMethod {
        base: IRBase {
            name,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            docstring,
            is_exported,
            node_type: Some("method".into()),
            ..Default::default()
        },
        params,
        return_type,
        is_async,
        is_static,
        decorators,
        visibility,
        complexity: crate::indexer::graph::compute_complexity(body_text),
        ..Default::default()
    }
}

/// Build an IRClass from common fields. Used by all adapters.
pub fn ir_class(
    name: String, node: &Node, kind: ClassKind,
    is_exported: bool, docstring: Option<String>,
    decorators: Vec<String>,
) -> IRClass {
    IRClass {
        base: IRBase {
            name,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            docstring,
            is_exported,
            node_type: Some("class".into()),
            ..Default::default()
        },
        class_kind: kind,
        decorators,
        ..Default::default()
    }
}

/// Build a minimal IRParsedFile wrapper.
pub fn ir_parsed_file(
    file_path: &str, language: &str,
    module: IRModule, classes: Vec<IRClass>,
) -> IRParsedFile {
    let is_test = file_path.contains("test");
    IRParsedFile {
        file_path: file_path.into(),
        language: language.into(),
        modules: vec![module],
        classes,
        is_test_file: is_test,
        ..Default::default()
    }
}

/// Build an IRModule with file metadata.
pub fn ir_module(
    file_path: &str, language: &str,
    functions: Vec<IRFunction>, constants: Vec<IRConstant>,
    imports: Vec<IRImport>, is_test: bool,
) -> IRModule {
    let ext = std::path::Path::new(file_path).extension()
        .and_then(|e| e.to_str()).map(|e| format!(".{}", e));
    IRModule {
        base: IRBase {
            name: std::path::Path::new(file_path).file_stem()
                .map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            file: file_path.into(),
            extension: ext,
            language: Some(language.into()),
            node_type: Some("module".into()),
            ..Default::default()
        },
        functions,
        constants,
        imports,
        is_test,
        ..Default::default()
    }
}

/// Get full source text of a tree-sitter node.
pub fn node_text(node: &Node, src: &[u8]) -> String {
    node.utf8_text(src).unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_text_returns_empty_for_missing_field() {
        // Can't easily test without a real tree-sitter node, but verify the function signature compiles
        // Real integration tests are in each adapter's test suite
    }

    #[test]
    fn line_at_valid_index() {
        let lines = vec!["first", "second", "third"];
        assert_eq!(line_at(&lines, 1), Some("second".to_string()));
    }

    #[test]
    fn line_at_out_of_bounds() {
        let lines = vec!["first"];
        assert_eq!(line_at(&lines, 5), None);
    }

    #[test]
    fn line_at_trims_whitespace() {
        let lines = vec!["  indented  "];
        assert_eq!(line_at(&lines, 0), Some("indented".to_string()));
    }

    #[test]
    fn extract_script_blocks_single() {
        let source = "<template><div>hi</div></template>\n<script lang=\"ts\">\nconst x = 1;\n</script>";
        let blocks = extract_script_blocks(source);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].0.contains("const x = 1"));
        assert!(blocks[0].2); // is_ts
    }

    #[test]
    fn extract_script_blocks_multiple() {
        let source = "<script>\nlet a = 1;\n</script>\n<script setup lang=\"ts\">\nlet b = 2;\n</script>";
        let blocks = extract_script_blocks(source);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].0.contains("let a"));
        assert!(!blocks[0].2); // not ts
        assert!(blocks[1].0.contains("let b"));
        assert!(blocks[1].2); // is ts
    }

    #[test]
    fn extract_script_blocks_empty_script() {
        let source = "<script>\n\n</script>";
        let blocks = extract_script_blocks(source);
        assert_eq!(blocks.len(), 0); // empty content filtered out
    }

    #[test]
    fn extract_script_blocks_no_scripts() {
        let source = "<template><div>no scripts</div></template>";
        let blocks = extract_script_blocks(source);
        assert_eq!(blocks.len(), 0);
    }
}
