use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, ParsedEdge, SymbolKind};
use super::LanguageAdapter;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::*;

pub struct TypeScriptAdapter;
pub struct JavaScriptAdapter;

impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> &str { "typescript" }
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        parse_oxc(source, file_path)
    }
}

impl LanguageAdapter for JavaScriptAdapter {
    fn language(&self) -> &str { "javascript" }
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        parse_oxc(source, file_path)
    }
}

fn parse_oxc(source: &str, file_path: &str) -> ParsedFile {
    let source_type = SourceType::from_path(file_path).unwrap_or_default();
    let lang_name = if source_type.is_typescript() { "typescript" } else { "javascript" };

    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();

    if ret.panicked {
        return ParsedFile {
            file_path: file_path.into(),
            language: lang_name.into(),
            symbols: vec![], edges: vec![], imports: vec![],
        };
    }

    let lines: Vec<&str> = source.lines().collect();
    let program = &ret.program;
    let mut symbols = Vec::new();
    let mut imports = Vec::new();
    let mut edges = Vec::new();

    for stmt in &program.body {
        extract_statement(stmt, source, &lines, &mut symbols, &mut imports, &mut edges);
    }

    ParsedFile {
        file_path: file_path.into(),
        language: lang_name.into(),
        symbols,
        edges,
        imports,
    }
}

fn line_col(source: &str, offset: u32) -> u32 {
    // Convert byte offset to 1-based line number
    source[..offset as usize].matches('\n').count() as u32 + 1
}

fn extract_statement(
    stmt: &Statement, source: &str, lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>,
    _edges: &mut Vec<ParsedEdge>,
) {
    match stmt {
        Statement::FunctionDeclaration(f) => {
            if let Some(id) = &f.id {
                let start = line_col(source, f.span.start);
                let end = line_col(source, f.span.end);
                symbols.push(make_sym(id.name.to_string(), SymbolKind::Function, start, end, lines, false));
            }
        }
        Statement::ClassDeclaration(c) => {
            if let Some(id) = &c.id {
                let start = line_col(source, c.span.start);
                let end = line_col(source, c.span.end);
                let class_name = id.name.to_string();
                symbols.push(make_sym(class_name.clone(), SymbolKind::Class, start, end, lines, false));
                extract_class_body(&c.body, source, lines, symbols, &class_name);
            }
        }
        Statement::VariableDeclaration(var) => {
            extract_var_decl(var, source, lines, symbols, false);
        }
        Statement::ExportNamedDeclaration(export) => {
            if let Some(decl) = &export.declaration {
                extract_exported_decl(decl, source, lines, symbols);
            }
        }
        Statement::ExportDefaultDeclaration(export) => {
            match &export.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                    let name = f.id.as_ref().map(|i| i.name.to_string()).unwrap_or_else(|| "default".into());
                    let start = line_col(source, f.span.start);
                    let end = line_col(source, f.span.end);
                    symbols.push(make_sym(name, SymbolKind::Function, start, end, lines, true));
                }
                ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                    let name = c.id.as_ref().map(|i| i.name.to_string()).unwrap_or_else(|| "default".into());
                    let start = line_col(source, c.span.start);
                    let end = line_col(source, c.span.end);
                    symbols.push(make_sym(name.clone(), SymbolKind::Class, start, end, lines, true));
                    extract_class_body(&c.body, source, lines, symbols, &name);
                }
                ExportDefaultDeclarationKind::TSInterfaceDeclaration(iface) => {
                    let start = line_col(source, iface.span.start);
                    let end = line_col(source, iface.span.end);
                    symbols.push(make_sym(iface.id.name.to_string(), SymbolKind::Interface, start, end, lines, true));
                }
                _ => {}
            }
        }
        Statement::ImportDeclaration(import) => {
            let target = import.source.value.to_string();
            let names: Vec<String> = import.specifiers.as_ref().map(|specs| {
                specs.iter().map(|s| match s {
                    ImportDeclarationSpecifier::ImportSpecifier(named) => named.local.name.to_string(),
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(def) => def.local.name.to_string(),
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => format!("* as {}", ns.local.name),
                }).collect()
            }).unwrap_or_default();
            imports.push(ParsedImport { target_path: target, names });
        }
        Statement::TSInterfaceDeclaration(iface) => {
            let start = line_col(source, iface.span.start);
            let end = line_col(source, iface.span.end);
            symbols.push(make_sym(iface.id.name.to_string(), SymbolKind::Interface, start, end, lines, false));
        }
        Statement::TSTypeAliasDeclaration(alias) => {
            let start = line_col(source, alias.span.start);
            let end = line_col(source, alias.span.end);
            symbols.push(make_sym(alias.id.name.to_string(), SymbolKind::Type, start, end, lines, false));
        }
        Statement::TSEnumDeclaration(en) => {
            let start = line_col(source, en.span.start);
            let end = line_col(source, en.span.end);
            symbols.push(make_sym(en.id.name.to_string(), SymbolKind::Enum, start, end, lines, false));
        }
        _ => {}
    }
}

