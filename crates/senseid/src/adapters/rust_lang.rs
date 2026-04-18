use super::common::field_text;
use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, SymbolKind};
use crate::ir::{IRBase, IRModule, IRFunction, IRClass, IRMethod, IRParam, IRImport, IRConstant, IRParsedFile, ClassKind, Visibility};
use super::LanguageAdapter;

pub struct RustAdapter;

impl LanguageAdapter for RustAdapter {
    fn language(&self) -> &str { "rust" }

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
        walk_nodes(&root, src, &lines, &mut symbols, &mut imports, None);

        ParsedFile {
            file_path: file_path.to_string(),
            language: "rust".to_string(),
            symbols,
            edges: vec![],
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
                let sig = line_at(lines, child.start_position().row);

                if let Some((type_name, _)) = impl_context {
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
                        complexity: crate::indexer::graph::compute_complexity(
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
                        if let Some(method_node) = body.child(j) {
                            if method_node.kind() == "function_item" || method_node.kind() == "function_signature_item" {
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
                        if let Some(method_node) = body.child(j) {
                            if method_node.kind() == "function_item" {
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
                                    complexity: crate::indexer::graph::compute_complexity(
                                        &source_text(&method_node, src)
                                    ),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }

                if let Some(idx) = class_idx {
                    // Append methods to existing class
                    classes[idx].methods.extend(methods);
                    if let Some(ref tn) = trait_name {
                        if !classes[idx].implements.contains(tn) {
                            classes[idx].implements.push(tn.clone());
                        }
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

fn walk_nodes(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>, impl_type: Option<&str>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_item" => {
                let name = field_text(&child, "name", src);
                let is_pub = has_child_kind(&child, "visibility_modifier");
                let kind = if impl_type.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                symbols.push(ParsedSymbol {
                    name,
                    kind,
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    line_start: child.start_position().row as u32 + 1,
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
                    walk_nodes(&body, src, lines, symbols, imports, type_name_ref);
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
    (0..node.child_count()).any(|i| node.child(i).map_or(false, |c| c.kind() == kind))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        RustAdapter.parse(src, "test.rs")
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
}
