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
                sym.signature.clone(), None, sym.docstring.clone(),
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
            type_node.docstring = sym.docstring.clone();
            graph.merge_node(&type_node).ok();
            graph.merge_edge(&result.file_id, &sym.id, "EXPORTS_TYPE").ok();
        }
    }

    // Write IR extension data (functions, classes) if available
    if let Some(ref ir) = result.ir {
        for module in &ir.modules {
            for func in &module.functions {
                graph.write_ir_function(func, repo_id, None).ok();
            }
        }
        for class in &ir.classes {
            graph.write_ir_class(class, repo_id).ok();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::graph::GraphDb;

    fn make_result(symbols: Vec<SymbolResult>) -> FileProcessResult {
        FileProcessResult {
            file_id: "file:/tmp/src/app.ts".into(),
            rel_path: "src/app.ts".into(),
            abs_path: "/tmp/src/app.ts".into(),
            kind: "file".into(),
            tags: "src".into(),
            language: Some("typescript".into()),
            doc_type: None,
            doc_category: None,
            title: Some("src/app".into()),
            symbols,
            unresolved_imports: vec![],
            unresolved_calls: vec![],
            parent_refs: vec![],
            file_refs: vec![],
            fn_mentions: vec![],
            ir: None,
        }
    }

    #[test]
    fn writes_function_docstring_to_graph() {
        let db = GraphDb::open_memory().unwrap();
        let result = make_result(vec![SymbolResult {
            id: "fn:/tmp/src/app.ts:handleRequest:10".into(),
            name: "handleRequest".into(),
            kind: "function".into(),
            line: 10,
            line_end: 30,
            signature: Some("(req: Request) => Response".into()),
            docstring: Some("Handle an incoming HTTP request".into()),
            is_exported: true,
            complexity: Some(5),
            parent: None,
        }]);

        write_to_graph(&db, &result, "proj", None).unwrap();

        // Verify docstring was stored
        let found = db.search_nodes("handleRequest", "proj", &["function"]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].docstring, Some("Handle an incoming HTTP request".into()));
    }

    #[test]
    fn writes_type_docstring_to_graph() {
        let db = GraphDb::open_memory().unwrap();
        let result = make_result(vec![SymbolResult {
            id: "type:/tmp/src/app.ts:AppConfig:1".into(),
            name: "AppConfig".into(),
            kind: "interface".into(),
            line: 1,
            line_end: 15,
            signature: None,
            docstring: Some("Application configuration".into()),
            is_exported: true,
            complexity: None,
            parent: None,
        }]);

        write_to_graph(&db, &result, "proj", None).unwrap();

        // Verify docstring was stored on type node
        let node: Option<String> = db.conn_ref().query_row(
            "SELECT docstring FROM hierarchy_nodes WHERE name='AppConfig'",
            [], |row| row.get(0),
        ).ok();
        assert_eq!(node, Some("Application configuration".into()));
    }

    #[test]
    fn writes_ir_functions_and_classes_to_graph() {
        let db = GraphDb::open_memory().unwrap();

        let ir = crate::ir::IRParsedFile {
            file_path: "src/app.ts".into(),
            language: "typescript".into(),
            modules: vec![crate::ir::IRModule {
                base: crate::ir::IRBase { name: "app".into(), file: "src/app.ts".into(), ..Default::default() },
                functions: vec![crate::ir::IRFunction {
                    base: crate::ir::IRBase { name: "main".into(), file: "src/app.ts".into(), line_start: 1, ..Default::default() },
                    params: vec![crate::ir::IRParam { name: "args".into(), type_: Some("string[]".into()), ..Default::default() }],
                    return_type: Some("void".into()),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            classes: vec![crate::ir::IRClass {
                base: crate::ir::IRBase { name: "AppService".into(), file: "src/app.ts".into(), line_start: 10, ..Default::default() },
                class_kind: crate::ir::ClassKind::Class,
                implements: vec!["Service".into()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let mut result = make_result(vec![]);
        result.ir = Some(ir);

        write_to_graph(&db, &result, "proj", None).unwrap();

        // Verify IR function was stored
        let ir_fn = db.read_ir_function("fn:proj:src/app.ts:main").unwrap();
        assert!(ir_fn.is_some(), "IR function should be stored");
        let ir_fn = ir_fn.unwrap();
        assert_eq!(ir_fn.params.len(), 1);
        assert_eq!(ir_fn.return_type, Some("void".into()));

        // Verify IR class was stored with IMPLEMENTS edge
        let ir_class = db.read_ir_class("class:proj:src/app.ts:AppService").unwrap();
        assert!(ir_class.is_some(), "IR class should be stored");
        let ir_class = ir_class.unwrap();
        assert_eq!(ir_class.implements, vec!["Service"]);

        // Verify IMPLEMENTS edge
        let count: i64 = db.conn_ref().query_row(
            "SELECT COUNT(*) FROM edges WHERE from_id='class:proj:src/app.ts:AppService' AND edge_type='IMPLEMENTS'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn ir_populated_from_real_code_processor() {
        // BAT: walk the full journey — process a real Rust file, verify IR flows through
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf();
        let rel = "crates/senseid/src/ir.rs";
        let abs = root.join(rel);
        let content = std::fs::read_to_string(&abs).expect("ir.rs should exist");
        let ext = "rs";

        let result = crate::tasks::processors::code::process(
            &abs.to_string_lossy(), rel, ext, &content, "sensei",
        ).expect("should process ir.rs");

        // IR should be populated
        assert!(result.ir.is_some(), "code processor should populate ir field");
        let ir = result.ir.as_ref().unwrap();

        // ir.rs defines structs like IRBase, IRDoc, IRFunction, IRClass
        assert!(!ir.classes.is_empty(), "ir.rs should have IR structs as classes");
        let class_names: Vec<&str> = ir.classes.iter().map(|c| c.base.name.as_str()).collect();
        assert!(class_names.contains(&"IRBase"), "should find IRBase struct, got {:?}", class_names);

        // Should also have module-level functions (none in ir.rs, but modules should exist)
        // The key assertion: IR data is present and correct
        assert_eq!(ir.language, "rust");

        // Now write to graph and verify enrichment
        let db = GraphDb::open_memory().unwrap();
        write_to_graph(&db, &result, "test", None).unwrap();

        // Verify ir_classes table was populated
        let ir_class_count: i64 = db.conn_ref().query_row(
            "SELECT COUNT(*) FROM ir_classes",
            [], |row| row.get(0),
        ).unwrap();
        assert!(ir_class_count > 0, "ir_classes should have entries after indexing ir.rs");
    }
}
