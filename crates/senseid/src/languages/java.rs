use super::LanguageAdapter;
use super::common::{
    field_text, ir_class, ir_method, ir_module, ir_parsed_file, make_symbol, node_text,
};
use crate::ir::{ClassKind, IRImport, IRParam, IRParsedFile, Visibility};
use crate::types::{ParsedFile, ParsedImport, ParsedSymbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub struct JavaAdapter;

impl LanguageAdapter for JavaAdapter {
    fn language(&self) -> &str {
        "java"
    }

    fn fqn_output(&self, _abs_path: &str, content: &str) -> Option<super::fqn::FqnFileOutput> {
        Some(java_fqn::produce_fqns(content))
    }

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
    ParsedFile {
        file_path: path.into(),
        language: "java".into(),
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
                symbols.push(make_sym(
                    name.clone(),
                    SymbolKind::Interface,
                    &child,
                    lines,
                    src,
                    has_modifier(&child, src, "public"),
                ));
                if let Some(body) = child.child_by_field_name("body") {
                    extract_members(&body, src, lines, symbols, &name);
                }
            }
            "enum_declaration" => {
                let name = field_text(&child, "name", src);
                symbols.push(make_sym(
                    name,
                    SymbolKind::Enum,
                    &child,
                    lines,
                    src,
                    has_modifier(&child, src, "public"),
                ));
            }
            "import_declaration" => {
                let text = child.utf8_text(src).unwrap_or_default();
                let path = text
                    .trim_start_matches("import ")
                    .trim_start_matches("static ")
                    .trim_end_matches(';')
                    .trim()
                    .to_string();
                let name = path.rsplit('.').next().unwrap_or(&path).to_string();
                imports.push(ParsedImport { target_path: path, names: vec![name] });
            }
            _ => {}
        }
    }
}

