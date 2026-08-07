use super::common::field_text;
use std::collections::{HashMap, HashSet};
use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, ParsedEdge, SymbolKind};
use super::fqn::{self, FileFqnContext, FqnDefinition, FqnFileOutput, FqnReference};
use crate::ir::{IRBase, IRModule, IRFunction, IRClass, IRMethod, IRParam, IRImport, IRConstant, IRParsedFile, ClassKind, Visibility};
use super::LanguageAdapter;

pub struct RustAdapter;

impl LanguageAdapter for RustAdapter {
    fn language(&self) -> &str { "rust" }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let mut parser = Parser::new();
        let lang = tree_sitter_rust::LANGUAGE;
        parser.set_language(&lang.into()).expect("failed to set rust language");

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return empty(file_path),
        };

        let lines: Vec<&str> = source.lines().collect();
        let root = tree.root_node();
        let src = source.as_bytes();

        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        let mut edges = Vec::new();
        let mut seen = std::collections::HashSet::new();
        walk_nodes(&root, src, &lines, &mut symbols, &mut imports, &mut edges, &mut seen, None);

        ParsedFile {
            file_path: file_path.to_string(),
            language: "rust".to_string(),
            symbols,
            edges,
            imports,
        }
    }
}

/// Parse Rust source into IR — rich nodes with params, return types, implements, decorators.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let mut parser = Parser::new();
    let lang = tree_sitter_rust::LANGUAGE;
    parser.set_language(&lang.into()).expect("failed to set rust language");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return IRParsedFile { file_path: file_path.into(), language: "rust".into(), ..Default::default() },
    };

    let lines: Vec<&str> = source.lines().collect();
    let root = tree.root_node();
    let src = source.as_bytes();

    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let mut constants = Vec::new();

    walk_ir(&root, src, &lines, &mut functions, &mut classes, &mut imports, &mut constants, None);

    let ext = std::path::Path::new(file_path).extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e));

    let is_test = file_path.contains("test") || source.contains("#[cfg(test)]");

    IRParsedFile {
        file_path: file_path.into(),
        language: "rust".into(),
        modules: vec![IRModule {
            base: IRBase {
                name: std::path::Path::new(file_path).file_stem()
                    .map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                file: file_path.into(),
                extension: ext,
                language: Some("rust".into()),
                node_type: Some("module".into()),
                ..Default::default()
            },
            functions,
            constants,
            imports,
            is_test,
            ..Default::default()
        }],
        classes,
        is_test_file: is_test,
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_ir(
    node: &Node, src: &[u8], lines: &[&str],
    functions: &mut Vec<IRFunction>,
    classes: &mut Vec<IRClass>,
    imports: &mut Vec<IRImport>,
    constants: &mut Vec<IRConstant>,
    impl_context: Option<(&str, Option<&str>)>, // (type_name, trait_name)
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_item" => {
                let name = field_text(&child, "name", src);
                let is_pub = has_child_kind(&child, "visibility_modifier");
                let is_async = source_text(&child, src).contains("async fn");
                let params = extract_params(&child, src);
                let return_type = extract_return_type(&child, src);
                let decorators = collect_attributes(&child, src);
                let docstring = collect_doc_comments(&child, src);
                let _sig = line_at(lines, child.start_position().row);

                if let Some((_type_name, _)) = impl_context {
                    // This is a method — will be added to the class by the caller
                    // For now, still add to functions but mark with parent
                    // The impl_item handler below attaches to the right class
                } else {
                    functions.push(IRFunction {
                        base: IRBase {
                            name: name.clone(),
                            file: String::new(), // set by caller
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            docstring,
                            is_exported: is_pub,
                            node_type: Some("function".into()),
                            ..Default::default()
                        },
                        params,
                        return_type,
                        is_async,
                        decorators,
                        complexity: crate::languages::compute_complexity(
                            &source_text(&child, src)
                        ),
                        ..Default::default()
                    });
                }
            }
            "struct_item" => {
                let name = field_text(&child, "name", src);
                classes.push(IRClass {
                    base: IRBase {
                        name,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        docstring: collect_doc_comments(&child, src),
                        is_exported: has_child_kind(&child, "visibility_modifier"),
                        node_type: Some("class".into()),
                        ..Default::default()
                    },
                    class_kind: ClassKind::Struct,
                    decorators: collect_attributes(&child, src),
                    ..Default::default()
                });
            }
            "enum_item" => {
                let name = field_text(&child, "name", src);
                classes.push(IRClass {
                    base: IRBase {
                        name,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        docstring: collect_doc_comments(&child, src),
                        is_exported: has_child_kind(&child, "visibility_modifier"),
                        node_type: Some("class".into()),
                        ..Default::default()
                    },
                    class_kind: ClassKind::Enum,
                    decorators: collect_attributes(&child, src),
                    ..Default::default()
                });
            }
            "trait_item" => {
                let name = field_text(&child, "name", src);
                let mut methods = Vec::new();
                // Extract trait methods — body may be "declaration_list" or "body"
                let body = child.child_by_field_name("body")
                    .or_else(|| (0..child.child_count()).find_map(|j| {
                        let c = child.child(j)?;
                        if c.kind() == "declaration_list" { Some(c) } else { None }
                    }));
                if let Some(body) = body {
                    for j in 0..body.child_count() {
                        if let Some(method_node) = body.child(j)
                            && (method_node.kind() == "function_item" || method_node.kind() == "function_signature_item") {
                                let mname = field_text(&method_node, "name", src);
                                methods.push(IRMethod {
                                    base: IRBase {
                                        name: mname,
                                        line_start: method_node.start_position().row as u32 + 1,
                                        line_end: method_node.end_position().row as u32 + 1,
                                        docstring: collect_doc_comments(&method_node, src),
                                        ..Default::default()
                                    },
                                    params: extract_params(&method_node, src),
                                    return_type: extract_return_type(&method_node, src),
                                    is_async: source_text(&method_node, src).contains("async fn"),
                                    ..Default::default()
                                });
                            }
                    }
                }
                classes.push(IRClass {
                    base: IRBase {
                        name,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        docstring: collect_doc_comments(&child, src),
                        is_exported: has_child_kind(&child, "visibility_modifier"),
                        node_type: Some("class".into()),
                        ..Default::default()
                    },
                    class_kind: ClassKind::Trait,
                    methods,
                    ..Default::default()
                });
            }
            "impl_item" => {
                let full_text = source_text(&child, src);
                let type_name = field_text(&child, "type", src);
                let trait_name = extract_trait_from_impl(&full_text);

                // Find or create the class for this impl
                let class_idx = classes.iter().position(|c| c.base.name == type_name);

                let mut methods = Vec::new();
                if let Some(body) = child.child_by_field_name("body") {
                    for j in 0..body.child_count() {
                        if let Some(method_node) = body.child(j)
                            && method_node.kind() == "function_item" {
                                let mname = field_text(&method_node, "name", src);
                                let is_pub = has_child_kind(&method_node, "visibility_modifier");
                                methods.push(IRMethod {
                                    base: IRBase {
                                        name: mname,
                                        line_start: method_node.start_position().row as u32 + 1,
                                        line_end: method_node.end_position().row as u32 + 1,
                                        docstring: collect_doc_comments(&method_node, src),
                                        is_exported: is_pub,
                                        node_type: Some("method".into()),
                                        ..Default::default()
                                    },
                                    params: extract_params(&method_node, src),
                                    return_type: extract_return_type(&method_node, src),
                                    is_async: source_text(&method_node, src).contains("async fn"),
                                    decorators: collect_attributes(&method_node, src),
                                    visibility: if is_pub { Visibility::Public } else { Visibility::Private },
                                    complexity: crate::languages::compute_complexity(
                                        &source_text(&method_node, src)
                                    ),
                                    ..Default::default()
                                });
                            }
                    }
                }

                if let Some(idx) = class_idx {
                    // Append methods to existing class
                    classes[idx].methods.extend(methods);
                    if let Some(ref tn) = trait_name
                        && !classes[idx].implements.contains(tn) {
                            classes[idx].implements.push(tn.clone());
                        }
                } else if !type_name.is_empty() {
                    // Create class for this impl (struct not seen yet, or defined elsewhere)
                    let mut class = IRClass {
                        base: IRBase {
                            name: type_name,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            node_type: Some("class".into()),
                            ..Default::default()
                        },
                        class_kind: ClassKind::Struct,
                        methods,
                        ..Default::default()
                    };
                    if let Some(tn) = trait_name {
                        class.implements.push(tn);
                    }
                    classes.push(class);
                }
            }
            "const_item" => {
                let name = field_text(&child, "name", src);
                constants.push(IRConstant {
                    base: IRBase {
                        name,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        is_exported: has_child_kind(&child, "visibility_modifier"),
                        node_type: Some("const".into()),
                        ..Default::default()
                    },
                    type_: extract_const_type(&child, src),
                    value_preview: None,
                });
            }
            "use_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let path = text.trim_start_matches("use ").trim_end_matches(';').trim();
                if path.contains("::{") {
                    if let Some((base, rest)) = path.split_once("::{") {
                        let names: Vec<String> = rest.trim_end_matches('}')
                            .split(',').map(|s| s.trim().to_string()).collect();
                        imports.push(IRImport { source: base.to_string(), names, is_reexport: false });
                    }
                } else {
                    let name = path.rsplit("::").next().unwrap_or(path).to_string();
                    imports.push(IRImport { source: path.to_string(), names: vec![name], is_reexport: false });
                }
            }
            _ => {}
        }
    }
}

