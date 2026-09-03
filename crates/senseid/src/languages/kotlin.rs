use super::LanguageAdapter;
use super::common::{
    ir_class, ir_function, ir_method, ir_module, ir_parsed_file, make_symbol, node_text,
};
use crate::ir::{ClassKind, IRImport, IRParsedFile, Visibility};
use crate::types::{ParsedFile, ParsedImport, ParsedSymbol, SymbolKind};
use tree_sitter::{Language, Node, Parser};

unsafe extern "C" {
    fn tree_sitter_kotlin() -> Language;
}

pub struct KotlinAdapter;

impl LanguageAdapter for KotlinAdapter {
    fn supports_fqn(&self) -> bool {
        true
    }

    fn fqn_output(
        &self,
        _abs_path: &str,
        _rel_path: &str,
        content: &str,
    ) -> Option<super::fqn::FqnFileOutput> {
        // Source-only, like Java: the package comes from the in-source
        // `package` header, so no manifest walk is needed.
        Some(kotlin_fqn::produce_fqns(content))
    }

    fn extensions(&self) -> &[&'static str] {
        &[".kt", ".kts"]
    }

    fn language(&self) -> &str {
        "kotlin"
    }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

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
    ParsedFile {
        file_path: path.into(),
        language: "kotlin".into(),
        symbols: vec![],
        edges: vec![],
        imports: vec![],
    }
}

fn walk(
    node: &Node,
    src: &[u8],
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    imports: &mut Vec<ParsedImport>,
) {
    walk_with_parent(node, src, lines, symbols, imports, None);
}

fn walk_with_parent(
    node: &Node,
    src: &[u8],
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    imports: &mut Vec<ParsedImport>,
    class_name: Option<&str>,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_declaration" => {
                let name = find_name(&child, src);
                if name.is_empty() {
                    continue;
                }
                let is_pub =
                    !has_modifier(&child, src, "private") && !has_modifier(&child, src, "internal");
                let kind =
                    if class_name.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                let mut sym = make_sym(name, kind, &child, lines, src, is_pub);
                sym.parent = class_name.map(|s| s.to_string());
                symbols.push(sym);
            }
            "class_declaration" => {
                let name = find_name(&child, src);
                if name.is_empty() {
                    continue;
                }
                let kind = if has_keyword(&child, src, "interface") {
                    SymbolKind::Interface
                } else if has_modifier(&child, src, "data") {
                    SymbolKind::Struct
                } else if has_modifier(&child, src, "enum") {
                    SymbolKind::Enum
                } else {
                    SymbolKind::Class
                };
                let is_pub =
                    !has_modifier(&child, src, "private") && !has_modifier(&child, src, "internal");
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
                    symbols.push(make_sym(
                        name.clone(),
                        SymbolKind::Class,
                        &child,
                        lines,
                        src,
                        true,
                    ));
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
                    symbols.push(make_sym(
                        name,
                        SymbolKind::Interface,
                        &child,
                        lines,
                        src,
                        !has_modifier(&child, src, "private"),
                    ));
                }
            }
            "property_declaration" => {
                let name = find_property_name(&child, src);
                if !name.is_empty() && class_name.is_none() {
                    symbols.push(make_sym(
                        name,
                        SymbolKind::Const,
                        &child,
                        lines,
                        src,
                        !has_modifier(&child, src, "private"),
                    ));
                }
            }
            "import_header" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let target =
                    text.strip_prefix("import").map(|s| s.trim().to_string()).unwrap_or_default();
                let clean = target.strip_suffix(".*").unwrap_or(&target).to_string();
                if !clean.is_empty() {
                    let last = clean.rsplit('.').next().unwrap_or("").to_string();
                    imports.push(ParsedImport {
                        target_path: clean,
                        names: if last.is_empty() { vec![] } else { vec![last] },
                    });
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
            if text == keyword {
                return true;
            }
        }
    }
    false
}

fn has_modifier(node: &Node, src: &[u8], modifier: &str) -> bool {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        let k = child.kind();
        if k == "modifiers"
            || k == "visibility_modifier"
            || k == "class_modifier"
            || k == "inheritance_modifier"
            || k == "member_modifier"
        {
            let text = child.utf8_text(src).unwrap_or_default();
            if text.contains(modifier) {
                return true;
            }
            // Recurse into modifiers container
            if has_modifier(&child, src, modifier) {
                return true;
            }
        }
    }
    false
}

fn make_sym(
    name: String,
    kind: SymbolKind,
    node: &Node,
    lines: &[&str],
    src: &[u8],
    is_exported: bool,
) -> ParsedSymbol {
    make_symbol(name, kind, node, lines, is_exported, extract_kdoc(node, src))
}

