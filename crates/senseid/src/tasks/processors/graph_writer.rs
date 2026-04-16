//! Graph writer — writes FileProcessResult to the graph DB.
//! Separated from processing so processing can be tested without DB.

use super::types::*;
use crate::types::{NodeKind, HierarchyNode};
use crate::indexer::graph::GraphDb;

/// Write a FileProcessResult to the graph. Creates nodes, edges, unresolved refs.
pub fn write_to_graph(
    graph: &GraphDb,
    result: &FileProcessResult,
    repo_id: &str,
    module_id: Option<&str>,
) -> Result<(), String> {
    // Delete old nodes for this file (handles modify case)
    graph.delete_by_file(&result.abs_path, repo_id).ok();
    graph.clear_unresolved_refs_from(&result.file_id, repo_id).ok();

    // Write file node
    let node_kind = NodeKind::from_str(&result.kind);
    let mut file_node = HierarchyNode::group(
        result.file_id.clone(),
        result.title.clone().unwrap_or_else(|| result.rel_path.clone()),
        node_kind,
        repo_id.to_string(),
    );
    file_node.file = Some(result.abs_path.clone());
    file_node.tags = Some(result.tags.clone());
    file_node.level = result.language.clone();
    file_node.doc_type = result.doc_type.clone();
    file_node.doc_category = result.doc_category.clone();
    file_node.parent_id = module_id.map(|s| s.to_string());
    graph.merge_node(&file_node)?;

    // Wire file to module
    if let Some(mod_id) = module_id {
        graph.merge_edge(mod_id, &result.file_id, "CONTAINS_FILE").ok();
    }

    // For doc/extension: create category node + wire
    if result.kind == "doc" || result.kind == "extension" {
        if let Some(ref doc_type) = result.doc_type {
            let cat_id = format!("doc:{}:{}", repo_id, doc_type);
            let repo_node_id = format!("repo:{}", repo_id);
            let mut cat_node = HierarchyNode::group(
                cat_id.clone(),
                capitalize(doc_type),
                NodeKind::Doc,
                repo_id.to_string(),
            );
            cat_node.tags = Some("doc".into());
            cat_node.doc_type = Some(doc_type.clone());
            cat_node.parent_id = Some(repo_node_id.clone());
            graph.merge_node(&cat_node).ok();
            graph.merge_edge(&repo_node_id, &cat_id, "CONTAINS_DOC").ok();
            graph.merge_edge(&cat_id, &result.file_id, "CONTAINS_DOC").ok();

            // Update file parent to category
            file_node.parent_id = Some(cat_id);
            graph.merge_node(&file_node).ok();
        }
    }

    // Write symbol nodes
    for sym in &result.symbols {
        let sym_kind = NodeKind::from_str(&sym.kind);
        if sym_kind.is_function_like() {
            let node = HierarchyNode::function(
                sym.id.clone(), sym.name.clone(), sym_kind,
                result.abs_path.clone(), sym.line,
                sym.signature.clone(), None, None,
                sym.complexity.unwrap_or(1), repo_id.to_string(),
            );
            graph.merge_node(&node).ok();
            graph.merge_edge(&result.file_id, &sym.id, "EXPORTS_FN").ok();
            if let Some(mod_id) = module_id {
                graph.merge_edge(mod_id, &sym.id, "CONTAINS_FN").ok();
            }
        } else {
            let mut type_node = HierarchyNode::group(sym.id.clone(), sym.name.clone(), sym_kind, repo_id.to_string());
            type_node.file = Some(result.abs_path.clone());
            type_node.line = sym.line;
            graph.merge_node(&type_node).ok();
            graph.merge_edge(&result.file_id, &sym.id, "EXPORTS_TYPE").ok();
        }
    }

    // Store unresolved references
    for imp in &result.unresolved_imports {
        graph.add_unresolved_ref(&result.file_id, "imports", imp, repo_id).ok();
    }
    for call in &result.unresolved_calls {
        let caller_id = format!("fn:{}:{}:", result.abs_path, call.caller_name);
        graph.add_unresolved_ref(&caller_id, "calls", &call.callee_name, repo_id).ok();
    }
    for pref in &result.parent_refs {
        graph.add_unresolved_ref(&pref.method_id, "parent", &pref.parent_name, repo_id).ok();
    }
    for fref in &result.file_refs {
        graph.add_unresolved_ref(&result.file_id, "covers", fref, repo_id).ok();
    }
    for fname in &result.fn_mentions {
        graph.add_unresolved_ref(&result.file_id, "mentions_fn", fname, repo_id).ok();
    }

    Ok(())
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}