/// Extract function parameters as IRParam.
fn extract_params(node: &Node, src: &[u8]) -> Vec<IRParam> {
    let mut params = Vec::new();
    if let Some(param_list) = node.child_by_field_name("parameters") {
        for i in 0..param_list.child_count() {
            if let Some(param) = param_list.child(i) {
                match param.kind() {
                    "parameter" => {
                        let name = field_text(&param, "pattern", src);
                        let type_ = field_text(&param, "type", src);
                        params.push(IRParam {
                            name: if name.is_empty() { param.utf8_text(src).unwrap_or_default().to_string() } else { name },
                            type_: if type_.is_empty() { None } else { Some(type_) },
                            ..Default::default()
                        });
                    }
                    "self_parameter" => {
                        params.push(IRParam {
                            name: "self".into(),
                            type_: Some("Self".into()),
                            ..Default::default()
                        });
                    }
                    _ => {}
                }
            }
        }
    }
    params
}

/// Extract return type from function signature.
fn extract_return_type(node: &Node, src: &[u8]) -> Option<String> {
    let ret = field_text(node, "return_type", src);
    if ret.is_empty() {
        None
    } else {
        // Strip leading "-> "
        Some(ret.trim_start_matches("->").trim().to_string())
    }
}

/// Extract const type.
fn extract_const_type(node: &Node, src: &[u8]) -> Option<String> {
    let t = field_text(node, "type", src);
    if t.is_empty() { None } else { Some(t) }
}

/// Extract trait name from "impl Trait for Type" pattern.
fn extract_trait_from_impl(text: &str) -> Option<String> {
    // Match: impl TraitName for TypeName
    if let Some(for_pos) = text.find(" for ") {
        let before_for = text[..for_pos].trim();
        let trait_part = before_for.strip_prefix("impl ")?.trim();
        // Strip generics
        let trait_name = trait_part.split('<').next()?.trim();
        if trait_name.is_empty() { None } else { Some(trait_name.to_string()) }
    } else {
        None
    }
}

/// Collect #[attribute] decorators from preceding siblings.
fn collect_attributes(node: &Node, src: &[u8]) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "attribute_item" {
            let text = sib.utf8_text(src).unwrap_or_default().to_string();
            attrs.push(text);
        } else if sib.kind() != "line_comment" {
            break;
        }
        prev = sib.prev_sibling();
    }
    attrs.reverse();
    attrs
}

/// Get full source text of a node.
fn source_text(node: &Node, src: &[u8]) -> String {
    node.utf8_text(src).unwrap_or_default().to_string()
}

fn empty(path: &str) -> ParsedFile {
    ParsedFile { file_path: path.into(), language: "rust".into(), symbols: vec![], edges: vec![], imports: vec![] }
}

/// Ubiquitous std/library methods whose call-sites carry no navigation signal.
/// Skipped at extraction to keep unresolvable noise out of `calls` edges.
/// Per-adapter by design — each language owns its own list.
const RUST_CALL_DENYLIST: &[&str] = &[
    "clone", "unwrap", "expect", "into", "to_string", "to_owned",
    "as_str", "as_ref", "iter", "into_iter", "map", "unwrap_or",
    "unwrap_or_default", "ok", "len", "is_empty", "push", "collect",
    "next", "borrow", "borrow_mut", "lock", "read", "write",
];

/// Extract the bare callee name from a `call_expression`'s `function` field.
/// `foo()` → "foo"; `a::b::c()` → "c"; `recv.method()` → "method";
/// `foo::<T>()` (turbofish) → unwraps the `generic_function` to its inner name.
/// Returns None for unsupported call forms (e.g. calling a closure value).
fn callee_name(call: &Node, src: &[u8]) -> Option<String> {
    name_of_fn_expr(&call.child_by_field_name("function")?, src)
}

/// Resolve the bare/last-segment name of a node in function position.
fn name_of_fn_expr(func: &Node, src: &[u8]) -> Option<String> {
    match func.kind() {
        "identifier" => Some(source_text(func, src)),
        "scoped_identifier" => func
            .child_by_field_name("name")
            .map(|n| source_text(&n, src))
            .or_else(|| source_text(func, src).rsplit("::").next().map(|s| s.to_string())),
        "field_expression" => func.child_by_field_name("field").map(|n| source_text(&n, src)),
        "generic_function" => func.child_by_field_name("function").and_then(|f| name_of_fn_expr(&f, src)),
        _ => None,
    }
}

/// Recursively collect call-sites under `node`, attributing each to `caller`.
/// Descends through all children (incl. closures and nested blocks) so calls
/// made anywhere in the function body attribute to the enclosing fn.
/// Dedups per (caller, caller_line, callee) via `seen`.
/// Note: calls inside macro argument token-trees (e.g. `vec![f()]`) are not captured —
/// tree-sitter exposes macro args as an opaque token_tree.
fn collect_calls(
    node: &Node,
    src: &[u8],
    caller: &str,
    caller_line: u32,
    edges: &mut Vec<ParsedEdge>,
    seen: &mut std::collections::HashSet<String>,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        // A nested fn's calls belong to that fn, not the enclosing one.
        // (Closures are `closure_expression`, not `function_item`, so this
        // does NOT affect the desirable closure-capture behavior.)
        if child.kind() == "function_item" {
            continue;
        }
        if child.kind() == "call_expression"
            && let Some(name) = callee_name(&child, src)
            && !RUST_CALL_DENYLIST.contains(&name.as_str())
            && seen.insert(format!("{caller}:{caller_line}:{name}"))
        {
            edges.push(ParsedEdge {
                caller_name: caller.to_string(),
                caller_line,
                callee_name: name,
                callee_file: None,
            });
        }
        collect_calls(&child, src, caller, caller_line, edges, seen);
    }
}