fn extract_exported_decl(
    decl: &Declaration, source: &str, lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
) {
    match decl {
        Declaration::FunctionDeclaration(f) => {
            if let Some(id) = &f.id {
                let start = line_col(source, f.span.start);
                let end = line_col(source, f.span.end);
                symbols.push(make_sym(id.name.to_string(), SymbolKind::Function, start, end, lines, true));
            }
        }
        Declaration::ClassDeclaration(c) => {
            if let Some(id) = &c.id {
                let start = line_col(source, c.span.start);
                let end = line_col(source, c.span.end);
                let class_name = id.name.to_string();
                symbols.push(make_sym(class_name.clone(), SymbolKind::Class, start, end, lines, true));
                extract_class_body(&c.body, source, lines, symbols, &class_name);
            }
        }
        Declaration::VariableDeclaration(var) => {
            extract_var_decl(var, source, lines, symbols, true);
        }
        Declaration::TSInterfaceDeclaration(iface) => {
            let start = line_col(source, iface.span.start);
            let end = line_col(source, iface.span.end);
            symbols.push(make_sym(iface.id.name.to_string(), SymbolKind::Interface, start, end, lines, true));
        }
        Declaration::TSTypeAliasDeclaration(alias) => {
            let start = line_col(source, alias.span.start);
            let end = line_col(source, alias.span.end);
            symbols.push(make_sym(alias.id.name.to_string(), SymbolKind::Type, start, end, lines, true));
        }
        Declaration::TSEnumDeclaration(en) => {
            let start = line_col(source, en.span.start);
            let end = line_col(source, en.span.end);
            symbols.push(make_sym(en.id.name.to_string(), SymbolKind::Enum, start, end, lines, true));
        }
        _ => {}
    }
}

fn extract_var_decl(
    var: &VariableDeclaration, source: &str, lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>, is_exported: bool,
) {
    for decl in &var.declarations {
        if let BindingPattern::BindingIdentifier(id) = &decl.id {
            let name = id.name.to_string();
            let kind = match &decl.init {
                Some(Expression::ArrowFunctionExpression(_)) | Some(Expression::FunctionExpression(_)) => SymbolKind::Function,
                _ => SymbolKind::Const,
            };
            let start = line_col(source, decl.span.start);
            let end = line_col(source, decl.span.end);
            symbols.push(make_sym(name, kind, start, end, lines, is_exported));
        }
    }
}

fn extract_class_body(
    body: &ClassBody, source: &str, lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>, class_name: &str,
) {
    for element in &body.body {
        match element {
            ClassElement::MethodDefinition(m) => {
                if let Some(name) = method_name(&m.key) {
                    let start = line_col(source, m.span.start);
                    let end = line_col(source, m.span.end);
                    let mut sym = make_sym(name, SymbolKind::Method, start, end, lines, true);
                    sym.parent = Some(class_name.to_string());
                    symbols.push(sym);
                }
            }
            _ => {}
        }
    }
}

fn method_name(key: &PropertyKey) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::PrivateIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

fn make_sym(
    name: String, kind: SymbolKind,
    line_start: u32, line_end: u32,
    lines: &[&str], is_exported: bool,
) -> ParsedSymbol {
    let signature = lines.get(line_start.saturating_sub(1) as usize).map(|l| l.trim().to_string());
    ParsedSymbol {
        name,
        kind,
        signature,
        docstring: None,
        line_start,
        line_end,
        is_exported,
        parent: None,
    }
}

