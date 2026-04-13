use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, SymbolKind};
use super::LanguageAdapter;

pub struct TypeScriptAdapter;
pub struct JavaScriptAdapter;

impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> &str { "typescript" }
    fn extensions(&self) -> &[&str] { &[".ts", ".tsx"] }
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        parse_ts(source, file_path, true)
    }
}

impl LanguageAdapter for JavaScriptAdapter {
    fn language(&self) -> &str { "javascript" }
    fn extensions(&self) -> &[&str] { &[".js", ".jsx", ".mjs", ".cjs"] }
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        parse_ts(source, file_path, false)
    }
}

fn parse_ts(source: &str, file_path: &str, is_typescript: bool) -> ParsedFile {
    let mut parser = Parser::new();
    let lang = if is_typescript {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT
    } else {
        tree_sitter_javascript::LANGUAGE
    };
    parser.set_language(&lang.into()).expect("failed to set language");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return empty(file_path, is_typescript),
    };

    let src = source.as_bytes();
    let lines: Vec<&str> = source.lines().collect();
    let root = tree.root_node();
    let lang_name = if is_typescript { "typescript" } else { "javascript" };

    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    walk_ts(&root, src, &lines, &mut symbols, &mut imports);

    ParsedFile {
        file_path: file_path.to_string(),
        language: lang_name.to_string(),
        symbols,
        edges: vec![],
        imports,
    }
}

fn empty(path: &str, is_ts: bool) -> ParsedFile {
    ParsedFile {
        file_path: path.into(),
        language: if is_ts { "typescript" } else { "javascript" }.into(),
        symbols: vec![], edges: vec![], imports: vec![],
    }
}

fn walk_ts(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_declaration" | "generator_function_declaration" => {
                let name = field_text(&child, "name", src);
                let is_exported = is_ts_exported(&child);
                symbols.push(make_symbol(name, SymbolKind::Function, &child, lines, src, is_exported));
            }
            "class_declaration" => {
                let name = field_text(&child, "name", src);
                let is_exported = is_ts_exported(&child);
                symbols.push(make_symbol(name, SymbolKind::Class, &child, lines, src, is_exported));
                // Methods inside class body
                if let Some(body) = child.child_by_field_name("body") {
                    extract_class_members(&body, src, lines, symbols);
                }
            }
            "interface_declaration" => {
                let name = field_text(&child, "name", src);
                symbols.push(make_symbol(name, SymbolKind::Interface, &child, lines, src, is_ts_exported(&child)));
            }
            "type_alias_declaration" => {
                let name = field_text(&child, "name", src);
                symbols.push(make_symbol(name, SymbolKind::Type, &child, lines, src, is_ts_exported(&child)));
            }
            "enum_declaration" => {
                let name = field_text(&child, "name", src);
                symbols.push(make_symbol(name, SymbolKind::Enum, &child, lines, src, is_ts_exported(&child)));
            }
            "lexical_declaration" | "variable_declaration" => {
                // export const FOO = ...
                extract_const_decl(&child, src, lines, symbols);
            }
            "export_statement" => {
                // export function/class/const/etc — recurse into the exported declaration
                walk_ts(&child, src, lines, symbols, imports);
            }
            "import_statement" => {
                extract_import(&child, src, imports);
            }
            _ => {}
        }
    }
}

fn extract_class_members(body: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>) {
    for i in 0..body.child_count() {
        let child = body.child(i).unwrap();
        if child.kind() == "method_definition" || child.kind() == "public_field_definition" {
            let name = field_text(&child, "name", src);
            if !name.is_empty() && child.kind() == "method_definition" {
                symbols.push(make_symbol(name, SymbolKind::Method, &child, lines, src, true));
            }
        }
    }
}

fn extract_const_decl(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>) {
    let is_exported = is_ts_exported(node);
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "variable_declarator" {
            let name = field_text(&child, "name", src);
            if !name.is_empty() {
                // Check if it looks like a constant (UPPER_CASE) or an arrow function
                let value = child.child_by_field_name("value");
                let kind = if value.map_or(false, |v| v.kind() == "arrow_function" || v.kind() == "function") {
                    SymbolKind::Function
                } else {
                    SymbolKind::Const
                };
                symbols.push(make_symbol(name, kind, node, lines, src, is_exported));
            }
        }
    }
}