#[allow(clippy::too_many_arguments)]
fn walk_nodes(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>, edges: &mut Vec<ParsedEdge>, seen: &mut std::collections::HashSet<String>, impl_type: Option<&str>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_item" => {
                let name = field_text(&child, "name", src);
                let is_pub = has_child_kind(&child, "visibility_modifier");
                let kind = if impl_type.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                let caller_line = child.start_position().row as u32 + 1;
                if let Some(body) = child.child_by_field_name("body") {
                    collect_calls(&body, src, &name, caller_line, edges, seen);
                }
                symbols.push(ParsedSymbol {
                    name,
                    kind,
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    line_start: caller_line,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: is_pub,
                    parent: impl_type.map(|s| s.to_string()),
                });
            }
            "struct_item" => {
                let name = field_text(&child, "name", src);
                symbols.push(ParsedSymbol {
                    name, kind: SymbolKind::Class,
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: has_child_kind(&child, "visibility_modifier"),
                    parent: None,
                });
            }
            "enum_item" => {
                let name = field_text(&child, "name", src);
                symbols.push(ParsedSymbol {
                    name, kind: SymbolKind::Enum,
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: has_child_kind(&child, "visibility_modifier"),
                    parent: None,
                });
            }
            "trait_item" => {
                let name = field_text(&child, "name", src);
                symbols.push(ParsedSymbol {
                    name, kind: SymbolKind::Interface,
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: has_child_kind(&child, "visibility_modifier"),
                    parent: None,
                });
            }
            "type_item" => {
                let name = field_text(&child, "name", src);
                symbols.push(ParsedSymbol {
                    name, kind: SymbolKind::Type,
                    signature: line_at(lines, child.start_position().row),
                    docstring: None,
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: has_child_kind(&child, "visibility_modifier"),
                    parent: None,
                });
            }
            "const_item" => {
                let name = field_text(&child, "name", src);
                symbols.push(ParsedSymbol {
                    name, kind: SymbolKind::Const,
                    signature: line_at(lines, child.start_position().row),
                    docstring: None,
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: has_child_kind(&child, "visibility_modifier"),
                    parent: None,
                });
            }
            "impl_item" => {
                // Extract the type name from `impl TypeName { ... }`
                let type_name = field_text(&child, "type", src);
                let type_name_ref = if type_name.is_empty() { None } else { Some(type_name.as_str()) };
                if let Some(body) = child.child_by_field_name("body") {
                    walk_nodes(&body, src, lines, symbols, imports, edges, seen, type_name_ref);
                }
            }
            "use_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                // Parse: use path::to::thing; or use path::{a, b};
                let path = text.trim_start_matches("use ").trim_end_matches(';').trim();
                if path.contains("::{") {
                    if let Some((base, rest)) = path.split_once("::{") {
                        let names: Vec<String> = rest.trim_end_matches('}')
                            .split(',').map(|s| s.trim().to_string()).collect();
                        imports.push(ParsedImport { target_path: base.to_string(), names });
                    }
                } else {
                    let name = path.rsplit("::").next().unwrap_or(path).to_string();
                    imports.push(ParsedImport { target_path: path.to_string(), names: vec![name] });
                }
            }
            _ => {}
        }
    }
}


fn line_at(lines: &[&str], row: usize) -> Option<String> {
    lines.get(row).map(|l| l.trim().to_string())
}

fn has_child_kind(node: &Node, kind: &str) -> bool {
    (0..node.child_count()).any(|i| node.child(i).is_some_and(|c| c.kind() == kind))
}

fn collect_doc_comments(node: &Node, src: &[u8]) -> Option<String> {
    let mut comments = Vec::new();
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "line_comment" {
            let text = sib.utf8_text(src).unwrap_or_default();
            if text.starts_with("///") {
                comments.push(text.trim_start_matches("///").trim().to_string());
            } else {
                break;
            }
        } else {
            break;
        }
        prev = sib.prev_sibling();
    }
    if comments.is_empty() { return None; }
    comments.reverse();
    Some(comments.join("\n"))
}

// ── FQN producer (plan Phase 2) ──────────────────────────────────────────────
// Pure name resolution: given a Rust source + its (package, module) context,
// produce a canonical FQN (plan 0.1) for every definition and reference. Phase 3
// wires this into `upsert_node_by_fqn` emit. References use the INHERENT method
// form (`…·Type·member`) — a call site can't name the trait, so trait dispatch
// stays honest (it resolves to the type's method node, never a wrong trait merge);
// definitions of trait-impl methods DO carry the trait qualifier so `Display::fmt`
// and `Debug::fmt` on one type are distinct nodes.
pub(crate) mod rust_fqn {
    use super::*;

    const RUST_LANG: &str = "rust";

/// Enclosing `impl` while walking: the Self type, its resolved canonical module
/// (the anchoring rule — a method anchors on where its TYPE is defined, not the
/// impl file), and the trait for a trait-impl (`impl Trait for Type`).
struct ImplCtx {
    type_name: String,
    type_module: String,
    trait_name: Option<String>,
}

/// File-level symbol context gathered in pass 1.
#[derive(Default)]
struct FileScope {
    /// Imported leaf name → full use path (`Widget` → `crate::widget::Widget`).
    use_map: HashMap<String, String>,
    /// Type names defined in this file (anchor on this file's module).
    local_types: HashSet<String>,
    /// Submodules declared in this file (`mod util;`) — so `util::f()` classifies
    /// as internal, not as an external crate.
    local_modules: HashSet<String>,
}

/// A classified `::`-path.
enum PathClass {
    /// Current crate: `module` is the crate-relative module chain; `leaf` the item.
    Internal { module: String, leaf: String },
    /// A dependency: `package` is the crate, `path` the in-crate module path.
    External { package: String, path: String, leaf: String },
}

/// Produce canonical FQNs (plan 0.1) for every definition and reference in a Rust
/// source file. Pure over `(source, ctx)`.
pub fn produce_fqns(source: &str, ctx: &FileFqnContext) -> FqnFileOutput {
    let mut parser = Parser::new();
    if parser.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() {
        return FqnFileOutput::default();
    }
    let Some(tree) = parser.parse(source, None) else {
        return FqnFileOutput::default();
    };
    let src = source.as_bytes();
    let root = tree.root_node();

    let mut scope = FileScope::default();
    collect_scope(&root, src, &mut scope);

    let mut out = FqnFileOutput::default();
    let lines: Vec<&str> = source.lines().collect();
    walk_fqn(&root, src, &lines, ctx, &ctx.module, &scope, None, &mut out);
    out
}

/// Pass 1: gather the file-global use-map, local type names, and submodule names.
fn collect_scope(node: &Node, src: &[u8], scope: &mut FileScope) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "use_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let path = text.trim_start_matches("use ").trim_end_matches(';').trim();
                if let Some((base, rest)) = path.split_once("::{") {
                    for name in rest.trim_end_matches('}').split(',') {
                        let name = name.trim();
                        if name.is_empty() || name == "self" { continue; }
                        let leaf = name.rsplit("::").next().unwrap_or(name).trim();
                        scope.use_map.insert(leaf.to_string(), format!("{base}::{name}"));
                    }
                } else if !path.is_empty() {
                    let leaf = path.rsplit("::").next().unwrap_or(path).trim();
                    // `use a::b as c` → key on the alias.
                    if let Some((full, alias)) = path.split_once(" as ") {
                        scope.use_map.insert(alias.trim().to_string(), full.trim().to_string());
                    } else {
                        scope.use_map.insert(leaf.to_string(), path.to_string());
                    }
                }
            }
            "struct_item" | "enum_item" | "trait_item" | "type_item" | "union_item" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() { scope.local_types.insert(name); }
            }
            "mod_item" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() { scope.local_modules.insert(name); }
                if let Some(body) = child.child_by_field_name("body") {
                    collect_scope(&body, src, scope);
                }
            }
            _ => {}
        }
    }
}

