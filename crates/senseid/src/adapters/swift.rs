use tree_sitter::{Language, Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, SymbolKind};
use super::LanguageAdapter;

unsafe extern "C" {
    fn tree_sitter_swift() -> Language;
}

pub struct SwiftAdapter;

impl LanguageAdapter for SwiftAdapter {
    fn language(&self) -> &str { "swift" }
    fn extensions(&self) -> &[&str] { &[".swift"] }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let mut parser = Parser::new();
        let lang = unsafe { tree_sitter_swift() };
        parser.set_language(&lang).expect("failed to set swift language");

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return empty(file_path),
        };

        let src = source.as_bytes();
        let lines: Vec<&str> = source.lines().collect();
        let root = tree.root_node();

        let mut symbols = Vec::new();
        let mut imports = Vec::new();
        walk(&root, src, &lines, &mut symbols, &mut imports);

        ParsedFile {
            file_path: file_path.to_string(),
            language: "swift".to_string(),
            symbols,
            edges: vec![],
            imports,
        }
    }
}

fn empty(path: &str) -> ParsedFile {
    ParsedFile { file_path: path.into(), language: "swift".into(), symbols: vec![], edges: vec![], imports: vec![] }
}

fn walk(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_declaration" => {
                let name = field_text(&child, "name", src);
                if name.is_empty() { continue; }
                let is_pub = has_access_modifier(&child, src, "public") || has_access_modifier(&child, src, "open");
                let kind = if is_inside_body(node) { SymbolKind::Method } else { SymbolKind::Function };
                symbols.push(make_sym(name, kind, &child, lines, src, is_pub));
            }
            "class_declaration" => {
                let name = find_type_name(&child, src);
                if name.is_empty() { continue; }
                let is_pub = has_access_modifier(&child, src, "public") || has_access_modifier(&child, src, "open");
                let kind = if has_keyword(&child, "struct") { SymbolKind::Struct }
                    else if has_keyword(&child, "enum") { SymbolKind::Enum }
                    else { SymbolKind::Class };
                symbols.push(make_sym(name, kind, &child, lines, src, is_pub));
                // Recurse into body
                for j in 0..child.child_count() {
                    let cc = child.child(j).unwrap();
                    if cc.kind() == "class_body" || cc.kind() == "enum_class_body" {
                        walk(&cc, src, lines, symbols, imports);
                    }
                }
            }
            "protocol_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    symbols.push(make_sym(name, SymbolKind::Interface, &child, lines, src, has_access_modifier(&child, src, "public")));
                }
            }
            // enums come through class_declaration with "enum" keyword
            "typealias_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    symbols.push(make_sym(name, SymbolKind::Type, &child, lines, src, has_access_modifier(&child, src, "public")));
                }
            }
            "import_declaration" => {
                // import Foundation
                let text = child.utf8_text(src).unwrap_or_default();
                let module = text.strip_prefix("import")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if !module.is_empty() {
                    imports.push(ParsedImport { target_path: module, names: vec![] });
                }
            }
            "property_declaration" => {
                if !is_inside_body(node) {
                    let name = find_pattern_name(&child, src);
                    if !name.is_empty() {
                        symbols.push(make_sym(name, SymbolKind::Const, &child, lines, src, has_access_modifier(&child, src, "public")));
                    }
                }
            }
            "init_declaration" => {
                symbols.push(make_sym("init".into(), SymbolKind::Method, &child, lines, src, true));
            }
            "deinit_declaration" => {
                symbols.push(make_sym("deinit".into(), SymbolKind::Method, &child, lines, src, true));
            }
            "extension_declaration" => {
                // Recurse into extension body
                for j in 0..child.child_count() {
                    let cc = child.child(j).unwrap();
                    if cc.kind().contains("body") || cc.kind() == "class_body" {
                        walk(&cc, src, lines, symbols, imports);
                    }
                }
            }
            _ => {}
        }
    }
}