#[cfg(test)]
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
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Foo"));
        assert!(names.contains(&"bar"));
        assert!(names.contains(&"baz"));
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
        assert_eq!(pf.symbols[0].name, "Color");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn ts_const_and_arrow() {
        let pf = parse_ts_src("const TIMEOUT = 30;\nconst greet = (name: string) => name;");
        assert_eq!(pf.symbols.len(), 2);
        assert_eq!(pf.symbols[0].kind, SymbolKind::Const);
        assert_eq!(pf.symbols[1].kind, SymbolKind::Function);
    }

    #[test]
    fn ts_exports() {
        let pf = parse_ts_src("export function hello() {}\nfunction internal() {}");
        assert_eq!(pf.symbols.len(), 2);
        assert!(pf.symbols[0].is_exported);
        assert!(!pf.symbols[1].is_exported);
    }

    #[test]
    fn ts_imports() {
        let pf = parse_ts_src("import { readFile } from 'fs';\nimport express from 'express';");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "fs");
        assert_eq!(pf.imports[0].names, vec!["readFile"]);
        assert_eq!(pf.imports[1].target_path, "express");
        assert_eq!(pf.imports[1].names, vec!["express"]);
    }

    #[test]
    fn tsx_jsx() {
        let pf = TypeScriptAdapter.parse(
            "export function App() { return <div>Hello</div>; }",
            "test.tsx",
        );
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "App");
    }

    #[test]
    fn js_function() {
        let pf = parse_js_src("function hello() { return 1; }");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
        assert_eq!(pf.language, "javascript");
    }

    #[test]
    fn js_jsx() {
        let pf = JavaScriptAdapter.parse(
            "function App() { return <div/>; }",
            "test.jsx",
        );
        assert_eq!(pf.symbols.len(), 1);
    }

    #[test]
    fn syntax_error_returns_empty() {
        let pf = parse_ts_src("function {{{ broken");
        assert!(pf.symbols.is_empty());
    }

    #[test]
    fn namespace_import() {
        let pf = parse_ts_src("import * as path from 'path';");
        assert_eq!(pf.imports.len(), 1);
        assert_eq!(pf.imports[0].target_path, "path");
        assert_eq!(pf.imports[0].names, vec!["* as path"]);
    }

    #[test]
    fn export_default_function() {
        let pf = parse_ts_src("export default function main() {}");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "main");
        assert!(pf.symbols[0].is_exported);
    }

    #[test]
    fn method_parent_set_on_class() {
        let pf = parse_ts_src("class Foo {\n  bar() {}\n  baz() {}\n}");
        let foo = pf.symbols.iter().find(|s| s.name == "Foo").unwrap();
        assert!(foo.parent.is_none(), "class itself should have no parent");
        let bar = pf.symbols.iter().find(|s| s.name == "bar").unwrap();
        assert_eq!(bar.parent.as_deref(), Some("Foo"));
        assert_eq!(bar.kind, SymbolKind::Method);
        let baz = pf.symbols.iter().find(|s| s.name == "baz").unwrap();
        assert_eq!(baz.parent.as_deref(), Some("Foo"));
    }

    #[test]
    fn method_parent_on_exported_class() {
        let pf = parse_ts_src("export class Service {\n  handle() {}\n}");
        let handle = pf.symbols.iter().find(|s| s.name == "handle").unwrap();
        assert_eq!(handle.parent.as_deref(), Some("Service"));
        assert!(handle.is_exported);
    }

    #[test]
    fn method_parent_on_default_export_class() {
        let pf = parse_ts_src("export default class Router {\n  route() {}\n}");
        let route = pf.symbols.iter().find(|s| s.name == "route").unwrap();
        assert_eq!(route.parent.as_deref(), Some("Router"));
    }

    #[test]
    fn standalone_function_has_no_parent() {
        let pf = parse_ts_src("function standalone() {}\nconst arrow = () => {};");
        for sym in &pf.symbols {
            assert!(sym.parent.is_none(), "{} should have no parent", sym.name);
        }
    }

    #[test]
    fn interface_and_enum_have_no_parent() {
        let pf = parse_ts_src("interface Foo { x: number }\nenum Color { Red }");
        for sym in &pf.symbols {
            assert!(sym.parent.is_none(), "{} should have no parent", sym.name);
        }
    }
}