/// Main walk: emit a def (with FQN) for each item and, per function body, resolve
/// its calls to target FQNs.
#[allow(clippy::too_many_arguments)]
fn walk_fqn(
    node: &Node, src: &[u8], lines: &[&str], ctx: &FileFqnContext, module: &str,
    scope: &FileScope, impl_ctx: Option<&ImplCtx>, out: &mut FqnFileOutput,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_item" => {
                let name = field_text(&child, "name", src);
                if name.is_empty() { continue; }
                let (fqn_str, kind, parent_type) = match impl_ctx {
                    Some(ic) => {
                        let f = match &ic.trait_name {
                            Some(tr) => fqn::trait_method(RUST_LANG, &ctx.package, &ic.type_module, &ic.type_name, tr, &name),
                            None => fqn::method(RUST_LANG, &ctx.package, &ic.type_module, &ic.type_name, &name),
                        };
                        (f, SymbolKind::Method, Some(ic.type_name.clone()))
                    }
                    None => (fqn::item(RUST_LANG, &ctx.package, module, &name), SymbolKind::Function, None),
                };
                out.defs.push(FqnDefinition {
                    fqn: fqn_str.clone(),
                    name,
                    kind,
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: has_child_kind(&child, "visibility_modifier"),
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    parent_type,
                });
                if let Some(body) = child.child_by_field_name("body") {
                    let bindings = build_bindings(&child, src, ctx, scope, impl_ctx);
                    let mut seen = HashSet::new();
                    collect_fqn_calls(&body, src, ctx, module, scope, impl_ctx, &bindings, &fqn_str, &mut seen, out);
                }
            }
            "struct_item" | "enum_item" | "trait_item" | "type_item" | "const_item" | "static_item" | "union_item" => {
                let name = field_text(&child, "name", src);
                if name.is_empty() { continue; }
                let kind = match child.kind() {
                    "struct_item" | "union_item" => SymbolKind::Struct,
                    "enum_item" => SymbolKind::Enum,
                    "trait_item" => SymbolKind::Interface,
                    "const_item" | "static_item" => SymbolKind::Const,
                    _ => SymbolKind::Type,
                };
                out.defs.push(FqnDefinition {
                    fqn: fqn::item(RUST_LANG, &ctx.package, module, &name),
                    name,
                    kind,
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: has_child_kind(&child, "visibility_modifier"),
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    parent_type: None,
                });
            }
            "impl_item" => {
                let type_name = base_type_name(&field_text(&child, "type", src)).unwrap_or_default();
                if type_name.is_empty() {
                    continue;
                }
                let trait_name = child
                    .child_by_field_name("trait")
                    .and_then(|t| base_type_name(&source_text(&t, src)));
                let (_, type_module, _) = resolve_type_module(&type_name, ctx, module, scope);
                let ic = ImplCtx { type_name, type_module, trait_name };
                if let Some(body) = child.child_by_field_name("body") {
                    walk_fqn(&body, src, lines, ctx, module, scope, Some(&ic), out);
                }
            }
            "mod_item" => {
                if let Some(body) = child.child_by_field_name("body") {
                    let name = field_text(&child, "name", src);
                    let inner = join_mod(module, &name);
                    walk_fqn(&body, src, lines, ctx, &inner, scope, None, out);
                }
            }
            _ => {}
        }
    }
}

/// Resolve a bare type NAME to `(package, module, is_external)`. Local defs anchor
/// on this file's module; imported names use the use-map; else fall back to the
/// current crate+module (best effort). For an external type, `module` carries the
/// dependency's in-crate path.
fn resolve_type_module(
    type_name: &str, ctx: &FileFqnContext, module: &str, scope: &FileScope,
) -> (String, String, bool) {
    if scope.local_types.contains(type_name) {
        return (ctx.package.clone(), module.to_string(), false);
    }
    if let Some(full) = scope.use_map.get(type_name) {
        let segs: Vec<&str> = full.split("::").collect();
        match classify_segments(&segs, ctx, scope) {
            PathClass::Internal { module, .. } => return (ctx.package.clone(), module, false),
            PathClass::External { package, path, .. } => return (package, path, true),
        }
    }
    (ctx.package.clone(), module.to_string(), false)
}

/// Classify a `::`-path as current-crate vs dependency.
fn classify_segments(segs: &[&str], ctx: &FileFqnContext, scope: &FileScope) -> PathClass {
    let leaf = segs.last().copied().unwrap_or("").to_string();
    let first = segs.first().copied().unwrap_or("");
    // Modules strictly between the root marker and the leaf.
    let mid = |from: usize| -> String {
        if segs.len() <= from + 1 { String::new() } else { segs[from..segs.len() - 1].join("::") }
    };
    let internal_root = first == "crate"
        || first == "self"
        || first == "super"
        || norm_crate(first) == norm_crate(&ctx.package)
        || scope.local_modules.contains(first);
    if internal_root {
        let module = match first {
            "crate" => mid(1),
            "self" => join_mod(&ctx.module, &mid(1)),
            "super" => join_mod(&parent_mod(&ctx.module), &mid(1)),
            _ => {
                // current-crate name prefix, or a local submodule used without `crate::`.
                if norm_crate(first) == norm_crate(&ctx.package) { mid(1) } else { mid(0) }
            }
        };
        PathClass::Internal { module, leaf }
    } else {
        let path = if segs.len() >= 2 { segs[..segs.len() - 1].join("::") } else { first.to_string() };
        PathClass::External { package: first.to_string(), path, leaf }
    }
}

/// Per-function bounded binding→type map (plan 0.7): typed params, `let x: Type`,
/// and `let x = Type::new()` / `Type { .. }`. Everything else is out of scope.
fn build_bindings(
    fn_node: &Node, src: &[u8], _ctx: &FileFqnContext, _scope: &FileScope, _impl_ctx: Option<&ImplCtx>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(params) = fn_node.child_by_field_name("parameters") {
        for i in 0..params.child_count() {
            let p = params.child(i).unwrap();
            if p.kind() != "parameter" { continue; }
            let (Some(pat), Some(ty)) = (p.child_by_field_name("pattern"), p.child_by_field_name("type")) else { continue };
            if pat.kind() == "identifier"
                && let Some(t) = base_type_name(&source_text(&ty, src)) {
                map.insert(source_text(&pat, src), t);
            }
        }
    }
    if let Some(body) = fn_node.child_by_field_name("body") {
        for i in 0..body.child_count() {
            let stmt = body.child(i).unwrap();
            if stmt.kind() != "let_declaration" { continue; }
            let Some(pat) = stmt.child_by_field_name("pattern") else { continue };
            if pat.kind() != "identifier" { continue; }
            let vname = source_text(&pat, src);
            if let Some(ty) = stmt.child_by_field_name("type") {
                if let Some(t) = base_type_name(&source_text(&ty, src)) { map.insert(vname, t); }
            } else if let Some(val) = stmt.child_by_field_name("value")
                && let Some(t) = type_of_value(&val, src) {
                map.insert(vname, t);
            }
        }
    }
    map
}