fn extract_import(node: &Node, src: &[u8], imports: &mut Vec<ParsedImport>) {
    let text = node.utf8_text(src).unwrap_or_default();
    // Simple extraction: find the source string
    if let Some(source_node) = node.child_by_field_name("source") {
        let target = source_node.utf8_text(src).unwrap_or_default()
            .trim_matches(|c: char| c == '"' || c == '\'' || c == '`')
            .to_string();
        let mut names = Vec::new();
        // Collect named imports
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind() == "import_clause" || child.kind() == "named_imports" {
                collect_import_names(&child, src, &mut names);
            }
        }
        if names.is_empty() && text.contains("* as") {
            names.push("*".to_string());
        }
        imports.push(ParsedImport { target_path: target, names });
    }
}

fn collect_import_names(node: &Node, src: &[u8], names: &mut Vec<String>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "identifier" | "type_identifier" => {
                names.push(child.utf8_text(src).unwrap_or_default().to_string());
            }
            "import_specifier" => {
                if let Some(n) = child.child_by_field_name("name") {
                    names.push(n.utf8_text(src).unwrap_or_default().to_string());
                }
            }
            "named_imports" => collect_import_names(&child, src, names),
            "import_clause" => collect_import_names(&child, src, names),
            _ => {}
        }
    }
}

fn make_symbol(name: String, kind: SymbolKind, node: &Node, lines: &[&str], src: &[u8], is_exported: bool) -> ParsedSymbol {
    ParsedSymbol {
        name,
        kind,
        signature: lines.get(node.start_position().row).map(|l| l.trim().to_string()),
        docstring: extract_jsdoc(node, src),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        is_exported,
    }
}

fn extract_jsdoc(node: &Node, src: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "comment" { return None; }
    let text = prev.utf8_text(src).ok()?;
    if !text.starts_with("/**") { return None; }
    let inner = text.trim_start_matches("/**").trim_end_matches("*/").trim();
    let cleaned: Vec<&str> = inner.lines()
        .map(|l| l.trim().trim_start_matches('*').trim())
        .filter(|l| !l.is_empty())
        .collect();
    if cleaned.is_empty() { None } else { Some(cleaned.join("\n")) }
}

fn field_text(node: &Node, field: &str, src: &[u8]) -> String {
    node.child_by_field_name(field)
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or_default()
        .to_string()
}

fn is_ts_exported(node: &Node) -> bool {
    if let Some(parent) = node.parent() {
        if parent.kind() == "export_statement" { return true; }
    }
    // Check for "export" keyword child
    (0..node.child_count()).any(|i| {
        node.child(i).map_or(false, |c| c.kind() == "export" || c.utf8_text(&[]).unwrap_or_default() == "export")
    })
}

// TS/JS grammar 0.23 has linking issues with tree-sitter 0.24 on macOS arm64.
// Tests disabled until grammar versions align.
#[cfg(test)]
#[cfg(feature = "__ts_adapter_tests")]
mod tests {
    use super::*;

    fn parse_ts_src(src: &str) -> ParsedFile { TypeScriptAdapter.parse(src, "test.ts") }
    fn parse_js_src(src: &str) -> ParsedFile { JavaScriptAdapter.parse(src, "test.js") }

    #[test]
    fn ts_function() {
        let pf = parse_ts_src("function hello(name: string): string { return name; }");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "hello");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn ts_class_with_methods() {
        let pf = parse_ts_src("class Foo {\n  bar() {}\n  baz() {}\n}");
        assert!(pf.symbols.len() >= 3); // Foo + bar + baz
        assert_eq!(pf.symbols[0].kind, SymbolKind::Class);
    }

    #[test]
    fn ts_interface_and_type() {
        let pf = parse_ts_src("interface Foo { x: number }\ntype Bar = string;");
        assert_eq!(pf.symbols.len(), 2);
        assert_eq!(pf.symbols[0].kind, SymbolKind::Interface);
        assert_eq!(pf.symbols[1].kind, SymbolKind::Type);
    }

    #[test]
    fn ts_enum() {
        let pf = parse_ts_src("enum Color { Red, Green }");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn ts_const_and_arrow() {
        let pf = parse_ts_src("const TIMEOUT = 30;\nconst greet = (name: string) => name;");
        assert_eq!(pf.symbols.len(), 2);
        assert_eq!(pf.symbols[0].kind, SymbolKind::Const);
        assert_eq!(pf.symbols[1].kind, SymbolKind::Function); // arrow fn
    }

    #[test]
    fn ts_imports() {
        let pf = parse_ts_src("import { readFile } from 'fs';\nimport express from 'express';");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "fs");
        assert_eq!(pf.imports[1].target_path, "express");
    }

    #[test]
    fn js_function() {
        let pf = parse_js_src("function hello() { return 1; }");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
        assert_eq!(pf.language, "javascript");
    }
}