fn extract_kdoc(node: &Node, src: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "multiline_comment" {
        return None;
    }
    let text = prev.utf8_text(src).ok()?;
    if !text.starts_with("/**") {
        return None;
    }
    let inner = text.trim_start_matches("/**").trim_end_matches("*/").trim();
    let cleaned: Vec<&str> = inner
        .lines()
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

/// Parse Kotlin source into IR.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let mut parser = Parser::new();
    let lang = unsafe { tree_sitter_kotlin() };
    parser.set_language(&lang).expect("kotlin");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return IRParsedFile {
                file_path: file_path.into(),
                language: "kotlin".into(),
                ..Default::default()
            };
        }
    };
    let lines: Vec<&str> = source.lines().collect();
    let root = tree.root_node();
    let src = source.as_bytes();
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let constants = Vec::new();
    // Kotlin top-level: functions, classes, objects, imports
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        match child.kind() {
            "function_declaration" => {
                let name = find_name(&child, src);
                if name.is_empty() {
                    continue;
                }
                let is_pub = !has_modifier(&child, src, "private");
                functions.push(ir_function(
                    name,
                    &child,
                    &lines,
                    is_pub,
                    node_text(&child, src).contains("suspend "),
                    Vec::new(),
                    None,
                    extract_kdoc(&child, src),
                    Vec::new(),
                    &node_text(&child, src),
                ));
            }
            "class_declaration" | "object_declaration" => {
                let name = find_name(&child, src);
                let kind = ClassKind::Class;
                let is_pub = !has_modifier(&child, src, "private");
                let mut class =
                    ir_class(name, &child, kind, is_pub, extract_kdoc(&child, src), Vec::new());
                // Walk class body — Kotlin uses "class_body" child, not field name
                for c in 0..child.child_count() {
                    let cc = child.child(c).unwrap();
                    if cc.kind() != "class_body" {
                        continue;
                    }
                    for j in 0..cc.child_count() {
                        if let Some(m) = cc.child(j)
                            && m.kind() == "function_declaration"
                        {
                            let mname = find_name(&m, src);
                            class.methods.push(ir_method(
                                mname,
                                &m,
                                !has_modifier(&m, src, "private"),
                                node_text(&m, src).contains("suspend "),
                                false,
                                Vec::new(),
                                None,
                                extract_kdoc(&m, src),
                                Vec::new(),
                                Visibility::Public,
                                &node_text(&m, src),
                            ));
                        }
                    }
                }
                classes.push(class);
            }
            "import_header" | "import_directive" => {
                let text = node_text(&child, src);
                let path = text.trim_start_matches("import ").trim();
                let name = path.rsplit('.').next().unwrap_or(path).to_string();
                imports.push(IRImport {
                    source: path.into(),
                    names: vec![name],
                    is_reexport: false,
                });
            }
            _ => {
                // Walk deeper for nested imports
                for j in 0..child.child_count() {
                    if let Some(c) = child.child(j)
                        && c.kind() == "import_header"
                    {
                        let text = node_text(&c, src);
                        let path = text.trim_start_matches("import ").trim();
                        let name = path.rsplit('.').next().unwrap_or(path).to_string();
                        imports.push(IRImport {
                            source: path.into(),
                            names: vec![name],
                            is_reexport: false,
                        });
                    }
                }
            }
        }
    }
    let module =
        ir_module(file_path, "kotlin", functions, constants, imports, file_path.contains("Test"));
    ir_parsed_file(file_path, "kotlin", module, classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        KotlinAdapter.parse(src, "test.kt")
    }

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

    fn parse_ir(src: &str) -> IRParsedFile {
        parse_to_ir(src, "Test.kt")
    }

    #[test]
    fn ir_class() {
        let pf = parse_ir("class Dog {\n    fun bark(): String = \"woof\"\n}");
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].base.name, "Dog");
        assert!(!pf.classes[0].methods.is_empty());
    }
}

/// Kotlin FQN production.
///
/// Modelled on `java_fqn` because the shape is the same — a `package` header, an
/// import list, and types whose members anchor to them — and deliberately NOT a
/// copy of its walk: Kotlin's grammar names things differently
/// (`package_header`/`import_header` vs `package_declaration`, plus
/// `object_declaration` which Java has no equivalent of).
///
/// Exists because FQN support is required, not optional. Without it Kotlin
/// symbols were created on the bare-name path only, so an fqn lookup could never
/// find them WHILE a name lookup still matched them — the same symbol visible to
/// one mechanism and invisible to another. 3,713 Kotlin import edges had nothing
/// to resolve against.
pub(crate) mod kotlin_fqn {
    use super::super::fqn::{self, FqnDefinition, FqnFileOutput};
    use super::{Node, Parser, SymbolKind, tree_sitter_kotlin};