fn has_keyword(node: &Node, keyword: &str) -> bool {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if !child.is_named() {
            if child.kind() == keyword { return true; }
        }
    }
    false
}

fn is_inside_body(node: &Node) -> bool {
    let k = node.kind();
    k.contains("body") || k == "class_body" || k == "protocol_body" || k == "extension_body"
}

fn find_type_name(node: &Node, src: &[u8]) -> String {
    // Try field "name" first, then look for type_identifier child
    let name = field_text(node, "name", src);
    if !name.is_empty() { return name; }
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "type_identifier" || child.kind() == "simple_identifier" {
            return child.utf8_text(src).unwrap_or_default().to_string();
        }
    }
    String::new()
}

fn find_pattern_name(node: &Node, src: &[u8]) -> String {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "pattern" | "simple_identifier" => {
                return child.utf8_text(src).unwrap_or_default().to_string();
            }
            "property_binding_pattern" | "value_binding_pattern" => {
                return find_pattern_name(&child, src);
            }
            _ => {}
        }
    }
    String::new()
}

fn has_access_modifier(node: &Node, src: &[u8], modifier: &str) -> bool {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        let k = child.kind();
        if k.contains("modifier") || k == "attribute" {
            let text = child.utf8_text(src).unwrap_or_default();
            if text.contains(modifier) { return true; }
        }
    }
    false
}

fn make_sym(name: String, kind: SymbolKind, node: &Node, lines: &[&str], src: &[u8], is_exported: bool) -> ParsedSymbol {
    ParsedSymbol {
        name,
        kind,
        signature: lines.get(node.start_position().row).map(|l| l.trim().to_string()),
        docstring: extract_doc_comment(node, src),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        is_exported,
    }
}

fn extract_doc_comment(node: &Node, src: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "comment" && prev.kind() != "multiline_comment" { return None; }
    let text = prev.utf8_text(src).ok()?;
    if text.starts_with("///") {
        Some(text.trim_start_matches('/').trim().to_string())
    } else if text.starts_with("/**") {
        let inner = text.trim_start_matches("/**").trim_end_matches("*/").trim();
        let cleaned: Vec<&str> = inner.lines()
            .map(|l| l.trim().trim_start_matches('*').trim())
            .filter(|l| !l.is_empty())
            .collect();
        if cleaned.is_empty() { None } else { Some(cleaned.join("\n")) }
    } else {
        None
    }
}

fn field_text(node: &Node, field: &str, src: &[u8]) -> String {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile { SwiftAdapter.parse(src, "test.swift") }

    #[test]
    fn swift_function() {
        let pf = parse("func greet(name: String) -> String {\n    return \"hello \\(name)\"\n}");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "greet");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn swift_class_with_methods() {
        let pf = parse("class Dog {\n    func bark() {}\n    func sit() {}\n}");
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Dog"));
        assert!(names.contains(&"bark"));
        assert!(names.contains(&"sit"));
    }

    #[test]
    fn swift_struct() {
        let pf = parse("struct Point {\n    var x: Int\n    var y: Int\n}");
        assert_eq!(pf.symbols[0].name, "Point");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn swift_protocol() {
        let pf = parse("protocol Drawable {\n    func draw()\n}");
        assert_eq!(pf.symbols[0].name, "Drawable");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Interface);
    }

    #[test]
    fn swift_enum() {
        let pf = parse("enum Direction {\n    case north, south\n}");
        assert_eq!(pf.symbols[0].name, "Direction");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn swift_imports() {
        let pf = parse("import Foundation\nimport UIKit\nfunc hello() {}");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "Foundation");
        assert_eq!(pf.imports[1].target_path, "UIKit");
    }

    #[test]
    fn swift_language() {
        let pf = parse("func x() {}");
        assert_eq!(pf.language, "swift");
    }

    #[test]
    fn swift_public_function() {
        let pf = parse("public func serve() {}");
        assert!(pf.symbols[0].is_exported);
    }
}
