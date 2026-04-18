use super::common::{field_text, make_symbol};
use tree_sitter::{Language, Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, SymbolKind};
use super::LanguageAdapter;

unsafe extern "C" {
    fn tree_sitter_kotlin() -> Language;
}

pub struct KotlinAdapter;

impl LanguageAdapter for KotlinAdapter {
    fn language(&self) -> &str { "kotlin" }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let mut parser = Parser::new();
        let lang = unsafe { tree_sitter_kotlin() };
        parser.set_language(&lang).expect("failed to set kotlin language");

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
            language: "kotlin".to_string(),
            symbols,
            edges: vec![],
            imports,
        }
    }
}

fn empty(path: &str) -> ParsedFile {
    ParsedFile { file_path: path.into(), language: "kotlin".into(), symbols: vec![], edges: vec![], imports: vec![] }
}

fn walk(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>) {
    walk_with_parent(node, src, lines, symbols, imports, None);
}

fn walk_with_parent(node: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, imports: &mut Vec<ParsedImport>, class_name: Option<&str>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_declaration" => {
                let name = find_name(&child, src);
                if name.is_empty() { continue; }
                let is_pub = !has_modifier(&child, src, "private") && !has_modifier(&child, src, "internal");
                let kind = if class_name.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                let mut sym = make_sym(name, kind, &child, lines, src, is_pub);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "class_declaration" => {
                let name = find_name(&child, src);
                if name.is_empty() { continue; }
                let kind = if has_keyword(&child, src, "interface") { SymbolKind::Interface }
                    else if has_modifier(&child, src, "data") { SymbolKind::Struct }
                    else if has_modifier(&child, src, "enum") { SymbolKind::Enum }
                    else { SymbolKind::Class };
                let is_pub = !has_modifier(&child, src, "private") && !has_modifier(&child, src, "internal");
                symbols.push(make_sym(name.clone(), kind, &child, lines, src, is_pub));
                for j in 0..child.child_count() {
                    let cc = child.child(j).unwrap();
                    if cc.kind() == "class_body" {
                        walk_with_parent(&cc, src, lines, symbols, imports, Some(&name));
                    }
                }
            }
            "object_declaration" => {
                let name = find_name(&child, src);
                if !name.is_empty() {
                    symbols.push(make_sym(name.clone(), SymbolKind::Class, &child, lines, src, true));
                    for j in 0..child.child_count() {
                        let cc = child.child(j).unwrap();
                        if cc.kind() == "class_body" {
                            walk_with_parent(&cc, src, lines, symbols, imports, Some(&name));
                        }
                    }
                }
            }
            "interface_declaration" => {
                let name = find_name(&child, src);
                if !name.is_empty() {
                    symbols.push(make_sym(name, SymbolKind::Interface, &child, lines, src, !has_modifier(&child, src, "private")));
                }
            }
            "property_declaration" => {
                let name = find_property_name(&child, src);
                if !name.is_empty() && class_name.is_none() {
                    symbols.push(make_sym(name, SymbolKind::Const, &child, lines, src, !has_modifier(&child, src, "private")));
                }
            }
            "import_header" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let target = text.strip_prefix("import")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                let clean = target.strip_suffix(".*").unwrap_or(&target).to_string();
                if !clean.is_empty() {
                    let last = clean.rsplit('.').next().unwrap_or("").to_string();
                    imports.push(ParsedImport { target_path: clean, names: if last.is_empty() { vec![] } else { vec![last] } });
                }
            }
            "import_list" | "source_file" => {
                walk_with_parent(&child, src, lines, symbols, imports, class_name);
            }
            _ => {}
        }
    }
}

fn find_property_name(node: &Node, src: &[u8]) -> String {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "variable_declaration" {
            return find_name(&child, src);
        }
        if child.kind() == "simple_identifier" {
            return child.utf8_text(src).unwrap_or_default().to_string();
        }
    }
    String::new()
}

fn has_keyword(node: &Node, src: &[u8], keyword: &str) -> bool {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if !child.is_named() {
            let text = child.utf8_text(src).unwrap_or_default();
            if text == keyword { return true; }
        }
    }
    false
}

