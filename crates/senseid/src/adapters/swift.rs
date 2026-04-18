use super::common::{field_text, make_symbol};
use tree_sitter::{Language, Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, SymbolKind};
use super::LanguageAdapter;

unsafe extern "C" {
    fn tree_sitter_swift() -> Language;
}

pub struct SwiftAdapter;

impl LanguageAdapter for SwiftAdapter {
    fn language(&self) -> &str { "swift" }

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
    walk_with_parent(node, src, lines, symbols, imports, None);
}

fn walk_with_parent(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>, class_name: Option<&str>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_declaration" => {
                let name = field_text(&child, "name", src);
                if name.is_empty() { continue; }
                let is_pub = has_access_modifier(&child, src, "public") || has_access_modifier(&child, src, "open");
                let kind = if class_name.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                let mut sym = make_sym(name, kind, &child, lines, src, is_pub);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "class_declaration" => {
                let name = find_type_name(&child, src);
                if name.is_empty() { continue; }
                let is_pub = has_access_modifier(&child, src, "public") || has_access_modifier(&child, src, "open");
                let kind = if has_keyword(&child, "struct") { SymbolKind::Struct }
                    else if has_keyword(&child, "enum") { SymbolKind::Enum }
                    else { SymbolKind::Class };
                symbols.push(make_sym(name.clone(), kind, &child, lines, src, is_pub));
                for j in 0..child.child_count() {
                    let cc = child.child(j).unwrap();
                    if cc.kind() == "class_body" || cc.kind() == "enum_class_body" {
                        walk_with_parent(&cc, src, lines, symbols, imports, Some(&name));
                    }
                }
            }
            "protocol_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    symbols.push(make_sym(name, SymbolKind::Interface, &child, lines, src, has_access_modifier(&child, src, "public")));
                }
            }
            "typealias_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    symbols.push(make_sym(name, SymbolKind::Type, &child, lines, src, has_access_modifier(&child, src, "public")));
                }
            }
            "import_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let module = text.strip_prefix("import")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if !module.is_empty() {
                    imports.push(ParsedImport { target_path: module, names: vec![] });
                }
            }
            "property_declaration" => {
                if class_name.is_none() {
                    let name = find_pattern_name(&child, src);
                    if !name.is_empty() {
                        symbols.push(make_sym(name, SymbolKind::Const, &child, lines, src, has_access_modifier(&child, src, "public")));
                    }
                }
            }
            "init_declaration" => {
                let mut sym = make_sym("init".into(), SymbolKind::Method, &child, lines, src, true);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "deinit_declaration" => {
                let mut sym = make_sym("deinit".into(), SymbolKind::Method, &child, lines, src, true);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "extension_declaration" => {
                // Extract extended type name
                let ext_name = find_type_name(&child, src);
                let parent = if ext_name.is_empty() { class_name } else { Some(ext_name.as_str()) };
                for j in 0..child.child_count() {
                    let cc = child.child(j).unwrap();
                    if cc.kind().contains("body") || cc.kind() == "class_body" {
                        walk_with_parent(&cc, src, lines, symbols, imports, parent);
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
    make_symbol(name, kind, node, lines, is_exported, extract_doc_comment(node, src))
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

    #[test]
    fn method_parent_set_on_class() {
        let pf = parse("class Dog {\n    func bark() {}\n    func sit() {}\n}");
        let dog = pf.symbols.iter().find(|s| s.name == "Dog").unwrap();
        assert!(dog.parent.is_none(), "class should have no parent");
        let bark = pf.symbols.iter().find(|s| s.name == "bark").unwrap();
        assert_eq!(bark.parent.as_deref(), Some("Dog"));
        assert_eq!(bark.kind, SymbolKind::Method);
        let sit = pf.symbols.iter().find(|s| s.name == "sit").unwrap();
        assert_eq!(sit.parent.as_deref(), Some("Dog"));
    }

    #[test]
    fn method_parent_on_struct() {
        let pf = parse("struct Point {\n    var x: Int\n    var y: Int\n    func distance() -> Double { return 0.0 }\n}");
        let dist = pf.symbols.iter().find(|s| s.name == "distance").unwrap();
        assert_eq!(dist.parent.as_deref(), Some("Point"));
    }

    #[test]
    fn init_has_parent() {
        let pf = parse("class Foo {\n    init() {}\n}");
        let init = pf.symbols.iter().find(|s| s.name == "init").unwrap();
        assert_eq!(init.parent.as_deref(), Some("Foo"));
    }

    #[test]
    fn free_function_no_parent() {
        let pf = parse("func greet() {}");
        assert!(pf.symbols[0].parent.is_none());
    }

    #[test]
    fn protocol_no_parent() {
        let pf = parse("protocol Drawable {\n    func draw()\n}");
        let drawable = pf.symbols.iter().find(|s| s.name == "Drawable").unwrap();
        assert!(drawable.parent.is_none());
    }
}
