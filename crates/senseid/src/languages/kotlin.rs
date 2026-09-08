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

    /// Emits `extends`/`implements` (or trait impls) into `relations`.
    fn emits_inheritance(&self) -> bool {
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

        let imports = collect_imports(&root, src);

        let mut out =
            FqnFileOutput { package: package.clone(), module: String::new(), ..Default::default() };
        walk_top(&root, src, &package, &imports, &mut out);
        out
    }

    /// Simple name → fully-qualified name, from this file's `import` headers.
    ///
    /// `import_header` is wrapped in an `import_list` under `source_file`
    /// (pinned in `kotlin_grammar_shapes`), so java's root-level loop finds
    /// nothing here and this descends one level.
    ///
    /// A star import carries a `wildcard_import` child and is SKIPPED: it binds
    /// no simple name, and keying it on `*` — or on the package's last segment —
    /// would invent a binding the source never made. A supertype that came in
    /// through a star import therefore resolves same-package, which is honest:
    /// the file does not say where it came from.
    fn collect_imports(root: &Node, src: &[u8]) -> std::collections::HashMap<String, String> {
        let mut imports = std::collections::HashMap::new();
        for i in 0..root.child_count() {
            let Some(list) = root.child(i) else { continue };
            if list.kind() != "import_list" {
                continue;
            }
            for j in 0..list.child_count() {
                let Some(header) = list.child(j) else { continue };
                if header.kind() != "import_header" {
                    continue;
                }
                if child_of_kind(&header, "wildcard_import").is_some() {
                    continue;
                }
                // Read the dotted path from the identifier node rather than
                // string-stripping the header text, so an alias or trailing
                // comment cannot leak into the path.
                let Some(path) =
                    named_child_text(&header, src, &["identifier", "qualified_identifier"])
                else {
                    continue;
                };
                let path = path.trim();
                if let Some(leaf) = path.rsplit('.').next().filter(|l| !l.is_empty()) {
                    imports.insert(leaf.to_string(), path.to_string());
                }
            }
        }
        imports
    }

    fn child_of_kind<'t>(node: &Node<'t>, kind: &str) -> Option<Node<'t>> {
        (0..node.child_count()).filter_map(|i| node.child(i)).find(|c| c.kind() == kind)
    }

    /// First DESCENDANT of `kind`, depth-first.
    ///
    /// A supertype's name is not a direct child of its `delegation_specifier`:
    /// a base class nests `constructor_invocation > user_type > type_identifier`
    /// and an interface nests `user_type > type_identifier`. `named_child_text`
    /// looks at direct children only, so it returned None for both and every
    /// relation was silently dropped.
    fn descendant_of_kind<'t>(node: &Node<'t>, kind: &str) -> Option<Node<'t>> {
        if node.kind() == kind {
            return Some(*node);
        }
        (0..node.child_count())
            .filter_map(|i| node.child(i))
            .find_map(|c| descendant_of_kind(&c, kind))
    }

    /// Record every call site under `caller_fqn`, naming a target ONLY when the
    /// language says so unambiguously.
    ///
    /// Kotlin emitted 0 refs before this, so ANY target named wrongly here
    /// becomes a minted node at process.rs's `OnMiss::CreateStub`. That makes
    /// this the one step in the kotlin work whose failure mode is a fresh
    /// fabrication pile rather than a rate regression, so the ladder is
    /// miss-first and deliberately incomplete:
    ///
    /// - `Type.method()` where `Type` is imported and PascalCase → the shared
    ///   `jvm::resolve_type_call`, so it lands on the key a java call already
    ///   writes for that string.
    /// - `obj.method()` on a lowercase receiver → UNRESOLVED. Naming it needs
    ///   type inference this producer does not do.
    /// - a bare `helper()` → UNRESOLVED. Kotlin has top-level functions, so
    ///   java's "an unqualified call is a method on the enclosing class" rule is
    ///   FALSE here. Resolving it wants the definition pre-pass typescript's
    ///   `#3` fix introduced; until that exists a miss is the honest answer.
    ///
    /// The call site is recorded either way, so "who calls this" and "how much
    /// of this file is call sites" both work before targets improve.
    fn collect_calls(
        node: &Node,
        src: &[u8],
        package: &str,
        caller_fqn: &str,
        imports: &std::collections::HashMap<String, String>,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            if child.kind() == "call_expression"
                && let Some((target_fqn, is_lib, target_name)) =
                    resolve_call(&child, src, package, imports)
            {
                out.refs.push(fqn::FqnReference {
                    caller_fqn: caller_fqn.to_string(),
                    caller_line: child.start_position().row as u32 + 1,
                    target_fqn,
                    target_name,
                    is_lib,
                    receiver: None,
                });
            }
            collect_calls(&child, src, package, caller_fqn, imports, out);
        }
    }

    /// The callee is a `call_expression`'s FIRST child: a `simple_identifier`
    /// when unqualified, a `navigation_expression` when qualified (both pinned
    /// in `kotlin_grammar_shapes`).
    fn resolve_call(
        call: &Node,
        src: &[u8],
        package: &str,
        imports: &std::collections::HashMap<String, String>,
    ) -> Option<(Option<String>, bool, String)> {
        let callee = call.child(0)?;
        match callee.kind() {
            // `helper()` — see above: not necessarily a member, so unresolved.
            "simple_identifier" => {
                let name = text(&callee, src);
                (!name.is_empty()).then_some((None, false, name))
            }
            "navigation_expression" => {
                let receiver = child_of_kind(&callee, "simple_identifier")?;
                let suffix = child_of_kind(&callee, "navigation_suffix")?;
                let method = text(&descendant_of_kind(&suffix, "simple_identifier")?, src);
                if method.is_empty() {
                    return None;
                }
                let recv = text(&receiver, src);
                // PascalCase AND imported ⇒ a type whose package the import
                // states outright. Anything else is a value whose type we do not
                // know, so it stays unresolved rather than being stamped onto
                // this file's package.
                let is_type = recv.chars().next().is_some_and(|c| c.is_ascii_uppercase());
                match imports.get(&recv).filter(|_| is_type) {
                    Some(fqcn) => Some(crate::languages::jvm::resolve_type_call(
                        KOTLIN_LANG,
                        fqcn,
                        &method,
                        package,
                    )),
                    None => Some((None, false, method)),
                }
            }
            _ => None,
        }
    }

    /// Emit this type's `extends` / `implements` facts.
    ///
    /// Kotlin lists the superclass and every interface in ONE
    /// `delegation_specifier` sequence — `class W : Base(), Iface` — and
    /// `child_by_field_name` is unusable here (FIELD_COUNT 0), so java's
    /// split-by-field approach cannot be ported. The discriminator is
    /// STRUCTURAL: a base class is CONSTRUCTED, so its specifier holds a
    /// `constructor_invocation`; an interface's does not. A `by` delegation
    /// (`explicit_delegation`) carries no constructor invocation either, so it
    /// classifies as interface-like, which is correct — a delegate is not a base
    /// class.
    ///
    /// Resolution is the SHARED `jvm::resolve_supertype`, so a kotlin supertype
    /// lands on the exact key a java call already writes for the same string.
    fn emit_relations(
        type_node: &Node,
        src: &[u8],
        package: &str,
        child_fqn: &str,
        imports: &std::collections::HashMap<String, String>,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..type_node.child_count() {
            let Some(spec) = type_node.child(i) else { continue };
            if spec.kind() != "delegation_specifier" {
                continue;
            }
            let Some(name_node) = descendant_of_kind(&spec, "type_identifier") else { continue };
            let name = text(&name_node, src);
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let relation = if child_of_kind(&spec, "constructor_invocation").is_some() {
                crate::types::RelationKind::Extends
            } else {
                crate::types::RelationKind::Implements
            };
            let resolved =
                crate::languages::jvm::resolve_supertype(KOTLIN_LANG, name, imports, package);
            out.relations.push(fqn::TypeRelation {
                child_fqn: child_fqn.to_string(),
                parent_fqn: resolved.as_ref().map(|(f, _)| f.clone()),
                parent_name: name.to_string(),
                is_lib: resolved.as_ref().is_some_and(|(_, l)| *l),
                relation,
            });
        }
    }

    fn walk_top(
        node: &Node,
        src: &[u8],
        package: &str,
        imports: &std::collections::HashMap<String, String>,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..node.child_count() {
            let Some(child) = node.child(i) else { continue };
            match child.kind() {
                "class_declaration" | "object_declaration" => {
                    let Some(name) =
                        named_child_text(&child, src, &["type_identifier", "simple_identifier"])
                    else {
                        continue;
                    };
                    // An interface and an enum are BOTH `class_declaration`,
                    // distinguished by an unnamed keyword child. This arm used to
                    // match `"interface_declaration"`, which is not a node kind in
                    // this grammar at all (pinned in `kotlin_grammar_shapes`), so
                    // it was DEAD and every Kotlin interface was emitted as a
                    // Class. The non-fqn walk above already tests the keyword.
                    let kind = if super::has_keyword(&child, src, "interface") {
                        SymbolKind::Interface
                    } else if super::has_keyword(&child, src, "enum") {
                        SymbolKind::Enum
                    } else {
                        SymbolKind::Class
                    };
                    // A top-level type anchors on the package, with no module
                    // segment — the same shape `java_fqn` produces, so a Kotlin
                    // type and a Java type in one package are addressable alike.
                    let type_fqn = fqn::item(KOTLIN_LANG, package, "", &name);
                    out.defs.push(def(&type_fqn, &name, kind, &child, None, None));
                    emit_relations(&child, src, package, &type_fqn, imports, out);
                    walk_members(&child, src, package, &name, &type_fqn, imports, out);
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
                _ => walk_top(&child, src, package, imports, out),
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn walk_members(
        type_node: &Node,
        src: &[u8],
        package: &str,
        type_name: &str,
        type_fqn: &str,
        imports: &std::collections::HashMap<String, String>,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..type_node.child_count() {
            let Some(body) = type_node.child(i) else { continue };
            // An ENUM's body is `enum_class_body`, not `class_body` (pinned in
            // `kotlin_grammar_shapes`), so testing only for `class_body` dropped
            // every member of every enum.
            if !matches!(body.kind(), "class_body" | "enum_class_body") {
                continue;
            }
            collect_members(&body, src, package, type_name, type_fqn, imports, out);
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
    #[allow(clippy::too_many_arguments)]
    fn collect_members(
        body: &Node,
        src: &[u8],
        package: &str,
        type_name: &str,
        type_fqn: &str,
        imports: &std::collections::HashMap<String, String>,
        out: &mut FqnFileOutput,
    ) {
        for j in 0..body.child_count() {
            let Some(m) = body.child(j) else { continue };
            let (kind, want) = match m.kind() {
                "function_declaration" => (SymbolKind::Method, true),
                // No `Property` variant exists; a Kotlin `val`/`var` member is
                // closest to Const, which is also what the top-level branch uses.
                "property_declaration" => (SymbolKind::Const, true),
                // A COMPANION OBJECT nests its members in its OWN `class_body`
                // (pinned in `kotlin_grammar_shapes`), so a direct-children walk
                // saw none of them. They are attributed to the ENCLOSING type,
                // which is how the JVM addresses them and how a
                // `Widget.create()` reference will resolve — a member with no
                // definition is precisely what `OnMiss::CreateStub` turns into a
                // phantom, which is why defs land before refs in this slice.
                "companion_object" => {
                    walk_members(&m, src, package, type_name, type_fqn, imports, out);
                    continue;
                }
                // Recovery wrapper — the declaration is inside it.
                "ERROR" => {
                    collect_members(&m, src, package, type_name, type_fqn, imports, out);
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
                collect_calls(&m, src, package, &f, imports, out);
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
            return_type: None,
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
    /// An INTERFACE is an interface, a COMPANION member is a member, and an ENUM
    /// entry is not lost — the three def gaps that make a later `Foo.create()`
    /// reference mint a phantom.
    ///
    /// All three come from matching the wrong node kind, verified in
    /// `kotlin_grammar_shapes`:
    ///
    /// - `walk_top` matched `"interface_declaration"`, which DOES NOT EXIST in
    ///   this grammar. The arm was dead, so every Kotlin interface was emitted as
    ///   `SymbolKind::Class`. The non-fqn walk already gets this right via
    ///   `has_keyword(.., "interface")`.
    /// - `collect_members` walks a type's direct `class_body` only, and a
    ///   `companion_object` nests its members in its OWN `class_body` — so
    ///   `Widget.create()` had no definition to land on. A missing def is exactly
    ///   what turns a legitimate call into a minted stub at
    ///   `OnMiss::CreateStub`, which is why defs must precede refs in this slice.
    /// - an enum's body is `enum_class_body`, not `class_body`, so its members
    ///   were dropped too.
    ///
    /// Breaking mutation: restore the `"interface_declaration"` arm and `Handler`
    /// goes back to `SymbolKind::Class`; drop the `companion_object` recursion and
    /// `create` loses its definition.
    #[test]
    fn an_interface_a_companion_member_and_an_enum_member_all_get_definitions() {
        use crate::languages::fqn::finders::def_fqn;
        use crate::types::SymbolKind;

        let out = produce_fqns(
            "package com.acme\n\
             \n\
             interface Handler {\n\
             \x20   fun handle()\n\
             }\n\
             \n\
             class Widget {\n\
             \x20   companion object {\n\
             \x20       fun create(): Widget = Widget()\n\
             \x20   }\n\
             \x20   fun go() {}\n\
             }\n\
             \n\
             enum class Colour { RED, GREEN }\n",
        );
        let kind_of = |n: &str| out.defs.iter().find(|d| d.name == n).map(|d| d.kind.clone());

        // 1. An interface is an Interface, not a Class.
        assert_eq!(
            kind_of("Handler"),
            Some(SymbolKind::Interface),
            "an interface is `class_declaration` + the `interface` keyword: {:?}",
            out.defs.iter().map(|d| (&d.name, d.kind.clone())).collect::<Vec<_>>()
        );
        assert_eq!(def_fqn(&out, "Handler"), "kotlin·com.acme·Handler");

        // 2. A companion member has a definition, nested on its OWNING type so a
        //    `Widget.create()` reference resolves to it.
        assert_eq!(
            def_fqn(&out, "create"),
            "kotlin·com.acme·Widget·create",
            "a companion member belongs to its enclosing type: {:?}",
            out.defs.iter().map(|d| &d.fqn).collect::<Vec<_>>()
        );
        // The ordinary member still works.
        assert_eq!(def_fqn(&out, "go"), "kotlin·com.acme·Widget·go");

        // 3. The enum type and its entries survive.
        assert_eq!(def_fqn(&out, "Colour"), "kotlin·com.acme·Colour");
        assert_eq!(kind_of("Colour"), Some(SymbolKind::Enum), "an enum is an Enum");
    }

    /// HERITAGE: a superclass is `Extends`, an interface is `Implements`, and the
    /// discriminator is STRUCTURAL — the parenthesised one is the base class.
    ///
    /// Kotlin puts the superclass and every interface in ONE
    /// `delegation_specifier` list: `class W : Base(), Iface`. Java splits them
    /// by field (`superclass` / `interfaces`); Kotlin cannot, because
    /// `child_by_field_name` returns None for every node in this grammar
    /// (FIELD_COUNT 0). The signal is the `constructor_invocation` — you call a
    /// base class's constructor, never an interface's. Pinned in
    /// `kotlin_grammar_shapes`.
    ///
    /// Resolution goes through the SHARED `jvm::resolve_supertype`, so a kotlin
    /// supertype lands on the exact key a java call already writes: a
    /// third-party one becomes a `lib·` node, a first-party one keeps its
    /// package. That sharing is why `09fd073b` lifted the rule out of java
    /// instead of copying it.
    ///
    /// Breaking mutation: treat every `delegation_specifier` as Extends and the
    /// interface becomes a base class; drop the `jvm::` call and
    /// `org.springframework` goes back to a fabricated first-party kotlin node.
    #[test]
    fn a_superclass_extends_and_an_interface_implements_via_the_shared_jvm_rule() {
        use crate::languages::fqn::finders::rel_to;
        use crate::types::RelationKind;

        let out = produce_fqns(
            "package com.acme.svc\n\
             \n\
             import com.acme.core.BaseService\n\
             import org.springframework.web.servlet.HandlerInterceptor\n\
             \n\
             class Widget : BaseService(), HandlerInterceptor, Runnable {\n\
             \x20   fun go() {}\n\
             }\n",
        );

        // The parenthesised specifier is the BASE CLASS.
        let base = rel_to(&out, "BaseService");
        assert_eq!(base.relation, RelationKind::Extends, "Base() is a superclass");
        assert_eq!(base.child_fqn, "kotlin·com.acme.svc·Widget");
        assert_eq!(
            base.parent_fqn.as_deref(),
            Some("kotlin·com.acme.core·BaseService"),
            "a first-party supertype keeps its own package"
        );
        assert!(!base.is_lib);

        // A bare specifier is an INTERFACE, and a third-party one is a lib node —
        // the same answer the java call path gives for the same string.
        let spring = rel_to(&out, "HandlerInterceptor");
        assert_eq!(spring.relation, RelationKind::Implements, "no parens ⇒ interface");
        assert!(spring.is_lib, "org.springframework is third-party: {:?}", spring.parent_fqn);
        assert!(spring.parent_fqn.as_deref().is_some_and(|f| f.starts_with("lib·")));

        // An UNIMPORTED interface resolves same-package, which is the language rule.
        let run = rel_to(&out, "Runnable");
        assert_eq!(run.relation, RelationKind::Implements);
        assert_eq!(run.parent_fqn.as_deref(), Some("kotlin·com.acme.svc·Runnable"));

        assert_eq!(out.relations.len(), 3, "one extends + two implements: {:?}", out.relations);
    }

    /// CALLS: a call site is always recorded; a target is named only when the
    /// language says so unambiguously, and is `None` otherwise.
    ///
    /// This is the one step in the kotlin work whose failure mode is a NEW
    /// FABRICATION PILE rather than a rate regression. Kotlin emits 0 refs
    /// today, so anything this names wrongly becomes a minted node at
    /// process.rs's `OnMiss::CreateStub` — the exact defect the whole slice
    /// exists to remove. So the ladder is MISS-FIRST:
    ///
    /// - `Type.method()` on an IMPORTED PascalCase receiver → resolved through
    ///   the shared `jvm::resolve_type_call`, so it lands on the key a java call
    ///   already writes (third-party ⇒ `lib·`, first-party ⇒ its own package).
    /// - `obj.method()` on a lowercase receiver → UNRESOLVED. Naming it would
    ///   require type inference this producer does not do, and java's
    ///   `bindings` shortcut (`val x = Type()`) is not ported yet.
    /// - a bare `helper()` → UNRESOLVED. Kotlin has top-level functions, so
    ///   java's "an unqualified call is a method on the enclosing class" rule is
    ///   simply false here. Resolving it needs the definition pre-pass that
    ///   typescript's `#3` fix introduced; until then a miss is the honest
    ///   answer.
    ///
    /// The caller is the enclosing member, so "who calls this" works even when
    /// the target is unknown.
    ///
    /// Breaking mutation: resolve the lowercase-receiver or bare arm to
    /// `fqn::method(KOTLIN_LANG, package, "", enclosing_type, name)` and every
    /// such call site mints a phantom.
    #[test]
    fn a_call_is_recorded_and_only_an_unambiguous_target_is_named() {
        use crate::languages::fqn::finders::ref_to;

        let out = produce_fqns(
            "package com.acme.svc\n\
             \n\
             import com.acme.core.Helper\n\
             import org.mockito.Mockito\n\
             \n\
             class Widget {\n\
             \x20   fun go(obj: Thing) {\n\
             \x20       Helper.run()\n\
             \x20       Mockito.mock()\n\
             \x20       obj.doThing()\n\
             \x20       bare()\n\
             \x20   }\n\
             }\n",
        );

        // A first-party imported type resolves to its own package.
        let helper = ref_to(&out, "run");
        assert_eq!(
            helper.target_fqn.as_deref(),
            Some("kotlin·com.acme.core·Helper·run"),
            "an imported first-party type keeps its package"
        );
        assert!(!helper.is_lib);
        assert_eq!(helper.caller_fqn, "kotlin·com.acme.svc·Widget·go", "caller is the member");

        // A third-party one becomes a lib node — same answer java gives.
        let mockito = ref_to(&out, "mock");
        assert!(mockito.is_lib, "org.mockito is third-party: {:?}", mockito.target_fqn);
        assert!(mockito.target_fqn.as_deref().is_some_and(|f| f.starts_with("lib·")));

        // An UNKNOWN receiver is recorded but NOT named.
        let unknown = ref_to(&out, "doThing");
        assert_eq!(unknown.target_fqn, None, "no type inference ⇒ no guess");
        assert!(!unknown.is_lib);
        assert_eq!(unknown.caller_fqn, "kotlin·com.acme.svc·Widget·go");

        // A bare call is recorded but NOT named — kotlin has top-level functions.
        let bare = ref_to(&out, "bare");
        assert_eq!(bare.target_fqn, None, "an unqualified call is not necessarily a member");

        // And nothing was invented: every named target is either a lib node or
        // an imported type's own package, never this file's.
        for r in &out.refs {
            if let Some(f) = r.target_fqn.as_deref() {
                assert!(
                    f.starts_with("lib·") || !f.starts_with("kotlin·com.acme.svc·Widget·"),
                    "a call target must not be stamped onto the calling type: {f}"
                );
            }
        }
    }

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

/// The tree-sitter-kotlin SHAPES the fqn producer resolves against.
///
/// This grammar ships NO `node-types.json` (only `parser.c`, `scanner.c`,
/// `tree_sitter/` under `crates/senseid/grammars/kotlin/src/`), so the node
/// kinds a resolver matches on cannot be looked up — they can only be observed.
/// A wrong kind does not fail loudly: the match arm simply never fires and the
/// producer emits NOTHING, which is indistinguishable from a language that has
/// no producer at all. That is precisely how kotlin came to emit 3,713 import
/// edges and zero calls, extends or implements.
///
/// So these are pinned here. A grammar bump that renames a kind breaks THIS
/// test loudly instead of silently emptying the producer.
#[cfg(test)]
mod kotlin_grammar_shapes {
    use super::*;

    fn parse(src: &str) -> (tree_sitter::Tree, String) {
        let mut p = Parser::new();
        p.set_language(&unsafe { tree_sitter_kotlin() }).expect("kotlin grammar");
        (p.parse(src, None).expect("parse"), src.to_string())
    }

    /// Depth-first search for the first node of `kind`.
    fn find<'t>(n: tree_sitter::Node<'t>, kind: &str) -> Option<tree_sitter::Node<'t>> {
        if n.kind() == kind {
            return Some(n);
        }
        for i in 0..n.child_count() {
            if let Some(hit) = n.child(i).and_then(|c| find(c, kind)) {
                return Some(hit);
            }
        }
        None
    }
    fn kinds_of(n: tree_sitter::Node) -> Vec<String> {
        (0..n.child_count()).filter_map(|i| n.child(i)).map(|c| c.kind().to_string()).collect()
    }

    /// `child_by_field_name` is USELESS for kotlin — `FIELD_COUNT 0`.
    ///
    /// Every java resolver line that reads a field (`field(node,"name",src)`)
    /// must become a positional kind scan when ported. Getting this wrong yields
    /// a producer that silently returns nothing.
    #[test]
    fn no_kotlin_node_exposes_a_field() {
        let (t, _) = parse("class W { fun go() {} }\n");
        let cls = find(t.root_node(), "class_declaration").expect("class_declaration");
        assert!(cls.child_by_field_name("name").is_none(), "FIELD_COUNT is 0 — scan kinds instead");
        let f = find(t.root_node(), "function_declaration").expect("function_declaration");
        assert!(f.child_by_field_name("name").is_none(), "FIELD_COUNT is 0 — scan kinds instead");
    }

    /// An INTERFACE is a `class_declaration` carrying the `interface` keyword.
    ///
    /// There is no `interface_declaration` kind in this grammar, yet
    /// `kotlin_fqn::walk_top` matches on one — a DEAD ARM, which is why every
    /// kotlin interface is currently emitted as `SymbolKind::Class`. The
    /// non-fqn walk already gets this right via `has_keyword(.., "interface")`.
    #[test]
    fn an_interface_is_a_class_declaration_with_a_keyword() {
        let (t, src) = parse("interface Handler { fun handle() }\n");
        assert!(
            find(t.root_node(), "interface_declaration").is_none(),
            "no such kind — the walk_top arm matching it is dead"
        );
        let cls = find(t.root_node(), "class_declaration").expect("an interface parses as a class");
        assert!(kinds_of(cls).contains(&"interface".to_string()), "{:?}", kinds_of(cls));
        assert!(has_keyword(&cls, src.as_bytes(), "interface"));
    }

    /// A CALL is `call_expression`; the callee is its FIRST child.
    ///
    /// Unqualified → `simple_identifier`. Qualified → `navigation_expression`,
    /// whose receiver is its first `simple_identifier` and whose method is the
    /// `simple_identifier` inside its `navigation_suffix`.
    #[test]
    fn a_call_is_a_call_expression_and_a_qualified_one_nests_a_navigation() {
        let (t, src) = parse("fun go() {\n  helper()\n  obj.method()\n}\n");
        let b = src.as_bytes();
        let mut calls = Vec::new();
        fn collect<'t>(n: tree_sitter::Node<'t>, out: &mut Vec<tree_sitter::Node<'t>>) {
            if n.kind() == "call_expression" {
                out.push(n);
            }
            for i in 0..n.child_count() {
                if let Some(c) = n.child(i) {
                    collect(c, out);
                }
            }
        }
        collect(t.root_node(), &mut calls);
        assert_eq!(calls.len(), 2, "two calls");

        // Unqualified: first child names the callee outright.
        let plain = calls[0].child(0).unwrap();
        assert_eq!(plain.kind(), "simple_identifier");
        assert_eq!(plain.utf8_text(b).unwrap(), "helper");

        // Qualified: receiver + navigation_suffix.
        let nav = calls[1].child(0).unwrap();
        assert_eq!(nav.kind(), "navigation_expression");
        assert_eq!(nav.child(0).unwrap().utf8_text(b).unwrap(), "obj", "receiver");
        let suffix = find(nav, "navigation_suffix").expect("navigation_suffix");
        let method = find(suffix, "simple_identifier").expect("method name");
        assert_eq!(method.utf8_text(b).unwrap(), "method");
    }

    /// HERITAGE is `delegation_specifier`, and the SUPERCLASS is the one holding
    /// a `constructor_invocation` — a structural discriminator, not a heuristic.
    ///
    /// `class W : Base(), Iface` puts the superclass and the interfaces in ONE
    /// list. Java splits them by field (`superclass` / `interfaces`); kotlin
    /// cannot, so the parentheses are the signal.
    #[test]
    fn a_superclass_is_the_delegation_specifier_holding_a_constructor_invocation() {
        let (t, src) = parse("class W : Base(), Iface {\n}\n");
        let b = src.as_bytes();
        let cls = find(t.root_node(), "class_declaration").unwrap();
        let specs: Vec<_> = (0..cls.child_count())
            .filter_map(|i| cls.child(i))
            .filter(|c| c.kind() == "delegation_specifier")
            .collect();
        assert_eq!(specs.len(), 2, "one superclass + one interface");

        let sup = specs[0];
        assert!(find(sup, "constructor_invocation").is_some(), "Base() is the SUPERCLASS");
        assert_eq!(find(sup, "type_identifier").unwrap().utf8_text(b).unwrap(), "Base");

        let iface = specs[1];
        assert!(find(iface, "constructor_invocation").is_none(), "Iface is an INTERFACE");
        assert_eq!(find(iface, "type_identifier").unwrap().utf8_text(b).unwrap(), "Iface");
    }

    /// A COMPANION OBJECT nests its members in its OWN `class_body`.
    ///
    /// `collect_members` walks only the type's direct `class_body`, so companion
    /// members are dropped today — and a MISSING def is what turns a legitimate
    /// `W.create()` into a minted phantom.
    #[test]
    fn a_companion_object_nests_a_second_class_body() {
        let (t, src) = parse("class W {\n  companion object {\n    fun create() = 1\n  }\n}\n");
        let comp = find(t.root_node(), "companion_object").expect("companion_object exists");
        let body = find(comp, "class_body").expect("its own class_body");
        let f = find(body, "function_declaration").expect("member is inside it");
        assert_eq!(
            find(f, "simple_identifier").unwrap().utf8_text(src.as_bytes()).unwrap(),
            "create"
        );
    }

    /// An ENUM body is `enum_class_body`, not `class_body`.
    #[test]
    fn an_enum_uses_its_own_body_kind() {
        let (t, _) = parse("enum class Color { RED, GREEN }\n");
        assert!(find(t.root_node(), "enum_class_body").is_some());
        assert!(find(t.root_node(), "enum_entry").is_some());
    }

    /// IMPORTS are `import_header` under an `import_list`, and a star import is
    /// marked by a `wildcard_import` child — so it can be skipped rather than
    /// keyed on a bogus simple name.
    #[test]
    fn imports_nest_under_an_import_list_and_star_imports_are_marked() {
        let (t, src) = parse("package p\nimport a.b.C\nimport d.e.*\n");
        let list = find(t.root_node(), "import_list").expect("import_list wraps them");
        let headers: Vec<_> = (0..list.child_count())
            .filter_map(|i| list.child(i))
            .filter(|c| c.kind() == "import_header")
            .collect();
        assert_eq!(headers.len(), 2);
        assert!(find(headers[0], "wildcard_import").is_none(), "a.b.C is not a star import");
        assert!(find(headers[1], "wildcard_import").is_some(), "d.e.* IS a star import");
        assert!(headers[0].utf8_text(src.as_bytes()).unwrap().contains("a.b.C"));
    }

    /// `by` DELEGATION parses cleanly and does NOT cost the class body.
    ///
    /// Worth pinning because the opposite was asserted during design ("`by`
    /// delegation destroys the parse of the class body") and it is not true —
    /// acting on it would have meant writing off every delegating class's
    /// members as unreachable. A delegating specifier is `explicit_delegation`,
    /// and it carries NO `constructor_invocation`, so the superclass rule above
    /// correctly treats it as an interface-like supertype rather than a base
    /// class.
    #[test]
    fn by_delegation_parses_and_keeps_the_class_body() {
        let (t, src) = parse("class W : Store by delegate {\n  fun go() {}\n}\n");
        let del = find(t.root_node(), "explicit_delegation").expect("`by` is explicit_delegation");
        assert!(find(del, "constructor_invocation").is_none(), "a delegate is not a base class");

        let f = find(t.root_node(), "function_declaration").expect("the body SURVIVES");
        assert_eq!(
            find(f, "simple_identifier").unwrap().utf8_text(src.as_bytes()).unwrap(),
            "go",
            "members of a by-delegating class are reachable"
        );
    }
}
