use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, SymbolKind};
use crate::ir::{IRParam, IRImport, IRParsedFile, ClassKind, Visibility};
use super::common::{field_text, make_symbol, ir_method, ir_class, ir_module, ir_parsed_file, node_text};
use super::LanguageAdapter;

pub struct JavaAdapter;

impl LanguageAdapter for JavaAdapter {
    fn language(&self) -> &str { "java" }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

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
                symbols.push(make_sym(name.clone(), SymbolKind::Class, &child, lines, src, is_pub));
                if let Some(body) = child.child_by_field_name("body") {
                    extract_members(&body, src, lines, symbols, &name);
                }
            }
            "interface_declaration" => {
                let name = field_text(&child, "name", src);
                symbols.push(make_sym(name.clone(), SymbolKind::Interface, &child, lines, src, has_modifier(&child, src, "public")));
                if let Some(body) = child.child_by_field_name("body") {
                    extract_members(&body, src, lines, symbols, &name);
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

fn extract_members(body: &Node, src: &[u8], lines: &[&str], symbols: &mut Vec<ParsedSymbol>, class_name: &str) {
    for i in 0..body.child_count() {
        let child = body.child(i).unwrap();
        match child.kind() {
            "method_declaration" | "constructor_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    let mut sym = make_sym(name, SymbolKind::Method, &child, lines, src, has_modifier(&child, src, "public"));
                    sym.parent = Some(class_name.to_string());
                    symbols.push(sym);
                }
            }
            "field_declaration" => {
                if has_modifier(&child, src, "static") && has_modifier(&child, src, "final")
                    && let Some(declarator) = find_child_kind(&child, "variable_declarator") {
                        let name = field_text(&declarator, "name", src);
                        if !name.is_empty() {
                            symbols.push(make_sym(name, SymbolKind::Const, &child, lines, src, has_modifier(&child, src, "public")));
                        }
                    }
            }
            _ => {}
        }
    }
}

fn make_sym(name: String, kind: SymbolKind, node: &Node, lines: &[&str], src: &[u8], is_exported: bool) -> ParsedSymbol {
    make_symbol(name, kind, node, lines, is_exported, extract_javadoc(node, src))
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

/// Parse Java source into IR.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into()).expect("java");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return IRParsedFile { file_path: file_path.into(), language: "java".into(), ..Default::default() },
    };
    let _lines: Vec<&str> = source.lines().collect();
    let root = tree.root_node();
    let src = source.as_bytes();

    let functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let constants = Vec::new();

    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        match child.kind() {
            "class_declaration" | "interface_declaration" | "enum_declaration" => {
                let name = field_text(&child, "name", src);
                let kind = match child.kind() {
                    "interface_declaration" => ClassKind::Interface,
                    "enum_declaration" => ClassKind::Enum,
                    _ => ClassKind::Class,
                };
                let is_pub = has_modifier(&child, src, "public");
                let mut class = ir_class(name, &child, kind, is_pub, extract_javadoc(&child, src), collect_java_annotations(&child, src));

                // Extract implements/extends
                if let Some(sc) = child.child_by_field_name("superclass") {
                    class.extends = Some(sc.utf8_text(src).unwrap_or_default().to_string());
                }
                if let Some(ifaces) = child.child_by_field_name("interfaces") {
                    class.implements = ifaces.utf8_text(src).unwrap_or_default()
                        .split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }

                // Extract methods from body
                if let Some(body) = child.child_by_field_name("body") {
                    for j in 0..body.child_count() {
                        if let Some(member) = body.child(j)
                            && (member.kind() == "method_declaration" || member.kind() == "constructor_declaration") {
                                let mname = field_text(&member, "name", src);
                                let mparams = extract_java_params(&member, src);
                                let ret = field_text(&member, "type", src);
                                let is_static = has_modifier(&member, src, "static");
                                let vis = if has_modifier(&member, src, "public") { Visibility::Public }
                                    else if has_modifier(&member, src, "private") { Visibility::Private }
                                    else if has_modifier(&member, src, "protected") { Visibility::Protected }
                                    else { Visibility::Internal };
                                class.methods.push(ir_method(
                                    mname, &member, vis == Visibility::Public, false, is_static,
                                    mparams, if ret.is_empty() { None } else { Some(ret) },
                                    extract_javadoc(&member, src),
                                    collect_java_annotations(&member, src), vis, &node_text(&member, src),
                                ));
                            }
                    }
                }
                classes.push(class);
            }
            "import_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let path = text.trim_start_matches("import ").trim_end_matches(';').trim();
                let name = path.rsplit('.').next().unwrap_or(path).to_string();
                imports.push(IRImport { source: path.to_string(), names: vec![name], is_reexport: false });
            }
            _ => {}
        }
    }

    let is_test = file_path.contains("test") || file_path.contains("Test");
    let module = ir_module(file_path, "java", functions, constants, imports, is_test);
    ir_parsed_file(file_path, "java", module, classes)
}

