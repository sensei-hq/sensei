use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport, SymbolKind};
use super::LanguageAdapter;

pub struct RustAdapter;

impl LanguageAdapter for RustAdapter {
    fn language(&self) -> &str { "rust" }
    fn extensions(&self) -> &[&str] { &[".rs"] }

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
        walk_nodes(&root, src, &lines, &mut symbols, &mut imports, false);

        ParsedFile {
            file_path: file_path.to_string(),
            language: "rust".to_string(),
            symbols,
            edges: vec![],
            imports,
        }
    }
}

fn empty(path: &str) -> ParsedFile {
    ParsedFile { file_path: path.into(), language: "rust".into(), symbols: vec![], edges: vec![], imports: vec![] }
}

fn walk_nodes(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>, in_impl: bool) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_item" => {
                let name = field_text(&child, "name", src);
                let is_pub = has_child_kind(&child, "visibility_modifier");
                let kind = if in_impl { SymbolKind::Method } else { SymbolKind::Function };
                symbols.push(ParsedSymbol {
                    name,
                    kind,
                    signature: line_at(lines, child.start_position().row),
                    docstring: collect_doc_comments(&child, src),
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: is_pub,
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
                });
            }
            "impl_item" => {
                // Recurse into impl body for methods
                if let Some(body) = child.child_by_field_name("body") {
                    walk_nodes(&body, src, lines, symbols, imports, true);
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

fn field_text(node: &Node, field: &str, src: &[u8]) -> String {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or_default()
        .to_string()
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
}