    const KOTLIN_LANG: &str = "kotlin";

    fn text(node: &Node, src: &[u8]) -> String {
        node.utf8_text(src).unwrap_or_default().to_string()
    }
    fn named_child_text(node: &Node, src: &[u8], kinds: &[&str]) -> Option<String> {
        for i in 0..node.child_count() {
            let c = node.child(i)?;
            if kinds.contains(&c.kind()) {
                return Some(text(&c, src));
            }
        }
        None
    }

    pub fn produce_fqns(source: &str) -> FqnFileOutput {
        let mut parser = Parser::new();
        let lang = unsafe { tree_sitter_kotlin() };
        if parser.set_language(&lang).is_err() {
            return FqnFileOutput::default();
        }
        let Some(tree) = parser.parse(source, None) else { return FqnFileOutput::default() };
        let src = source.as_bytes();
        let root = tree.root_node();

        // `package a.b.c` — no trailing semicolon, unlike Java.
        let mut package = String::new();
        for i in 0..root.child_count() {
            let Some(child) = root.child(i) else { continue };
            if child.kind() == "package_header" {
                if let Some(id) =
                    named_child_text(&child, src, &["identifier", "qualified_identifier"])
                {
                    package = id.trim().to_string();
                }
                break;
            }
        }

        let mut out =
            FqnFileOutput { package: package.clone(), module: String::new(), ..Default::default() };
        walk_top(&root, src, &package, &mut out);
        out
    }

    fn walk_top(node: &Node, src: &[u8], package: &str, out: &mut FqnFileOutput) {
        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            match child.kind() {
                "class_declaration" | "object_declaration" | "interface_declaration" => {
                    let Some(name) =
                        named_child_text(&child, src, &["type_identifier", "simple_identifier"])
                    else {
                        continue;
                    };
                    let kind = match child.kind() {
                        "interface_declaration" => SymbolKind::Interface,
                        _ => SymbolKind::Class,
                    };
                    // A top-level type anchors on the package, with no module
                    // segment — the same shape `java_fqn` produces, so a Kotlin
                    // type and a Java type in one package are addressable alike.
                    let type_fqn = fqn::item(KOTLIN_LANG, package, "", &name);
                    out.defs.push(def(&type_fqn, &name, kind, &child, None, None));
                    walk_members(&child, src, package, &name, &type_fqn, out);
                }
                "function_declaration" | "property_declaration" => {
                    if let Some(name) = named_child_text(
                        &child,
                        src,
                        &["simple_identifier", "variable_declaration"],
                    ) {
                        let name = name.split(':').next().unwrap_or(&name).trim().to_string();
                        let k = if child.kind() == "function_declaration" {
                            SymbolKind::Function
                        } else {
                            SymbolKind::Const
                        };
                        let f = fqn::item(KOTLIN_LANG, package, "", &name);
                        out.defs.push(def(&f, &name, k, &child, None, None));
                    }
                }
                // Kotlin allows declarations nested under file-level constructs;
                // recurse so they are not silently dropped.
                _ => walk_top(&child, src, package, out),
            }
        }
    }

    fn walk_members(
        type_node: &Node,
        src: &[u8],
        package: &str,
        type_name: &str,
        type_fqn: &str,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..type_node.child_count() {
            let Some(body) = type_node.child(i) else { continue };
            if body.kind() != "class_body" {
                continue;
            }
            collect_members(&body, src, package, type_name, type_fqn, out);
        }
    }

    /// Collect a type's members, DESCENDING THROUGH `ERROR` nodes.
    ///
    /// tree-sitter-kotlin's error recovery nests declarations: the valid
    /// one-liner `class Loose { fun m() {} }` parses as
    /// `class_body > ERROR > function_declaration`, while the multi-line form
    /// puts `function_declaration` directly under `class_body`. A
    /// direct-children-only walk therefore silently loses every member of any
    /// file the grammar stumbles on — so recursion here is correctness, not
    /// thoroughness.
    fn collect_members(
        body: &Node,
        src: &[u8],
        package: &str,
        type_name: &str,
        type_fqn: &str,
        out: &mut FqnFileOutput,
    ) {
        for j in 0..body.child_count() {
            let Some(m) = body.child(j) else { continue };
            let (kind, want) = match m.kind() {
                "function_declaration" => (SymbolKind::Method, true),
                // No `Property` variant exists; a Kotlin `val`/`var` member is
                // closest to Const, which is also what the top-level branch uses.
                "property_declaration" => (SymbolKind::Const, true),
                // Recovery wrapper — the declaration is inside it.
                "ERROR" => {
                    collect_members(&m, src, package, type_name, type_fqn, out);
                    continue;
                }
                _ => (SymbolKind::Method, false),
            };
            if !want {
                continue;
            }
            {
                let Some(raw) =
                    named_child_text(&m, src, &["simple_identifier", "variable_declaration"])
                else {
                    continue;
                };
                let name = raw.split(':').next().unwrap_or(&raw).trim().to_string();
                if name.is_empty() {
                    continue;
                }
                let f = fqn::method(KOTLIN_LANG, package, "", type_name, &name);
                out.defs.push(def(
                    &f,
                    &name,
                    kind,
                    &m,
                    Some(type_name.to_string()),
                    Some(type_fqn.to_string()),
                ));
            }
        }
    }

    fn def(
        fqn_str: &str,
        name: &str,
        kind: SymbolKind,
        node: &Node,
        parent_type: Option<String>,
        parent_fqn: Option<String>,
    ) -> FqnDefinition {
        FqnDefinition {
            fqn: fqn_str.to_string(),
            name: name.to_string(),
            kind,
            line_start: node.start_position().row as u32 + 1,
            line_end: node.end_position().row as u32 + 1,
            is_exported: true,
            signature: None,
            docstring: None,
            parent_type,
            parent_fqn,
        }
    }
}

