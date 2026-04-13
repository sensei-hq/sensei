use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, SymbolKind};
use super::LanguageAdapter;

pub struct JavaAdapter;

impl LanguageAdapter for JavaAdapter {
    fn language(&self) -> &str { "java" }
    fn extensions(&self) -> &[&str] { &[".java"] }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let mut parser = Parser::new();
        let lang = tree_sitter_java::LANGUAGE;
        parser.set_language(&lang.into()).expect("failed to set java language");

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
            language: "java".to_string(),
            symbols,
            edges: vec![],
            imports,
        }
    }
}

fn empty(path: &str) -> ParsedFile {
    ParsedFile { file_path: path.into(), language: "java".into(), symbols: vec![], edges: vec![], imports: vec![] }
}

fn walk(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "class_declaration" => {
                let name = field_text(&child, "name", src);
                let is_pub = has_modifier(&child, src, "public");
                symbols.push(make_sym(name, SymbolKind::Class, &child, lines, src, is_pub));
                if let Some(body) = child.child_by_field_name("body") {
                    extract_members(&body, src, lines, symbols);
                }
            }
            "interface_declaration" => {
                let name = field_text(&child, "name", src);
                symbols.push(make_sym(name, SymbolKind::Interface, &child, lines, src, has_modifier(&child, src, "public")));
                if let Some(body) = child.child_by_field_name("body") {
                    extract_members(&body, src, lines, symbols);
                }
            }
            "enum_declaration" => {
                let name = field_text(&child, "name", src);
                symbols.push(make_sym(name, SymbolKind::Enum, &child, lines, src, has_modifier(&child, src, "public")));
            }
            "import_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let path = text.trim_start_matches("import ")
                    .trim_start_matches("static ")
                    .trim_end_matches(';').trim().to_string();
                let name = path.rsplit('.').next().unwrap_or(&path).to_string();
                imports.push(ParsedImport { target_path: path, names: vec![name] });
            }
            _ => {}
        }
    }
}

fn extract_members(body: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>) {
    for i in 0..body.child_count() {
        let child = body.child(i).unwrap();
        match child.kind() {
            "method_declaration" | "constructor_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    symbols.push(make_sym(name, SymbolKind::Method, &child, lines, src, has_modifier(&child, src, "public")));
                }
            }
            "field_declaration" => {
                // static final fields → constants
                if has_modifier(&child, src, "static") && has_modifier(&child, src, "final") {
                    if let Some(declarator) = find_child_kind(&child, "variable_declarator") {
                        let name = field_text(&declarator, "name", src);
                        if !name.is_empty() {
                            symbols.push(make_sym(name, SymbolKind::Const, &child, lines, src, has_modifier(&child, src, "public")));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn make_sym(name: String, kind: SymbolKind, node: &Node, lines: &[&str], src: &[u8], is_exported: bool) -> ParsedSymbol {
    ParsedSymbol {
        name,
        kind,
        signature: lines.get(node.start_position().row).map(|l| l.trim().to_string()),
        docstring: extract_javadoc(node, src),
        line_start: node.start_position().row as u32 + 1,
        line_end: node.end_position().row as u32 + 1,
        is_exported,
    }
}

fn extract_javadoc(node: &Node, src: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "block_comment" { return None; }
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

fn has_modifier(node: &Node, src: &[u8], keyword: &str) -> bool {
    for i in 0..node.child_count() {
        let c = node.child(i).unwrap();
        if c.kind() == "modifiers" {
            let text = c.utf8_text(src).unwrap_or_default();
            if text.contains(keyword) { return true; }
        }
    }
    false
}

fn find_child_kind<'a>(node: &'a Node, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        let c = node.child(i).unwrap();
        if c.kind() == kind { return Some(c); }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile { JavaAdapter.parse(src, "Test.java") }

    #[test]
    fn parses_class() {
        let pf = parse("public class Foo { }");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "Foo");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Class);
        assert!(pf.symbols[0].is_exported);
    }

    #[test]
    fn parses_interface() {
        let pf = parse("public interface Bar { void doIt(); }");
        assert!(pf.symbols.len() >= 1);
        assert_eq!(pf.symbols[0].kind, SymbolKind::Interface);
    }

    #[test]
    fn parses_methods() {
        let pf = parse("public class Svc {\n  public void run() {}\n  private int calc() { return 0; }\n}");
        let methods: Vec<_> = pf.symbols.iter().filter(|s| s.kind == SymbolKind::Method).collect();
        assert_eq!(methods.len(), 2);
        assert!(methods[0].is_exported); // public
        assert!(!methods[1].is_exported); // private
    }

    #[test]
    fn parses_enum() {
        let pf = parse("public enum Color { RED, GREEN, BLUE }");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Enum);
    }

    #[test]
    fn parses_imports() {
        let pf = parse("import java.util.List;\nimport java.util.Optional;\npublic class X {}");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "java.util.List");
        assert_eq!(pf.imports[0].names, vec!["List"]);
    }

    #[test]
    fn parses_javadoc() {
        let pf = parse("/** Does stuff. */\npublic class Foo {}");
        assert_eq!(pf.symbols[0].docstring, Some("Does stuff.".to_string()));
    }

    #[test]
    fn static_final_constant() {
        let pf = parse("public class C {\n  public static final int MAX = 100;\n}");
        let consts: Vec<_> = pf.symbols.iter().filter(|s| s.kind == SymbolKind::Const).collect();
        assert_eq!(consts.len(), 1);
        assert_eq!(consts[0].name, "MAX");
    }
}