fn has_modifier(node: &Node, src: &[u8], modifier: &str) -> bool {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        let k = child.kind();
        if k == "modifiers" || k == "visibility_modifier" || k == "class_modifier"
            || k == "inheritance_modifier" || k == "member_modifier"
        {
            let text = child.utf8_text(src).unwrap_or_default();
            if text.contains(modifier) { return true; }
            // Recurse into modifiers container
            if has_modifier(&child, src, modifier) { return true; }
        }
    }
    false
}

fn make_sym(name: String, kind: SymbolKind, node: &Node, lines: &[&str], src: &[u8], is_exported: bool) -> ParsedSymbol {
    make_symbol(name, kind, node, lines, is_exported, extract_kdoc(node, src))
}

fn extract_kdoc(node: &Node, src: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "multiline_comment" { return None; }
    let text = prev.utf8_text(src).ok()?;
    if !text.starts_with("/**") { return None; }
    let inner = text.trim_start_matches("/**").trim_end_matches("*/").trim();
    let cleaned: Vec<&str> = inner.lines()
        .map(|l| l.trim().trim_start_matches('*').trim())
        .filter(|l| !l.is_empty())
        .collect();
    if cleaned.is_empty() { None } else { Some(cleaned.join("\n")) }
}

fn find_name(node: &Node, src: &[u8]) -> String {
    // Kotlin grammar has no named fields — find first simple_identifier child
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "simple_identifier" || child.kind() == "type_identifier" {
            return child.utf8_text(src).unwrap_or_default().to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile { KotlinAdapter.parse(src, "test.kt") }

    #[test]
    fn kotlin_function() {
        let pf = parse("fun greet(name: String): String {\n    return \"hello $name\"\n}");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "greet");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn kotlin_class_with_methods() {
        let pf = parse("class Dog {\n    fun bark() {}\n    fun sit() {}\n}");
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Dog"));
        assert!(names.contains(&"bark"));
        assert!(names.contains(&"sit"));
    }

    #[test]
    fn kotlin_data_class() {
        let pf = parse("data class Point(val x: Int, val y: Int)");
        assert_eq!(pf.symbols[0].name, "Point");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn kotlin_interface() {
        let pf = parse("interface Drawable {\n    fun draw()\n}");
        assert_eq!(pf.symbols[0].name, "Drawable");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Interface);
    }

    #[test]
    fn kotlin_object() {
        let pf = parse("object Singleton {\n    fun instance() {}\n}");
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Singleton"));
    }

    #[test]
    fn kotlin_imports() {
        let pf = parse("import kotlin.collections.List\nimport java.io.*\nfun hello() {}");
        assert!(pf.imports.len() >= 2);
        assert_eq!(pf.imports[0].target_path, "kotlin.collections.List");
        assert_eq!(pf.imports[1].target_path, "java.io");
    }

    #[test]
    fn kotlin_suspend_function() {
        let pf = parse("suspend fun fetchData(): String { return \"\" }");
        assert_eq!(pf.symbols[0].name, "fetchData");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn kotlin_language() {
        let pf = parse("fun x() {}");
        assert_eq!(pf.language, "kotlin");
    }

    #[test]
    fn method_parent_set_on_class() {
        let pf = parse("class Dog {\n    fun bark() {}\n    fun sit() {}\n}");
        let dog = pf.symbols.iter().find(|s| s.name == "Dog").unwrap();
        assert!(dog.parent.is_none(), "class should have no parent");
        let bark = pf.symbols.iter().find(|s| s.name == "bark").unwrap();
        assert_eq!(bark.parent.as_deref(), Some("Dog"));
        assert_eq!(bark.kind, SymbolKind::Method);
        let sit = pf.symbols.iter().find(|s| s.name == "sit").unwrap();
        assert_eq!(sit.parent.as_deref(), Some("Dog"));
    }

    #[test]
    fn method_parent_set_on_object() {
        let pf = parse("object Singleton {\n    fun instance() {}\n}");
        let inst = pf.symbols.iter().find(|s| s.name == "instance").unwrap();
        assert_eq!(inst.parent.as_deref(), Some("Singleton"));
    }

    #[test]
    fn top_level_function_no_parent() {
        let pf = parse("fun greet(name: String): String {\n    return \"hello $name\"\n}");
        assert!(pf.symbols[0].parent.is_none());
    }

    #[test]
    fn data_class_no_parent() {
        let pf = parse("data class Point(val x: Int, val y: Int)");
        assert!(pf.symbols[0].parent.is_none());
    }
}