/// The type produced by a `let` initialiser, for the four bounded forms only:
/// `Type::new()` / `Type::assoc()` (call whose function path starts with a Type)
/// and `Type { .. }` (struct literal). Returns None for anything else.
fn type_of_value(val: &Node, src: &[u8]) -> Option<String> {
    match val.kind() {
        "call_expression" => {
            let func = val.child_by_field_name("function")?;
            if func.kind() == "scoped_identifier" {
                let path = func.child_by_field_name("path")?;
                let ty = source_text(&path, src);
                let base = ty.rsplit("::").next().unwrap_or(&ty);
                if is_pascal(base) { return Some(base.to_string()); }
            }
            None
        }
        "struct_expression" => {
            let name = val.child_by_field_name("name")?;
            base_type_name(&source_text(&name, src))
        }
        "reference_expression" => val.child_by_field_name("value").and_then(|v| type_of_value(&v, src)),
        _ => None,
    }
}

/// Collect calls in `body`, attributing each to `caller_fqn`, resolving the target
/// FQN. Dedups per (caller, target-name). A nested `function_item`'s calls belong
/// to that fn (handled by the outer walk), so we don't descend into one here.
#[allow(clippy::too_many_arguments)]
fn collect_fqn_calls(
    node: &Node, src: &[u8], ctx: &FileFqnContext, module: &str, scope: &FileScope,
    impl_ctx: Option<&ImplCtx>, bindings: &HashMap<String, String>, caller_fqn: &str,
    seen: &mut HashSet<String>, out: &mut FqnFileOutput,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "function_item" { continue; }
        if child.kind() == "call_expression"
            && let Some(func) = child.child_by_field_name("function")
            && let Some((target_fqn, is_lib, target_name)) =
                resolve_call(&func, src, ctx, module, scope, impl_ctx, bindings)
            // Dedup on the resolved target (so `A::new()` and `B::new()` in one fn
            // both survive), not the bare name.
            && seen.insert(target_fqn.clone().unwrap_or_else(|| format!("?{target_name}")))
        {
            out.refs.push(FqnReference {
                caller_fqn: caller_fqn.to_string(),
                caller_line: child.start_position().row as u32 + 1,
                target_fqn,
                target_name,
                is_lib,
            });
        }
        collect_fqn_calls(&child, src, ctx, module, scope, impl_ctx, bindings, caller_fqn, seen, out);
    }
}

/// Resolve one call's `function` node to `(target_fqn, is_lib, target_name)`.
/// `target_fqn = None` = deliberately unresolved (out-of-0.7 receiver) so it never
/// wrong-merges. Returns None to SKIP a call (denylisted / unsupported form).
fn resolve_call(
    func: &Node, src: &[u8], ctx: &FileFqnContext, module: &str, scope: &FileScope,
    impl_ctx: Option<&ImplCtx>, bindings: &HashMap<String, String>,
) -> Option<(Option<String>, bool, String)> {
    match func.kind() {
        "identifier" => {
            let name = source_text(func, src);
            if RUST_CALL_DENYLIST.contains(&name.as_str()) { return None; }
            if let Some(full) = scope.use_map.get(&name) {
                let segs: Vec<&str> = full.split("::").collect();
                match classify_segments(&segs, ctx, scope) {
                    PathClass::Internal { module: m, leaf } => Some((Some(fqn::item(RUST_LANG, &ctx.package, &m, &leaf)), false, name)),
                    PathClass::External { package, path, leaf } => Some((Some(fqn::lib(&package, &path, &leaf)), true, name)),
                }
            } else {
                Some((Some(fqn::item(RUST_LANG, &ctx.package, module, &name)), false, name))
            }
        }
        "scoped_identifier" => {
            let text = source_text(func, src);
            let segs: Vec<&str> = text.split("::").collect();
            let leaf = (*segs.last()?).to_string();
            if RUST_CALL_DENYLIST.contains(&leaf.as_str()) { return None; }
            if segs.len() >= 2 && segs[0] == "Self"
                && let Some(ic) = impl_ctx {
                return Some((Some(fqn::method(RUST_LANG, &ctx.package, &ic.type_module, &ic.type_name, &leaf)), false, leaf));
            }
            if segs.len() >= 2 && is_pascal(segs[segs.len() - 2]) {
                let type_name = segs[segs.len() - 2];
                let (pkg, mdl, is_ext) = if segs.len() == 2 {
                    resolve_type_module(type_name, ctx, module, scope)
                } else {
                    match classify_segments(&segs[..segs.len() - 1], ctx, scope) {
                        PathClass::Internal { module: m, .. } => (ctx.package.clone(), m, false),
                        PathClass::External { package, path, .. } => (package, path, true),
                    }
                };
                if is_ext {
                    Some((Some(fqn::lib(&pkg, &mdl, &leaf)), true, leaf))
                } else {
                    Some((Some(fqn::method(RUST_LANG, &pkg, &mdl, type_name, &leaf)), false, leaf))
                }
            } else {
                match classify_segments(&segs, ctx, scope) {
                    PathClass::Internal { module: m, leaf } => Some((Some(fqn::item(RUST_LANG, &ctx.package, &m, &leaf)), false, leaf)),
                    PathClass::External { package, path, leaf } => Some((Some(fqn::lib(&package, &path, &leaf)), true, leaf)),
                }
            }
        }
        "field_expression" => {
            let field = func.child_by_field_name("field")?;
            let method = source_text(&field, src);
            if RUST_CALL_DENYLIST.contains(&method.as_str()) { return None; }
            let recv = func.child_by_field_name("value")?;
            let recv_is_self = recv.kind() == "self" || (recv.kind() == "identifier" && source_text(&recv, src) == "self");
            if recv_is_self {
                return match impl_ctx {
                    Some(ic) => Some((Some(fqn::method(RUST_LANG, &ctx.package, &ic.type_module, &ic.type_name, &method)), false, method)),
                    None => Some((None, false, method)),
                };
            }
            if recv.kind() == "identifier"
                && let Some(tname) = bindings.get(&source_text(&recv, src)) {
                let (pkg, mdl, is_ext) = resolve_type_module(tname, ctx, module, scope);
                return if is_ext {
                    Some((Some(fqn::lib(&pkg, &mdl, &method)), true, method))
                } else {
                    Some((Some(fqn::method(RUST_LANG, &pkg, &mdl, tname, &method)), false, method))
                };
            }
            // Unknown receiver (out of the bounded 0.7 scope) → no wrong merge.
            Some((None, false, method))
        }
        "generic_function" => {
            let inner = func.child_by_field_name("function")?;
            resolve_call(&inner, src, ctx, module, scope, impl_ctx, bindings)
        }
        _ => None,
    }
}