fn extract_members(
    body: &Node,
    src: &[u8],
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    class_name: &str,
) {
    for i in 0..body.child_count() {
        let child = body.child(i).unwrap();
        match child.kind() {
            "method_declaration" | "constructor_declaration" => {
                let name = field_text(&child, "name", src);
                if !name.is_empty() {
                    let mut sym = make_sym(
                        name,
                        SymbolKind::Method,
                        &child,
                        lines,
                        src,
                        has_modifier(&child, src, "public"),
                    );
                    sym.parent = Some(class_name.to_string());
                    symbols.push(sym);
                }
            }
            "field_declaration" => {
                if has_modifier(&child, src, "static")
                    && has_modifier(&child, src, "final")
                    && let Some(declarator) = find_child_kind(&child, "variable_declarator")
                {
                    let name = field_text(&declarator, "name", src);
                    if !name.is_empty() {
                        symbols.push(make_sym(
                            name,
                            SymbolKind::Const,
                            &child,
                            lines,
                            src,
                            has_modifier(&child, src, "public"),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

fn make_sym(
    name: String,
    kind: SymbolKind,
    node: &Node,
    lines: &[&str],
    src: &[u8],
    is_exported: bool,
) -> ParsedSymbol {
    make_symbol(name, kind, node, lines, is_exported, extract_javadoc(node, src))
}

fn extract_javadoc(node: &Node, src: &[u8]) -> Option<String> {
    let prev = node.prev_sibling()?;
    if prev.kind() != "block_comment" {
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

fn has_modifier(node: &Node, src: &[u8], keyword: &str) -> bool {
    for i in 0..node.child_count() {
        let c = node.child(i).unwrap();
        if c.kind() == "modifiers" {
            let text = c.utf8_text(src).unwrap_or_default();
            if text.contains(keyword) {
                return true;
            }
        }
    }
    false
}

fn find_child_kind<'a>(node: &'a Node, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        let c = node.child(i).unwrap();
        if c.kind() == kind {
            return Some(c);
        }
    }
    None
}

/// Parse Java source into IR.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_java::LANGUAGE.into()).expect("java");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return IRParsedFile {
                file_path: file_path.into(),
                language: "java".into(),
                ..Default::default()
            };
        }
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
                let mut class = ir_class(
                    name,
                    &child,
                    kind,
                    is_pub,
                    extract_javadoc(&child, src),
                    collect_java_annotations(&child, src),
                );

                // Extract implements/extends
                if let Some(sc) = child.child_by_field_name("superclass") {
                    class.extends = Some(sc.utf8_text(src).unwrap_or_default().to_string());
                }
                if let Some(ifaces) = child.child_by_field_name("interfaces") {
                    class.implements = ifaces
                        .utf8_text(src)
                        .unwrap_or_default()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }

                // Extract methods from body
                if let Some(body) = child.child_by_field_name("body") {
                    for j in 0..body.child_count() {
                        if let Some(member) = body.child(j)
                            && (member.kind() == "method_declaration"
                                || member.kind() == "constructor_declaration")
                        {
                            let mname = field_text(&member, "name", src);
                            let mparams = extract_java_params(&member, src);
                            let ret = field_text(&member, "type", src);
                            let is_static = has_modifier(&member, src, "static");
                            let vis = if has_modifier(&member, src, "public") {
                                Visibility::Public
                            } else if has_modifier(&member, src, "private") {
                                Visibility::Private
                            } else if has_modifier(&member, src, "protected") {
                                Visibility::Protected
                            } else {
                                Visibility::Internal
                            };
                            class.methods.push(ir_method(
                                mname,
                                &member,
                                vis == Visibility::Public,
                                false,
                                is_static,
                                mparams,
                                if ret.is_empty() { None } else { Some(ret) },
                                extract_javadoc(&member, src),
                                collect_java_annotations(&member, src),
                                vis,
                                &node_text(&member, src),
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
                imports.push(IRImport {
                    source: path.to_string(),
                    names: vec![name],
                    is_reexport: false,
                });
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
                && (p.kind() == "formal_parameter" || p.kind() == "spread_parameter")
            {
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

// ── FQN producer (plan Phase 6.3) ────────────────────────────────────────────
// Java's package is declared IN the source (`package a.b.c;`), so the context comes
// from the file itself, not a manifest. Every method lives in a class (no free
// functions): an unqualified call is `this.<m>` (the enclosing class), `Type.m()`
// is static, `obj.m()` resolves via a bounded `Type v = new Type()` binding. An
// imported class resolves to ITS package's fqn; JDK-family packages (java./javax./…)
// are treated as external `lib` nodes (there is no project-wide package registry).
pub(crate) mod java_fqn {
    use super::super::fqn::{self, FileFqnContext, FqnDefinition, FqnFileOutput, FqnReference};
    use super::{Node, Parser, SymbolKind};
    use std::collections::{HashMap, HashSet};

    const JAVA_LANG: &str = "java";
    const JAVA_CALL_DENYLIST: &[&str] = &[
        "toString", "equals", "hashCode", "get", "set", "put", "add", "remove", "size", "length",
        "isEmpty", "println", "print", "printf", "format", "contains", "stream", "collect",
        "forEach", "build", "iterator", "next",
    ];

    fn text(node: &Node, src: &[u8]) -> String {
        node.utf8_text(src).unwrap_or_default().to_string()
    }
    fn field(node: &Node, name: &str, src: &[u8]) -> String {
        node.child_by_field_name(name).map(|n| text(&n, src)).unwrap_or_default()
    }
    fn is_pascal(s: &str) -> bool {
        s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    }
    /// JDK / well-known runtime roots → treated as external dependencies.
    fn is_external_pkg(pkg: &str) -> bool {
        ["java.", "javax.", "kotlin.", "android.", "sun.", "scala.", "jakarta."]
            .iter()
            .any(|p| pkg.starts_with(p))
    }

    pub fn produce_fqns(source: &str) -> FqnFileOutput {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_java::LANGUAGE.into()).is_err() {
            return FqnFileOutput::default();
        }
        let Some(tree) = parser.parse(source, None) else { return FqnFileOutput::default() };
        let src = source.as_bytes();
        let root = tree.root_node();

        // Package (from `package a.b.c;`) + import map (simple class → fqcn).
        let mut package = String::new();
        let mut imports: HashMap<String, String> = HashMap::new();
        for i in 0..root.child_count() {
            let child = root.child(i).unwrap();
            match child.kind() {
                "package_declaration" => {
                    for j in 0..child.child_count() {
                        let c = child.child(j).unwrap();
                        if c.kind() == "scoped_identifier" || c.kind() == "identifier" {
                            package = text(&c, src);
                            break;
                        }
                    }
                }
                "import_declaration" => {
                    let raw = text(&child, src);
                    let path = raw
                        .trim_start_matches("import ")
                        .trim_start_matches("static ")
                        .trim_end_matches(';')
                        .trim()
                        .to_string();
                    if !path.is_empty() && !path.ends_with('*') {
                        let leaf = path.rsplit('.').next().unwrap_or(&path).to_string();
                        imports.insert(leaf, path);
                    }
                }
                _ => {}
            }
        }
        let ctx = FileFqnContext { package, module: String::new() };

        let mut out = FqnFileOutput {
            package: ctx.package.clone(),
            module: String::new(),
            ..Default::default()
        };
        let lines: Vec<&str> = source.lines().collect();
        walk(&root, src, &lines, &ctx, &imports, None, &mut out);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(
        node: &Node,
        src: &[u8],
        lines: &[&str],
        ctx: &FileFqnContext,
        imports: &HashMap<String, String>,
        class: Option<&str>,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            match child.kind() {
                "class_declaration" | "interface_declaration" | "enum_declaration" => {
                    let name = field(&child, "name", src);
                    if name.is_empty() {
                        continue;
                    }
                    let kind = match child.kind() {
                        "interface_declaration" => SymbolKind::Interface,
                        "enum_declaration" => SymbolKind::Enum,
                        _ => SymbolKind::Class,
                    };
                    out.defs.push(FqnDefinition {
                        fqn: fqn::item(JAVA_LANG, &ctx.package, "", &name),
                        name: name.clone(),
                        kind,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        is_exported: true,
                        signature: lines
                            .get(child.start_position().row)
                            .map(|l| l.trim().to_string()),
                        docstring: None,
                        parent_type: None,
                        parent_fqn: None,
                    });
                    if let Some(body) = child.child_by_field_name("body") {
                        walk(&body, src, lines, ctx, imports, Some(&name), out);
                    }
                }
                "method_declaration" | "constructor_declaration" => {
                    let Some(cls) = class else { continue };
                    let name = field(&child, "name", src);
                    if name.is_empty() {
                        continue;
                    }
                    let fqn_str = fqn::method(JAVA_LANG, &ctx.package, "", cls, &name);
                    out.defs.push(FqnDefinition {
                        fqn: fqn_str.clone(),
                        name,
                        kind: SymbolKind::Method,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        is_exported: true,
                        signature: lines
                            .get(child.start_position().row)
                            .map(|l| l.trim().to_string()),
                        docstring: None,
                        parent_type: Some(cls.to_string()),
                        parent_fqn: Some(fqn::item(JAVA_LANG, &ctx.package, "", cls)),
                    });
                    if let Some(body) = child.child_by_field_name("body") {
                        let bindings = build_bindings(&child, src);
                        let mut seen = HashSet::new();
                        collect_calls(
                            &body, src, ctx, imports, cls, &bindings, &fqn_str, &mut seen, out,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Bounded binding→type map (plan 0.7): typed params + `Type v = …`.
    fn build_bindings(method: &Node, src: &[u8]) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(params) = method.child_by_field_name("parameters") {
            for i in 0..params.child_count() {
                let p = params.child(i).unwrap();
                if p.kind() == "formal_parameter" {
                    let ty = field(&p, "type", src);
                    let nm = field(&p, "name", src);
                    if !ty.is_empty() && !nm.is_empty() {
                        map.insert(nm, base_type(&ty));
                    }
                }
            }
        }
        collect_locals(method, src, &mut map);
        map
    }

    /// Recursively find `Type v = …` local declarations (they can nest in blocks).
    fn collect_locals(node: &Node, src: &[u8], map: &mut HashMap<String, String>) {
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind() == "local_variable_declaration" {
                let ty = field(&child, "type", src);
                if !ty.is_empty()
                    && let Some(decl) = (0..child.child_count())
                        .find_map(|j| child.child(j).filter(|c| c.kind() == "variable_declarator"))
                {
                    let nm = field(&decl, "name", src);
                    if !nm.is_empty() {
                        map.insert(nm, base_type(&ty));
                    }
                }
            }
            collect_locals(&child, src, map);
        }
    }

    /// Base type name: `List<Foo>` → `List`, `a.b.Foo` → `Foo`.
    fn base_type(t: &str) -> String {
        let base = t.split('<').next().unwrap_or(t).trim();
        base.rsplit('.').next().unwrap_or(base).trim().to_string()
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_calls(
        node: &Node,
        src: &[u8],
        ctx: &FileFqnContext,
        imports: &HashMap<String, String>,
        class: &str,
        bindings: &HashMap<String, String>,
        caller_fqn: &str,
        seen: &mut HashSet<String>,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind() == "method_invocation"
                && let Some((target_fqn, is_lib, target_name)) =
                    resolve_call(&child, src, ctx, imports, class, bindings)
                && seen.insert(target_fqn.clone().unwrap_or_else(|| format!("?{target_name}")))
            {
                out.refs.push(FqnReference {
                    caller_fqn: caller_fqn.to_string(),
                    caller_line: child.start_position().row as u32 + 1,
                    target_fqn,
                    target_name,
                    is_lib,
                });
            }
            collect_calls(&child, src, ctx, imports, class, bindings, caller_fqn, seen, out);
        }
    }

    fn resolve_call(
        mi: &Node,
        src: &[u8],
        ctx: &FileFqnContext,
        imports: &HashMap<String, String>,
        class: &str,
        bindings: &HashMap<String, String>,
    ) -> Option<(Option<String>, bool, String)> {
        let method = field(mi, "name", src);
        if method.is_empty() || JAVA_CALL_DENYLIST.contains(&method.as_str()) {
            return None;
        }
        let obj = mi.child_by_field_name("object");
        match obj {
            // Unqualified `m()` → the enclosing class's method.
            None => Some((
                Some(fqn::method(JAVA_LANG, &ctx.package, "", class, &method)),
                false,
                method,
            )),
            Some(o) => {
                if o.kind() == "this" {
                    return Some((
                        Some(fqn::method(JAVA_LANG, &ctx.package, "", class, &method)),
                        false,
                        method,
                    ));
                }
                if o.kind() == "identifier" {
                    let oname = text(&o, src);
                    if is_pascal(&oname) {
                        // `Type.staticMethod()` — resolve Type via imports (its own
                        // package) or as a same-package class.
                        return Some(match imports.get(&oname) {
                            Some(fqcn) => resolve_type_call(fqcn, &method),
                            None => (
                                Some(fqn::method(JAVA_LANG, &ctx.package, "", &oname, &method)),
                                false,
                                method,
                            ),
                        });
                    }
                    if let Some(ty) = bindings.get(&oname) {
                        // Instance receiver — its class's method (same package, best effort).
                        return Some((
                            Some(fqn::method(JAVA_LANG, &ctx.package, "", ty, &method)),
                            false,
                            method,
                        ));
                    }
                }
                // Unknown receiver → no wrong merge.
                Some((None, false, method))
            }
        }
    }

    /// Resolve a call on an imported/fully-qualified class `a.b.Foo` → `a.b.Foo.m`:
    /// JDK-family packages become `lib` nodes; any other package is treated as a
    /// project package and resolves to its own class-method fqn.
    fn resolve_type_call(fqcn: &str, method: &str) -> (Option<String>, bool, String) {
        let (pkg, cls) = fqcn.rsplit_once('.').unwrap_or(("", fqcn));
        if is_external_pkg(pkg) {
            let top = pkg.split('.').next().unwrap_or(pkg);
            (Some(fqn::lib(top, fqcn, method)), true, method.to_string())
        } else {
            (Some(fqn::method(JAVA_LANG, pkg, "", cls, method)), false, method.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        JavaAdapter.parse(src, "Test.java")
    }

    // ── FQN producer (Phase 6.3) ────────────────────────────────────────────
    use crate::languages::fqn::{FqnFileOutput, FqnReference};
    fn def_fqn<'a>(out: &'a FqnFileOutput, name: &str) -> &'a str {
        out.defs.iter().find(|d| d.name == name).map(|d| d.fqn.as_str()).unwrap_or("<no-def>")
    }
    fn ref_to<'a>(out: &'a FqnFileOutput, target_name: &str) -> &'a FqnReference {
        out.refs
            .iter()
            .find(|r| r.target_name == target_name)
            .unwrap_or_else(|| panic!("no ref to `{target_name}` in {:?}", out.refs))
    }

    #[test]
    fn java_def_fqn() {
        let out =
            java_fqn::produce_fqns("package com.app;\nclass Widget {\n    void spin() {}\n}\n");
        assert_eq!(def_fqn(&out, "Widget"), "java·com.app·Widget", "class carries its package");
        assert_eq!(def_fqn(&out, "spin"), "java·com.app·Widget·spin", "method nests on its class");
    }

    #[test]
    fn java_ref_fqn_import() {
        let out = java_fqn::produce_fqns(
            "package com.app;\nimport a.b.Helper;\nclass C {\n    void use() { Helper.run(); }\n}\n",
        );
        let r = ref_to(&out, "run");
        assert_eq!(
            r.target_fqn.as_deref(),
            Some("java·a.b·Helper·run"),
            "imported project class resolves to its own package"
        );
        assert!(!r.is_lib);
        assert_eq!(r.caller_fqn, "java·com.app·C·use");
    }

    #[test]
    fn java_method_scope() {
        let src = "package com.app;\nclass Engine {\n    void run() {\n        this.tick();\n        Gadget g = new Gadget();\n        g.spin();\n    }\n    void tick() {}\n}\n";
        let out = java_fqn::produce_fqns(src);
        assert_eq!(
            ref_to(&out, "tick").target_fqn.as_deref(),
            Some("java·com.app·Engine·tick"),
            "this.m → enclosing class"
        );
        assert_eq!(
            ref_to(&out, "spin").target_fqn.as_deref(),
            Some("java·com.app·Gadget·spin"),
            "Type v = new Type(); v.m() → Type.m (0.7 binding)"
        );
    }

    #[test]
    fn java_external_is_lib() {
        let out = java_fqn::produce_fqns(
            "package com.app;\nimport java.util.List;\nclass C {\n    void f() { List.of(); }\n}\n",
        );
        let r = ref_to(&out, "of");
        assert_eq!(
            r.target_fqn.as_deref(),
            Some("lib·java·java.util.List·of"),
            "JDK class → lib node"
        );
        assert!(r.is_lib);
    }

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
        assert!(!pf.symbols.is_empty());
        assert_eq!(pf.symbols[0].kind, SymbolKind::Interface);
    }

    #[test]
    fn parses_methods() {
        let pf = parse(
            "public class Svc {\n  public void run() {}\n  private int calc() { return 0; }\n}",
        );
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
        let pf = parse(
            "public class Svc {\n  public void run() {}\n  private int calc() { return 0; }\n}",
        );
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

    fn parse_ir(src: &str) -> IRParsedFile {
        parse_to_ir(src, "Test.java")
    }

    #[test]
    fn ir_class_with_methods() {
        let pf =
            parse_ir("public class Svc {\n  public String getName(int id) { return null; }\n}");
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
        assert!(!pf.modules[0].imports.is_empty());
        assert_eq!(pf.modules[0].imports[0].source, "java.util.List");
    }
}