fn extract_java_params(node: &Node, src: &[u8]) -> Vec<IRParam> {
    let mut params = Vec::new();
    if let Some(param_list) = node.child_by_field_name("parameters") {
        for i in 0..param_list.child_count() {
            if let Some(p) = param_list.child(i)
                && (p.kind() == "formal_parameter" || p.kind() == "spread_parameter") {
                    let ptype = field_text(&p, "type", src);
                    let pname = field_text(&p, "name", src);
                    params.push(IRParam {
                        name: pname,
                        type_: if ptype.is_empty() { None } else { Some(ptype) },
                        ..Default::default()
                    });
                }
        }
    }
    params
}

fn collect_java_annotations(node: &Node, src: &[u8]) -> Vec<String> {
    let mut annots = Vec::new();
    let mut prev = node.prev_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "marker_annotation" || sib.kind() == "annotation" {
            annots.push(sib.utf8_text(src).unwrap_or_default().trim().to_string());
        } else if sib.kind() != "line_comment" && sib.kind() != "block_comment" {
            break;
        }
        prev = sib.prev_sibling();
    }
    annots.reverse();
    annots
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

    #[test]
    fn method_parent_set_on_class() {
        let pf = parse("public class Svc {\n  public void run() {}\n  private int calc() { return 0; }\n}");
        let svc = pf.symbols.iter().find(|s| s.name == "Svc").unwrap();
        assert!(svc.parent.is_none(), "class should have no parent");
        let run = pf.symbols.iter().find(|s| s.name == "run").unwrap();
        assert_eq!(run.parent.as_deref(), Some("Svc"));
        assert_eq!(run.kind, SymbolKind::Method);
        let calc = pf.symbols.iter().find(|s| s.name == "calc").unwrap();
        assert_eq!(calc.parent.as_deref(), Some("Svc"));
    }

    #[test]
    fn method_parent_set_on_interface() {
        let pf = parse("public interface Handler {\n  void handle();\n}");
        let methods: Vec<_> = pf.symbols.iter().filter(|s| s.kind == SymbolKind::Method).collect();
        if !methods.is_empty() {
            assert_eq!(methods[0].parent.as_deref(), Some("Handler"));
        }
    }

    #[test]
    fn enum_has_no_parent() {
        let pf = parse("public enum Color { RED, GREEN }");
        assert!(pf.symbols[0].parent.is_none());
    }

    // ── IR Tests ──────────────────────────────────────────────────────

    fn parse_ir(src: &str) -> IRParsedFile { parse_to_ir(src, "Test.java") }

    #[test]
    fn ir_class_with_methods() {
        let pf = parse_ir("public class Svc {\n  public String getName(int id) { return null; }\n}");
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].base.name, "Svc");
        assert_eq!(pf.classes[0].methods.len(), 1);
        assert_eq!(pf.classes[0].methods[0].base.name, "getName");
        assert_eq!(pf.classes[0].methods[0].params.len(), 1);
        assert_eq!(pf.classes[0].methods[0].params[0].type_, Some("int".into()));
        assert_eq!(pf.classes[0].methods[0].return_type, Some("String".into()));
    }

    #[test]
    fn ir_interface() {
        let pf = parse_ir("public interface Handler { void handle(String input); }");
        assert_eq!(pf.classes[0].class_kind, ClassKind::Interface);
    }

    #[test]
    fn ir_enum() {
        let pf = parse_ir("public enum Color { RED, GREEN, BLUE }");
        assert_eq!(pf.classes[0].class_kind, ClassKind::Enum);
    }

    #[test]
    fn ir_imports() {
        let pf = parse_ir("import java.util.List;\npublic class X {}");
        assert!(pf.modules[0].imports.len() >= 1);
        assert_eq!(pf.modules[0].imports[0].source, "java.util.List");
    }
}