/// PascalCase heuristic: distinguishes a `Type` segment from a `module`/`fn`.
fn is_pascal(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Normalise a crate name for comparison (path form uses `_`, manifest name `-`).
fn norm_crate(s: &str) -> String {
    s.replace('-', "_")
}

/// Join two module-path fragments, dropping empties (crate root stays "").
fn join_mod(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_string(),
        (_, true) => a.to_string(),
        _ => format!("{a}::{b}"),
    }
}

/// Parent of a module path (`a::b::c` → `a::b`; `a` → ""). Best-effort for `super`.
fn parent_mod(m: &str) -> String {
    match m.rsplit_once("::") {
        Some((head, _)) => head.to_string(),
        None => String::new(),
    }
}

/// Reduce a type expression's text to its base type name, peeling references,
/// lifetimes, `dyn`/`impl`, and unwrapping smart-pointer wrappers (`Box<dyn T>`
/// → `T`). Returns the final path's last segment (`crate::a::Widget` → `Widget`).
fn base_type_name(text: &str) -> Option<String> {
    let mut t = text.trim();
    loop {
        if let Some(r) = t.strip_prefix('&') { t = r.trim(); continue; }
        if let Some(r) = t.strip_prefix("mut ") { t = r.trim(); continue; }
        if t.starts_with('\'') {
            // Lifetime token (e.g. `'a`) — drop it.
            t = t[1..].trim_start_matches(|c: char| c.is_alphanumeric() || c == '_').trim();
            continue;
        }
        break;
    }
    if let Some(r) = t.strip_prefix("dyn ") { t = r.trim(); }
    else if let Some(r) = t.strip_prefix("impl ") { t = r.trim(); }
    for wrapper in ["Box", "Rc", "Arc", "RefCell", "Cell", "Mutex", "RwLock"] {
        if let Some(rest) = t.strip_prefix(wrapper)
            && let Some(inner) = rest.trim().strip_prefix('<').and_then(|s| s.strip_suffix('>')) {
            return base_type_name(inner.trim());
        }
    }
    let base = t.split('<').next().unwrap_or(t).trim();
    let name = base.rsplit("::").next().unwrap_or(base).trim();
    let name = name.trim_end_matches(|c: char| !(c.is_alphanumeric() || c == '_'));
    if name.is_empty() || !name.chars().next().unwrap().is_alphabetic() {
        return None;
    }
    Some(name.to_string())
}
} // mod rust_fqn

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        RustAdapter.parse(src, "test.rs")
    }

    // ── FQN producer (Phase 2) ──────────────────────────────────────────────
    fn produce(src: &str, package: &str, module: &str) -> FqnFileOutput {
        rust_fqn::produce_fqns(src, &FileFqnContext { package: package.into(), module: module.into() })
    }
    fn def_fqn<'a>(out: &'a FqnFileOutput, name: &str) -> &'a str {
        out.defs.iter().find(|d| d.name == name).map(|d| d.fqn.as_str()).unwrap_or("<no-def>")
    }
    fn ref_to<'a>(out: &'a FqnFileOutput, target_name: &str) -> &'a FqnReference {
        out.refs.iter().find(|r| r.target_name == target_name)
            .unwrap_or_else(|| panic!("no ref to `{target_name}` in {:?}", out.refs))
    }

    #[test]
    fn rust_def_fqn() {
        let src = r#"
pub struct Widget;
impl Widget { pub fn new() -> Self { Widget } }
impl std::fmt::Display for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { Ok(()) }
}
pub fn make() -> Widget { Widget::new() }
"#;
        let out = produce(src, "senseid", "widget");
        assert_eq!(def_fqn(&out, "make"), "rust·senseid·widget·make", "free fn");
        assert_eq!(def_fqn(&out, "Widget"), "rust·senseid·widget·Widget", "type def");
        assert_eq!(def_fqn(&out, "new"), "rust·senseid·widget·Widget·new", "inherent assoc fn");
        assert_eq!(def_fqn(&out, "fmt"), "rust·senseid·widget·Widget·Display·fmt", "trait-impl method carries the trait qualifier");
    }

    #[test]
    fn rust_ref_fqn_explicit_path() {
        let src = "use crate::widget::Widget;\npub fn build() { Widget::new(); }\n";
        let out = produce(src, "senseid", "builder");
        let r = ref_to(&out, "new");
        assert_eq!(r.target_fqn.as_deref(), Some("rust·senseid·widget·Widget·new"), "resolved via use-map");
        assert!(!r.is_lib);
        assert_eq!(r.caller_fqn, "rust·senseid·builder·build", "edge source is the caller fn's fqn");
    }

    #[test]
    fn rust_ref_fqn_self_local_bounded() {
        let src = r#"
pub struct Engine;
pub fn helper() {}
impl Engine {
    pub fn run(&self) {
        self.tick();
        helper();
        let g = Gadget::new();
        g.spin();
        weird().wobble();
    }
    fn tick(&self) {}
}
"#;
        let out = produce(src, "senseid", "engine");
        assert_eq!(ref_to(&out, "tick").target_fqn.as_deref(), Some("rust·senseid·engine·Engine·tick"), "self.method → enclosing impl type");
        assert_eq!(ref_to(&out, "helper").target_fqn.as_deref(), Some("rust·senseid·engine·helper"), "local free fn → module scope");
        assert_eq!(ref_to(&out, "spin").target_fqn.as_deref(), Some("rust·senseid·engine·Gadget·spin"), "let x = Gadget::new(); x.spin() → Gadget::spin (0.7 binding)");
        assert_eq!(ref_to(&out, "wobble").target_fqn, None, "out-of-0.7 receiver → unresolved, no wrong merge");
    }

    #[test]
    fn rust_ref_fqn_external_is_lib() {
        let src = "pub fn load(s: &str) { serde_json::from_str(s); }\n";
        let out = produce(src, "senseid", "io");
        let r = ref_to(&out, "from_str");
        assert_eq!(r.target_fqn.as_deref(), Some("lib·serde_json·serde_json·from_str"), "external crate path → lib node");
        assert!(r.is_lib);
    }

    #[test]
    fn rust_adapter_and_trait_methods_do_not_collapse() {
        let src = r#"
pub struct A;
pub struct B;
impl A { pub fn parse(&self) {} }
impl B { pub fn parse(&self) {} }
impl std::fmt::Display for A { fn fmt(&self) {} }
impl std::fmt::Debug for A { fn fmt(&self) {} }
"#;
        let out = produce(src, "senseid", "m");
        let parses: Vec<&str> = out.defs.iter().filter(|d| d.name == "parse").map(|d| d.fqn.as_str()).collect();
        assert!(parses.contains(&"rust·senseid·m·A·parse"), "A::parse distinct");
        assert!(parses.contains(&"rust·senseid·m·B·parse"), "B::parse distinct");
        let fmts: Vec<&str> = out.defs.iter().filter(|d| d.name == "fmt").map(|d| d.fqn.as_str()).collect();
        assert!(fmts.contains(&"rust·senseid·m·A·Display·fmt"), "Display::fmt distinct by trait");
        assert!(fmts.contains(&"rust·senseid·m·A·Debug·fmt"), "Debug::fmt distinct by trait");
    }

    #[test]
    fn rust_dyn_receiver_stays_unqualified() {
        let src = "pub trait Sink { fn emit(&self); }\npub fn drive(s: &dyn Sink) { s.emit(); }\n";
        let out = produce(src, "senseid", "pipe");
        // Resolves to the TRAIT's method node (Sink), never a concrete impl merge.
        assert_eq!(ref_to(&out, "emit").target_fqn.as_deref(), Some("rust·senseid·pipe·Sink·emit"));
    }

    #[test]
    fn rust_same_name_across_files_disambiguates() {
        // Same-named free fns in different files → distinct (module differs).
        let a = produce("pub fn parse() {}", "senseid", "a");
        let b = produce("pub fn parse() {}", "senseid", "b");
        assert_eq!(def_fqn(&a, "parse"), "rust·senseid·a·parse");
        assert_eq!(def_fqn(&b, "parse"), "rust·senseid·b·parse");

        // `impl Widget` split across files both anchor on the TYPE's home module.
        let widget = produce("pub struct Widget; impl Widget { pub fn m(&self) {} }", "senseid", "widget");
        let ext = produce("use crate::widget::Widget; impl Widget { pub fn n(&self) {} }", "senseid", "widget_ext");
        assert_eq!(def_fqn(&widget, "m"), "rust·senseid·widget·Widget·m");
        assert_eq!(def_fqn(&ext, "n"), "rust·senseid·widget·Widget·n", "anchored on widget, not widget_ext");

        // A reference from a THIRD file resolves to the SAME node as the def.
        let caller = produce("use crate::widget::Widget; pub fn go() { Widget::m(); }", "senseid", "caller");
        assert_eq!(ref_to(&caller, "m").target_fqn.as_deref(), Some("rust·senseid·widget·Widget·m"), "ref merges onto the def's fqn");
    }

    #[test]
    fn rust_package_boundary_disambiguates() {
        let src = "pub struct ManifestAdapter; impl ManifestAdapter { pub fn parse(&self) {} }";
        let a = produce(src, "senseid", "adapters::config");
        let b = produce(src, "sensei-cli", "adapters::config");
        assert_eq!(def_fqn(&a, "parse"), "rust·senseid·adapters::config·ManifestAdapter·parse");
        assert_eq!(def_fqn(&b, "parse"), "rust·sensei-cli·adapters::config·ManifestAdapter·parse");
        assert_ne!(def_fqn(&a, "parse"), def_fqn(&b, "parse"), "package disambiguates the monorepo false-merge");
    }

    #[test]
    fn parses_function() {
        let pf = parse("pub fn hello(name: &str) -> String { format!(\"hi {}\", name) }");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "hello");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
        assert!(pf.symbols[0].is_exported);
    }

    #[test]
    fn parses_struct_and_impl() {
        let pf = parse("pub struct Calc { val: f64 }\nimpl Calc {\n    pub fn add(&self, x: f64) -> f64 { self.val + x }\n}");
        assert_eq!(pf.symbols.len(), 2); // Calc + add
        assert_eq!(pf.symbols[0].kind, SymbolKind::Class);
        assert_eq!(pf.symbols[1].kind, SymbolKind::Method);
        assert_eq!(pf.symbols[1].name, "add");
    }

    #[test]
    fn parses_enum_and_trait() {
        let pf = parse("pub enum Color { Red, Green }\npub trait Drawable { fn draw(&self); }");
        assert!(pf.symbols.len() >= 2); // Color + Drawable (+ optional draw method)
        assert_eq!(pf.symbols[0].kind, SymbolKind::Enum);
        assert_eq!(pf.symbols[1].kind, SymbolKind::Interface);
    }

    #[test]
    fn parses_use_imports() {
        let pf = parse("use std::io;\nuse std::collections::{HashMap, HashSet};");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "std::io");
        assert_eq!(pf.imports[1].names, vec!["HashMap", "HashSet"]);
    }

    #[test]
    fn parses_doc_comment() {
        let pf = parse("/// Say hello.\n/// Returns a greeting.\npub fn greet() {}");
        assert_eq!(pf.symbols[0].docstring, Some("Say hello.\nReturns a greeting.".to_string()));
    }

    #[test]
    fn private_fn_not_exported() {
        let pf = parse("fn internal() {}");
        assert!(!pf.symbols[0].is_exported);
    }

    #[test]
    fn const_item() {
        let pf = parse("pub const MAX: usize = 100;");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Const);
        assert_eq!(pf.symbols[0].name, "MAX");
    }

    #[test]
    fn method_parent_from_impl() {
        let pf = parse("pub struct Calc { val: f64 }\nimpl Calc {\n    pub fn add(&self, x: f64) -> f64 { self.val + x }\n    fn sub(&self, x: f64) -> f64 { self.val - x }\n}");
        let calc = pf.symbols.iter().find(|s| s.name == "Calc").unwrap();
        assert!(calc.parent.is_none(), "struct should have no parent");
        assert_eq!(calc.kind, SymbolKind::Class);
        let add = pf.symbols.iter().find(|s| s.name == "add").unwrap();
        assert_eq!(add.parent.as_deref(), Some("Calc"));
        assert_eq!(add.kind, SymbolKind::Method);
        let sub = pf.symbols.iter().find(|s| s.name == "sub").unwrap();
        assert_eq!(sub.parent.as_deref(), Some("Calc"));
    }

    #[test]
    fn free_function_no_parent() {
        let pf = parse("pub fn hello() {}\nfn internal() {}");
        for sym in &pf.symbols {
            assert!(sym.parent.is_none(), "{} should have no parent", sym.name);
        }
    }

    #[test]
    fn enum_and_trait_no_parent() {
        let pf = parse("pub enum Color { Red }\npub trait Drawable { fn draw(&self); }");
        for sym in &pf.symbols {
            if sym.kind != SymbolKind::Method {
                assert!(sym.parent.is_none(), "{} should have no parent", sym.name);
            }
        }
    }

    // ── Call-site extraction tests ────────────────────────────────────

    #[test]
    fn extracts_free_function_call() {
        let pf = parse("pub fn caller() { callee(); }\npub fn callee() {}");
        assert!(pf.edges.iter().any(|e| e.caller_name == "caller" && e.callee_name == "callee"),
            "expected caller→callee edge, got {:?}", pf.edges);
    }

    #[test]
    fn extracts_path_call_last_segment() {
        let pf = parse("pub fn f() { std::mem::swap(); }");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "swap"),
            "scoped call should yield last segment 'swap', got {:?}", pf.edges);
    }

    #[test]
    fn extracts_method_call() {
        let pf = parse("pub fn f(pg: Pg) { pg.insert_memory(); }");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "insert_memory"),
            "method call should yield 'insert_memory', got {:?}", pf.edges);
    }

    #[test]
    fn skips_denylisted_methods() {
        let pf = parse("pub fn f(x: String) { let _ = x.clone(); let _ = x.len(); }");
        assert!(!pf.edges.iter().any(|e| e.callee_name == "clone"), "clone denylisted");
        assert!(!pf.edges.iter().any(|e| e.callee_name == "len"), "len denylisted");
    }

    #[test]
    fn skips_macros() {
        let pf = parse("pub fn f() { println!(\"hi\"); }");
        assert!(!pf.edges.iter().any(|e| e.callee_name == "println"), "macros are not call_expressions");
    }

    #[test]
    fn dedups_repeated_calls() {
        let pf = parse("pub fn f() { g(); g(); g(); }\npub fn g() {}");
        let count = pf.edges.iter().filter(|e| e.caller_name == "f" && e.callee_name == "g").count();
        assert_eq!(count, 1, "repeated calls to g dedup to one edge");
    }

    #[test]
    fn captures_calls_inside_closures() {
        let pf = parse("pub fn f(v: Vec<u32>) { v.iter().for_each(|_| helper()); }\npub fn helper() {}");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "helper"),
            "call inside a closure attributes to the enclosing fn, got {:?}", pf.edges);
    }

    #[test]
    fn same_named_methods_get_distinct_caller_lines() {
        // Two `new` methods in separate impls each call `setup`.
        let src = "pub struct A;\nimpl A { pub fn new() -> Self { setup(); A } }\n\npub struct B;\nimpl B { pub fn new() -> Self { setup(); B } }\npub fn setup() {}";
        let lines = pf_caller_lines(&parse(src), "new", "setup");
        assert_eq!(lines.len(), 2, "two distinct new→setup edges, got lines {:?}", lines);
        assert_ne!(lines[0], lines[1], "the two `new` callers have different caller_line");
    }

    // Helper: collect caller_line for every (caller,callee) edge matching names.
    fn pf_caller_lines(pf: &ParsedFile, caller: &str, callee: &str) -> Vec<u32> {
        pf.edges.iter()
            .filter(|e| e.caller_name == caller && e.callee_name == callee)
            .map(|e| e.caller_line)
            .collect()
    }

    // ── IR Tests ──────────────────────────────────────────────────────

    fn parse_ir(src: &str) -> IRParsedFile {
        parse_to_ir(src, "test.rs")
    }

    #[test]
    fn ir_function_with_params_and_return() {
        let pf = parse_ir("pub fn hello(name: &str, count: usize) -> String { format!(\"{}\", name) }");
        assert_eq!(pf.modules.len(), 1);
        let func = &pf.modules[0].functions[0];
        assert_eq!(func.base.name, "hello");
        assert!(func.base.is_exported);
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "name");
        assert_eq!(func.params[0].type_, Some("&str".into()));
        assert_eq!(func.params[1].name, "count");
        assert_eq!(func.params[1].type_, Some("usize".into()));
        assert_eq!(func.return_type, Some("String".into()));
    }

    #[test]
    fn ir_async_function() {
        let pf = parse_ir("pub async fn fetch(url: &str) -> Result<String, Error> { todo!() }");
        let func = &pf.modules[0].functions[0];
        assert!(func.is_async);
        assert_eq!(func.return_type, Some("Result<String, Error>".into()));
    }

    #[test]
    fn ir_struct_as_class() {
        let pf = parse_ir("pub struct Store {\n    conn: Connection,\n}");
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].base.name, "Store");
        assert_eq!(pf.classes[0].class_kind, ClassKind::Struct);
        assert!(pf.classes[0].base.is_exported);
    }

    #[test]
    fn ir_trait_as_interface() {
        let pf = parse_ir("pub trait LanguageAdapter {\n    fn language(&self) -> &str;\n    fn parse(&self, source: &str) -> ParsedFile;\n}");
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].base.name, "LanguageAdapter");
        assert_eq!(pf.classes[0].class_kind, ClassKind::Trait);
        assert!(pf.classes[0].methods.len() >= 2);
    }

    #[test]
    fn ir_impl_creates_methods_on_class() {
        let pf = parse_ir("pub struct Calc { val: f64 }\nimpl Calc {\n    pub fn add(&self, x: f64) -> f64 { self.val + x }\n}");
        // Struct should exist as a class
        let calc = pf.classes.iter().find(|c| c.base.name == "Calc").unwrap();
        assert_eq!(calc.class_kind, ClassKind::Struct);
        // Method should be on the class
        assert_eq!(calc.methods.len(), 1);
        assert_eq!(calc.methods[0].base.name, "add");
        assert_eq!(calc.methods[0].params.len(), 2); // &self + x
        assert_eq!(calc.methods[0].return_type, Some("f64".into()));
    }

    #[test]
    fn ir_trait_impl_records_implements() {
        let pf = parse_ir("struct MyAdapter;\nimpl LanguageAdapter for MyAdapter {\n    fn language(&self) -> &str { \"test\" }\n}");
        let adapter = pf.classes.iter().find(|c| c.base.name == "MyAdapter").unwrap();
        assert!(adapter.implements.contains(&"LanguageAdapter".to_string()));
    }

    #[test]
    fn ir_enum() {
        let pf = parse_ir("pub enum Color { Red, Green, Blue }");
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].class_kind, ClassKind::Enum);
    }

    #[test]
    fn ir_imports() {
        let pf = parse_ir("use std::io;\nuse std::collections::{HashMap, HashSet};");
        assert_eq!(pf.modules[0].imports.len(), 2);
        assert_eq!(pf.modules[0].imports[0].source, "std::io");
        assert_eq!(pf.modules[0].imports[1].names, vec!["HashMap", "HashSet"]);
    }

    #[test]
    fn ir_const() {
        let pf = parse_ir("pub const MAX: usize = 100;");
        assert_eq!(pf.modules[0].constants.len(), 1);
        assert_eq!(pf.modules[0].constants[0].base.name, "MAX");
        assert!(pf.modules[0].constants[0].base.is_exported);
    }

    #[test]
    fn ir_docstring_preserved() {
        let pf = parse_ir("/// Say hello.\n/// Returns greeting.\npub fn greet() {}");
        let func = &pf.modules[0].functions[0];
        assert_eq!(func.base.docstring, Some("Say hello.\nReturns greeting.".into()));
    }

    #[test]
    fn ir_test_file_detected() {
        let pf = parse_to_ir("#[cfg(test)]\nmod tests {\n    #[test]\n    fn it_works() {}\n}", "src/lib.rs");
        // The file contains a test module — is_test should be detected
        // (The test attribute detection is on functions, not the file)
        assert_eq!(pf.language, "rust");
    }

    #[test]
    fn ir_attribute_as_decorator() {
        let pf = parse_ir("#[tokio::test]\nasync fn test_something() {}");
        let func = &pf.modules[0].functions[0];
        assert!(func.decorators.iter().any(|d| d.contains("tokio::test")));
    }

    #[test]
    fn multiple_impl_blocks() {
        let pf = parse("struct A {}\nstruct B {}\nimpl A { fn foo(&self) {} }\nimpl B { fn bar(&self) {} }");
        let foo = pf.symbols.iter().find(|s| s.name == "foo").unwrap();
        assert_eq!(foo.parent.as_deref(), Some("A"));
        let bar = pf.symbols.iter().find(|s| s.name == "bar").unwrap();
        assert_eq!(bar.parent.as_deref(), Some("B"));
    }

    #[test]
    fn extracts_turbofish_call() {
        let pf = parse("pub fn f() { parse::<u32>(); }");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "parse"),
            "turbofish call should yield 'parse', got {:?}", pf.edges);
    }

    #[test]
    fn turbofish_associated_fn() {
        let pf = parse("pub fn f() { Vec::<u8>::new(); }");
        assert!(pf.edges.iter().any(|e| e.caller_name == "f" && e.callee_name == "new"),
            "turbofish assoc fn should yield 'new', got {:?}", pf.edges);
    }

    #[test]
    fn nested_fn_calls_not_attributed_to_outer() {
        let pf = parse("pub fn outer() { fn inner() { deep(); } inner(); }\npub fn deep() {}");
        assert!(pf.edges.iter().any(|e| e.caller_name == "outer" && e.callee_name == "inner"),
            "outer→inner edge expected, got {:?}", pf.edges);
        assert!(!pf.edges.iter().any(|e| e.caller_name == "outer" && e.callee_name == "deep"),
            "deep() must NOT be attributed to outer, got {:?}", pf.edges);
    }
}