#[cfg(test)]
mod kotlin_fqn_tests {
    use super::kotlin_fqn::produce_fqns;
    /// Kotlin FQNs anchor on the in-source `package` header and use the SAME
    /// shape as Java, so a Kotlin type and a Java type in one package are
    /// addressable alike — which matters because JVM projects mix them.
    ///
    /// Breaking mutation: stop reading `package_header` — every fqn loses its
    /// package segment and stops matching what an import resolves to.
    #[test]
    fn kotlin_types_and_members_anchor_on_the_package_header() {
        let out = produce_fqns(
            "package com.acme.svc\n\
             \n\
             class Widget {\n\
                 fun render(): String { return \"x\" }\n\
                 val size: Int = 3\n\
             }\n\
             \n\
             interface Sink { fun accept(v: Int) }\n\
             \n\
             fun helper(): Int { return 1 }\n",
        );
        assert_eq!(out.package, "com.acme.svc", "package comes from the header, not the path");

        let fqns: Vec<&str> = out.defs.iter().map(|d| d.fqn.as_str()).collect();
        assert!(fqns.contains(&"kotlin·com.acme.svc·Widget"), "type: {fqns:?}");
        assert!(fqns.contains(&"kotlin·com.acme.svc·Widget·render"), "method: {fqns:?}");
        assert!(fqns.contains(&"kotlin·com.acme.svc·Widget·size"), "property: {fqns:?}");
        assert!(fqns.contains(&"kotlin·com.acme.svc·Sink"), "interface: {fqns:?}");
        assert!(fqns.contains(&"kotlin·com.acme.svc·helper"), "top-level fn: {fqns:?}");

        // A member records its owning type, so the graph nests method under type
        // rather than dangling it at file level.
        let render = out.defs.iter().find(|d| d.name == "render").expect("render");
        assert_eq!(render.parent_type.as_deref(), Some("Widget"));
        assert_eq!(render.parent_fqn.as_deref(), Some("kotlin·com.acme.svc·Widget"));
    }

    /// A file with NO package header still produces fqns — Kotlin allows it, and
    /// returning nothing would put the whole file back on the bare-name path.
    #[test]
    fn a_package_less_file_still_produces_fqns() {
        let out = produce_fqns("class Loose { fun m() {} }\n");
        assert_eq!(out.package, "");
        let fqns: Vec<&str> = out.defs.iter().map(|d| d.fqn.as_str()).collect();
        assert!(fqns.contains(&"kotlin·Loose"), "{fqns:?}");
        assert!(fqns.contains(&"kotlin·Loose·m"), "{fqns:?}");
    }

    /// `object` is Kotlin-specific (a singleton) and has no Java equivalent, so
    /// a straight copy of `java_fqn`'s walk would have dropped it silently.
    #[test]
    fn an_object_declaration_is_not_dropped() {
        let out = produce_fqns("package p\nobject Registry { fun lookup() {} }\n");
        let fqns: Vec<&str> = out.defs.iter().map(|d| d.fqn.as_str()).collect();
        assert!(fqns.contains(&"kotlin·p·Registry"), "{fqns:?}");
        assert!(fqns.contains(&"kotlin·p·Registry·lookup"), "{fqns:?}");
    }
}
