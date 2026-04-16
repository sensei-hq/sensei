//! Code file processor — uses language adapters (TypeScript, Python, Rust, etc.)

use super::types::*;
use crate::adapters;
use crate::indexer::graph::compute_complexity;
use crate::types::NodeKind;

/// Process a code file via the appropriate language adapter.
/// Returns None if no adapter exists for this extension.
pub fn process(abs_path: &str, rel_path: &str, ext: &str, content: &str, repo_id: &str) -> Option<FileProcessResult> {
    let dotted = format!(".{}", ext);
    let adapter = adapters::adapter_for_ext(&dotted)?;

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

    let unresolved_imports: Vec<String> = parsed.imports.iter()
        .map(|i| i.target_path.clone()).collect();

    let unresolved_calls: Vec<UnresolvedCall> = parsed.edges.iter()
        .map(|e| UnresolvedCall {
            caller_name: e.caller_name.clone(),
            callee_name: e.callee_name.clone(),
        }).collect();

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
    })
}
