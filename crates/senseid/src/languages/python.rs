use super::LanguageAdapter;
use super::common::{ir_class, ir_function, ir_method, ir_module, ir_parsed_file, node_text};
use crate::ir::{
    ClassKind, IRBase, IRClass, IRConstant, IRFunction, IRImport, IRParam, IRParsedFile, Visibility,
};
use crate::types::{ParsedEdge, ParsedFile, ParsedImport, ParsedSymbol, SymbolKind};
use tree_sitter::{Node, Parser};

pub struct PythonAdapter;

impl LanguageAdapter for PythonAdapter {
    fn supports_fqn(&self) -> bool {
        true
    }

    /// Backed by real machinery: an imports map keyed on the bound name.
    fn resolves_in_scope(&self) -> bool {
        true
    }

    fn extensions(&self) -> &[&'static str] {
        &[".py"]
    }

    fn language(&self) -> &str {
        "python"
    }

    fn fqn_output(
        &self,
        abs_path: &str,
        _rel_path: &str,
        content: &str,
    ) -> Option<super::fqn::FqnFileOutput> {
        python_fqn::python_file_context(abs_path).map(|ctx| python_fqn::produce_fqns(content, &ctx))
    }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let mut parser = Parser::new();
        let lang = tree_sitter_python::LANGUAGE;
        parser.set_language(&lang.into()).expect("failed to set python language");

        let tree = match parser.parse(source, None) {
            Some(t) => t,
            None => return empty_file(file_path),
        };

        let lines: Vec<&str> = source.lines().collect();
        let root = tree.root_node();

        let src = source.as_bytes();
        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        extract_symbols(&root, src, &lines, &mut symbols, None);
        extract_imports(&root, src, &mut imports);

        let edges = extract_edges(&root, &symbols);

        ParsedFile {
            file_path: file_path.to_string(),
            language: "python".to_string(),
            symbols,
            edges,
            imports,
        }
    }
}

/// Parse Python source into IR with params, return types, decorators, inheritance.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).expect("python");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            return IRParsedFile {
                file_path: file_path.into(),
                language: "python".into(),
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
    let mut constants = Vec::new();

    walk_ir_py(
        &root,
        src,
        &lines,
        &mut functions,
        &mut classes,
        &mut imports,
        &mut constants,
        None,
    );

    let is_test = file_path.contains("test")
        || source.contains("import pytest")
        || source.contains("import unittest");
    let module = ir_module(file_path, "python", functions, constants, imports, is_test);
    ir_parsed_file(file_path, "python", module, classes)
}

#[allow(clippy::too_many_arguments)]
fn walk_ir_py(
    node: &Node,
    src: &[u8],
    lines: &[&str],
    functions: &mut Vec<IRFunction>,
    classes: &mut Vec<IRClass>,
    imports: &mut Vec<IRImport>,
    constants: &mut Vec<IRConstant>,
    class_ctx: Option<&str>,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_definition" | "decorated_definition" => {
                let (func_node, decorators) = if child.kind() == "decorated_definition" {
                    let decos = collect_py_decorators(&child, src);
                    let inner = (0..child.child_count())
                        .find_map(|j| child.child(j).filter(|c| c.kind() == "function_definition"));
                    match inner {
                        Some(f) => (f, decos),
                        None => {
                            // Might be a decorated class
                            if let Some(cls) = (0..child.child_count()).find_map(|j| {
                                child.child(j).filter(|c| c.kind() == "class_definition")
                            }) {
                                let name = cls
                                    .child_by_field_name("name")
                                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                                    .unwrap_or_default();
                                let mut class = ir_class(
                                    name,
                                    &cls,
                                    ClassKind::Class,
                                    !cls.child_by_field_name("name")
                                        .map(|n| {
                                            n.utf8_text(src).unwrap_or_default().starts_with('_')
                                        })
                                        .unwrap_or(false),
                                    extract_docstring(&cls, src),
                                    collect_py_decorators(&child, src),
                                );
                                class.extends = extract_py_base_class(&cls, src);
                                if let Some(body) = cls.child_by_field_name("body") {
                                    walk_ir_py_methods(&body, src, &mut class);
                                }
                                classes.push(class);
                            }
                            continue;
                        }
                    }
                } else {
                    (child, Vec::new())
                };

                let name = func_node
                    .child_by_field_name("name")
                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let is_exported = !name.starts_with('_');
                let params = extract_py_params(&func_node, src);
                let return_type = extract_py_return_type(&func_node, src);
                let docstring = extract_docstring(&func_node, src);
                let is_async = node_text(&func_node, src).starts_with("async ");
                let body_text = node_text(&func_node, src);

                if class_ctx.is_none() {
                    functions.push(ir_function(
                        name,
                        &func_node,
                        lines,
                        is_exported,
                        is_async,
                        params,
                        return_type,
                        docstring,
                        decorators,
                        &body_text,
                    ));
                }
                // Methods are handled in walk_ir_py_methods
            }
            "class_definition" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let is_exported = !name.starts_with('_');
                let mut class = ir_class(
                    name,
                    &child,
                    ClassKind::Class,
                    is_exported,
                    extract_docstring(&child, src),
                    Vec::new(),
                );
                class.extends = extract_py_base_class(&child, src);
                if let Some(body) = child.child_by_field_name("body") {
                    walk_ir_py_methods(&body, src, &mut class);
                }
                classes.push(class);
            }
            "expression_statement" if class_ctx.is_none() => {
                if let Some(expr) = child.child(0)
                    && expr.kind() == "assignment"
                    && let Some(left) = expr.child_by_field_name("left")
                {
                    let name = left.utf8_text(src).unwrap_or_default().to_string();
                    if left.kind() == "identifier" && name == name.to_uppercase() && name.len() > 1
                    {
                        constants.push(IRConstant {
                            base: IRBase {
                                name,
                                is_exported: true,
                                line_start: child.start_position().row as u32 + 1,
                                line_end: child.end_position().row as u32 + 1,
                                node_type: Some("const".into()),
                                ..Default::default()
                            },
                            type_: None,
                            value_preview: Some(node_text(&expr, src).chars().take(100).collect()),
                        });
                    }
                }
            }
            "import_statement" | "import_from_statement" => {
                extract_py_imports(&child, src, imports);
            }
            _ => {}
        }
    }
}

fn walk_ir_py_methods(body: &Node, src: &[u8], class: &mut IRClass) {
    for i in 0..body.child_count() {
        let child = body.child(i).unwrap();
        let (func_node, decorators) = match child.kind() {
            "function_definition" => (child, Vec::new()),
            "decorated_definition" => {
                let decos = collect_py_decorators(&child, src);
                match (0..child.child_count())
                    .find_map(|j| child.child(j).filter(|c| c.kind() == "function_definition"))
                {
                    Some(f) => (f, decos),
                    None => continue,
                }
            }
            _ => continue,
        };

        let name = func_node
            .child_by_field_name("name")
            .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
            .unwrap_or_default();
        let is_exported = !name.starts_with('_');
        let is_static = decorators.iter().any(|d| d.contains("staticmethod"));
        let is_async = node_text(&func_node, src).starts_with("async ");
        let body_text = node_text(&func_node, src);

        class.methods.push(ir_method(
            name,
            &func_node,
            is_exported,
            is_async,
            is_static,
            extract_py_params(&func_node, src),
            extract_py_return_type(&func_node, src),
            extract_docstring(&func_node, src),
            decorators,
            if is_exported { Visibility::Public } else { Visibility::Private },
            &body_text,
        ));
    }
}

fn extract_py_params(node: &Node, src: &[u8]) -> Vec<IRParam> {
    let mut params = Vec::new();
    if let Some(param_list) = node.child_by_field_name("parameters") {
        for i in 0..param_list.child_count() {
            if let Some(p) = param_list.child(i) {
                match p.kind() {
                    "identifier" => {
                        let name = p.utf8_text(src).unwrap_or_default().to_string();
                        if name != "self" && name != "cls" {
                            params.push(IRParam { name, ..Default::default() });
                        } else {
                            params.push(IRParam {
                                name,
                                type_: Some("Self".into()),
                                ..Default::default()
                            });
                        }
                    }
                    "typed_parameter" => {
                        let name = p
                            .child_by_field_name("name")
                            .or_else(|| p.child(0))
                            .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                            .unwrap_or_default();
                        let type_ = p
                            .child_by_field_name("type")
                            .map(|t| t.utf8_text(src).unwrap_or_default().to_string());
                        params.push(IRParam { name, type_, ..Default::default() });
                    }
                    "default_parameter" | "typed_default_parameter" => {
                        let name = p
                            .child_by_field_name("name")
                            .or_else(|| p.child(0))
                            .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                            .unwrap_or_default();
                        let type_ = p
                            .child_by_field_name("type")
                            .map(|t| t.utf8_text(src).unwrap_or_default().to_string());
                        let default = p
                            .child_by_field_name("value")
                            .map(|v| v.utf8_text(src).unwrap_or_default().to_string());
                        params.push(IRParam {
                            name,
                            type_,
                            default_value: default,
                            is_optional: true,
                        });
                    }
                    _ => {}
                }
            }
        }
    }
    params
}

fn extract_py_return_type(node: &Node, src: &[u8]) -> Option<String> {
    node.child_by_field_name("return_type")
        .map(|t| t.utf8_text(src).unwrap_or_default().trim().to_string())
        .filter(|s| !s.is_empty())
}

fn extract_py_base_class(node: &Node, src: &[u8]) -> Option<String> {
    node.child_by_field_name("superclasses")
        .and_then(|args| args.child(1)) // skip '('
        .map(|c| c.utf8_text(src).unwrap_or_default().to_string())
        .filter(|s| !s.is_empty() && s != ")")
}

fn collect_py_decorators(decorated_node: &Node, src: &[u8]) -> Vec<String> {
    let mut decos = Vec::new();
    for i in 0..decorated_node.child_count() {
        if let Some(c) = decorated_node.child(i)
            && c.kind() == "decorator"
        {
            decos.push(c.utf8_text(src).unwrap_or_default().trim().to_string());
        }
    }
    decos
}

/// One name a Python import brings into scope.
struct PyBinding {
    /// The name in scope after the statement — the alias when one is given, the
    /// TOP segment for a dotted plain import (`import os.path` binds `os`), and
    /// `*` for a star-import.
    local_name: String,
    /// Dotted path the name refers to.
    full_path: String,
}

/// Join a `from`-base and a member, tolerating the relative forms (`.`, `..pkg`).
fn py_join(base: &str, name: &str) -> String {
    match (base.is_empty(), base.ends_with('.')) {
        (true, _) => name.to_string(),
        (_, true) => format!("{base}{name}"),
        _ => format!("{base}.{name}"),
    }
}

/// Every binding one `import_statement` / `import_from_statement` introduces.
///
/// The three import readers in this file each re-derived this. Two of them matched
/// only `dotted_name` under `import_statement`, so `import numpy as np` — which
/// parses to an `aliased_import` — produced no record whatsoever; and both
/// recorded the pre-`as` name for `from a import b as c`, which is the one name
/// that is not in scope.
fn py_bindings(stmt: &Node, src: &[u8]) -> Vec<PyBinding> {
    let mut out = Vec::new();
    let named = |c: &Node, field: &str| c.child_by_field_name(field).map(|n| node_text(&n, src));
    match stmt.kind() {
        "import_statement" => {
            for j in 0..stmt.child_count() {
                let c = stmt.child(j).unwrap();
                match c.kind() {
                    "dotted_name" => {
                        let full = node_text(&c, src);
                        let top = full.split('.').next().unwrap_or(&full).to_string();
                        out.push(PyBinding { local_name: top, full_path: full });
                    }
                    "aliased_import" => {
                        if let (Some(name), Some(alias)) = (named(&c, "name"), named(&c, "alias")) {
                            out.push(PyBinding { local_name: alias, full_path: name });
                        }
                    }
                    _ => {}
                }
            }
        }
        "import_from_statement" => {
            let mut base = String::new();
            for j in 0..stmt.child_count() {
                let c = stmt.child(j).unwrap();
                match c.kind() {
                    "dotted_name" | "relative_import" if base.is_empty() => {
                        base = node_text(&c, src)
                    }
                    "dotted_name" => {
                        let name = node_text(&c, src);
                        out.push(PyBinding { full_path: py_join(&base, &name), local_name: name });
                    }
                    "aliased_import" => {
                        if let (Some(name), Some(alias)) = (named(&c, "name"), named(&c, "alias")) {
                            out.push(PyBinding {
                                local_name: alias,
                                full_path: py_join(&base, &name),
                            });
                        }
                    }
                    "wildcard_import" => {
                        out.push(PyBinding { local_name: "*".into(), full_path: base.clone() })
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    out
}

/// Collapse an import statement into the `(path, names)` shape both import
/// records use. A plain `import` yields one record per bound module (so
/// `import a, b` stays two); a `from` import yields one record whose `names`
/// carry `source as bound` wherever an alias renames the member.
fn py_import_records(stmt: &Node, src: &[u8]) -> Vec<(String, Vec<String>)> {
    let bindings = py_bindings(stmt, src);
    if bindings.is_empty() {
        return Vec::new();
    }
    if stmt.kind() == "import_statement" {
        return bindings.into_iter().map(|b| (b.full_path, vec![b.local_name])).collect();
    }
    // `from <base> import …` — the base is the first dotted/relative child.
    let base = (0..stmt.child_count())
        .filter_map(|j| stmt.child(j))
        .find(|c| matches!(c.kind(), "dotted_name" | "relative_import"))
        .map(|c| node_text(&c, src))
        .unwrap_or_default();
    let names = bindings
        .iter()
        .map(|b| {
            let member = b.full_path.rsplit('.').next().unwrap_or_default();
            if member == b.local_name {
                b.local_name.clone()
            } else {
                format!("{member} as {}", b.local_name)
            }
        })
        .collect();
    vec![(base, names)]
}

fn extract_py_imports(node: &Node, src: &[u8], imports: &mut Vec<IRImport>) {
    for (source, names) in py_import_records(node, src) {
        imports.push(IRImport { source, names, is_reexport: false });
    }
}

fn empty_file(path: &str) -> ParsedFile {
    ParsedFile {
        file_path: path.to_string(),
        language: "python".to_string(),
        symbols: vec![],
        edges: vec![],
        imports: vec![],
    }
}

fn extract_symbols(
    node: &Node,
    src: &[u8],
    lines: &[&str],
    symbols: &mut Vec<ParsedSymbol>,
    class_name: Option<&str>,
) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_definition" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let kind =
                    if class_name.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                let sig = lines.get(child.start_position().row).map(|l| l.trim().to_string());
                let docstring = extract_docstring(&child, src);
                symbols.push(ParsedSymbol {
                    name: name.clone(),
                    kind,
                    signature: sig,
                    docstring,
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: !name.starts_with('_'),
                    parent: class_name.map(|s| s.to_string()),
                });
            }
            "class_definition" => {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let sig = lines.get(child.start_position().row).map(|l| l.trim().to_string());
                let docstring = extract_docstring(&child, src);
                symbols.push(ParsedSymbol {
                    name: name.clone(),
                    kind: SymbolKind::Class,
                    signature: sig,
                    docstring,
                    line_start: child.start_position().row as u32 + 1,
                    line_end: child.end_position().row as u32 + 1,
                    is_exported: !name.starts_with('_'),
                    parent: None,
                });
                // Recurse into class body for methods
                if let Some(body) = child.child_by_field_name("body") {
                    extract_symbols(&body, src, lines, symbols, Some(&name));
                }
            }
            "expression_statement" if class_name.is_none() => {
                // Top-level constant: FOO = ...
                if let Some(expr) = child.child(0)
                    && expr.kind() == "assignment"
                    && let Some(left) = expr.child_by_field_name("left")
                {
                    let name = left.utf8_text(src).unwrap_or_default().to_string();
                    if left.kind() == "identifier" && name == name.to_uppercase() && name.len() > 1
                    {
                        symbols.push(ParsedSymbol {
                            name,
                            kind: SymbolKind::Const,
                            signature: lines
                                .get(child.start_position().row)
                                .map(|l| l.trim().to_string()),
                            docstring: None,
                            line_start: child.start_position().row as u32 + 1,
                            line_end: child.end_position().row as u32 + 1,
                            is_exported: true,
                            parent: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_docstring(node: &Node, src: &[u8]) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let first = body.child(0)?;
    if first.kind() != "expression_statement" {
        return None;
    }
    let str_node = first.child(0)?;
    if str_node.kind() != "string" {
        return None;
    }

    let text = str_node.utf8_text(src).ok()?;
    let trimmed = if text.starts_with("\"\"\"") || text.starts_with("'''") {
        text[3..text.len() - 3].trim()
    } else if text.starts_with('"') || text.starts_with('\'') {
        text[1..text.len() - 1].trim()
    } else {
        text.trim()
    };
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn extract_imports(root: &Node, src: &[u8], imports: &mut Vec<ParsedImport>) {
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        for (target_path, names) in py_import_records(&child, src) {
            imports.push(ParsedImport { target_path, names });
        }
    }
}

fn extract_edges(root: &Node, symbols: &[ParsedSymbol]) -> Vec<ParsedEdge> {
    let known_names: std::collections::HashSet<&str> = symbols
        .iter()
        .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
        .map(|s| s.name.as_str())
        .collect();

    let mut edges = Vec::new();
    for sym in symbols {
        if !matches!(sym.kind, SymbolKind::Function | SymbolKind::Method) {
            continue;
        }
        // Walk the tree to find call expressions within this symbol's range
        find_calls(root, sym, &known_names, &mut edges);
    }
    edges
}

#[allow(clippy::only_used_in_recursion)] // known and edges accumulate across recursive traversal
fn find_calls(
    node: &Node,
    caller: &ParsedSymbol,
    known: &std::collections::HashSet<&str>,
    edges: &mut Vec<ParsedEdge>,
) {
    if node.kind() == "call"
        && let Some(func) = node.child_by_field_name("function")
        && func.kind() == "identifier"
    {
        // We need the text — for now skip (requires source bytes)
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let row = child.start_position().row as u32 + 1;
            if row >= caller.line_start && row <= caller.line_end {
                find_calls(&child, caller, known, edges);
            }
        }
    }
}

// ── FQN producer (plan Phase 6.2) ────────────────────────────────────────────
// Pure name resolution for Python — the tree-sitter analogue of the Rust producer.
// Python has no impl blocks or traits: methods live directly in a class body and
// nest under it; a call resolves via the import map (from/import), `self` → the
// enclosing class, a bounded `x = Type()` binding, or (external module) a lib node.
pub(crate) mod python_fqn {
    use super::super::fqn::{self, FileFqnContext, FqnDefinition, FqnFileOutput, FqnReference};
    use super::{Node, Parser, SymbolKind, py_bindings};
    use std::collections::{HashMap, HashSet};

    const PY_LANG: &str = "python";

    /// Ubiquitous builtin/method names whose call-sites carry no navigation signal.
    const PY_CALL_DENYLIST: &[&str] = &[
        "append",
        "extend",
        "get",
        "keys",
        "values",
        "items",
        "format",
        "join",
        "split",
        "strip",
        "lower",
        "upper",
        "len",
        "print",
        "isinstance",
        "super",
        "range",
        "enumerate",
        "zip",
        "map",
        "filter",
        "list",
        "dict",
        "set",
        "str",
        "int",
        "float",
        "bool",
        "add",
        "update",
        "pop",
        "sort",
        "sorted",
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
    /// A decorated_definition wraps the real class/function — unwrap to it.
    fn unwrap_decorated<'a>(node: &Node<'a>) -> Node<'a> {
        if node.kind() == "decorated_definition" {
            for i in 0..node.child_count() {
                let c = node.child(i).unwrap();
                if c.kind() == "function_definition" || c.kind() == "class_definition" {
                    return c;
                }
            }
        }
        *node
    }
    /// Reduce a Python type annotation to its base name: `Optional[Foo]` → `Foo`,
    /// `a.b.Foo` → `Foo`.
    fn base_py_type(t: &str) -> String {
        let t = t.trim();
        // Optional[X] / List[X] wrappers → inner.
        if let Some(inner) = t.strip_suffix(']').and_then(|s| s.split_once('[').map(|(_, r)| r)) {
            return base_py_type(inner);
        }
        let base = t.split('[').next().unwrap_or(t).trim();
        base.rsplit('.').next().unwrap_or(base).trim().to_string()
    }

    /// Produce canonical FQNs (plan 0.1) for a Python source file. Pure over
    /// `(source, ctx)`.
    pub fn produce_fqns(source: &str, ctx: &FileFqnContext) -> FqnFileOutput {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_python::LANGUAGE.into()).is_err() {
            return FqnFileOutput::default();
        }
        let Some(tree) = parser.parse(source, None) else {
            return FqnFileOutput::default();
        };
        let src = source.as_bytes();
        let root = tree.root_node();

        let mut imports: HashMap<String, String> = HashMap::new();
        collect_scope(&root, src, &mut imports);

        let mut out = FqnFileOutput {
            package: ctx.package.clone(),
            module: ctx.module.clone(),
            ..Default::default()
        };
        let lines: Vec<&str> = source.lines().collect();
        walk(&root, src, &lines, ctx, &imports, None, &mut out);
        out
    }

    /// Pass 1: import map (bound name → dotted source path), off the shared
    /// binding reader so the FQN map and the import records cannot disagree.
    fn collect_scope(node: &Node, src: &[u8], imports: &mut HashMap<String, String>) {
        for i in 0..node.child_count() {
            let child = unwrap_decorated(&node.child(i).unwrap());
            for b in py_bindings(&child, src) {
                // A star-import binds names this pass cannot enumerate.
                if b.local_name == "*" {
                    continue;
                }
                imports.insert(b.local_name, b.full_path);
            }
        }
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
            let child = unwrap_decorated(&node.child(i).unwrap());
            match child.kind() {
                "function_definition" => {
                    let name = field(&child, "name", src);
                    if name.is_empty() {
                        continue;
                    }
                    let is_exported = !name.starts_with('_');
                    let (fqn_str, kind, parent_type, parent_fqn) = match class {
                        Some(cls) => (
                            fqn::method(PY_LANG, &ctx.package, &ctx.module, cls, &name),
                            SymbolKind::Method,
                            Some(cls.to_string()),
                            Some(fqn::item(PY_LANG, &ctx.package, &ctx.module, cls)),
                        ),
                        None => (
                            fqn::item(PY_LANG, &ctx.package, &ctx.module, &name),
                            SymbolKind::Function,
                            None,
                            None,
                        ),
                    };
                    out.defs.push(FqnDefinition {
                        fqn: fqn_str.clone(),
                        name,
                        kind,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        is_exported,
                        signature: lines
                            .get(child.start_position().row)
                            .map(|l| l.trim().to_string()),
                        docstring: None,
                        parent_type,
                        parent_fqn,
                    });
                    if let Some(body) = child.child_by_field_name("body") {
                        let bindings = build_bindings(&child, src);
                        let mut seen = HashSet::new();
                        collect_calls(
                            &body, src, ctx, imports, class, &bindings, &fqn_str, &mut seen, out,
                        );
                    }
                }
                "class_definition" => {
                    let name = field(&child, "name", src);
                    if name.is_empty() {
                        continue;
                    }
                    out.defs.push(FqnDefinition {
                        fqn: fqn::item(PY_LANG, &ctx.package, &ctx.module, &name),
                        name: name.clone(),
                        kind: SymbolKind::Class,
                        line_start: child.start_position().row as u32 + 1,
                        line_end: child.end_position().row as u32 + 1,
                        is_exported: !name.starts_with('_'),
                        signature: lines
                            .get(child.start_position().row)
                            .map(|l| l.trim().to_string()),
                        docstring: None,
                        parent_type: None,
                        parent_fqn: None,
                    });

                    // Bases. Python has no `implements`, so every base is
                    // Extends — including the multiple-inheritance case, which
                    // is why IRClass.extends (ONE parent) could never hold this.
                    if let Some(sup) = child.child_by_field_name("superclasses") {
                        let child_fqn = fqn::item(PY_LANG, &ctx.package, &ctx.module, &name);
                        for dotted in base_class_paths(&sup, src) {
                            let leaf = dotted.rsplit('.').next().unwrap_or(&dotted).to_string();
                            // A bare name may have been brought in by a
                            // from-import; a dotted path carries its own route.
                            let path = match imports.get(&leaf) {
                                Some(full) if !dotted.contains('.') => full.clone(),
                                _ if dotted.contains('.') => dotted.clone(),
                                // No import and no dots: defined in this module.
                                _ => format!("{}.{}", ctx.package, leaf),
                            };
                            let (parent_fqn, is_lib, _) = classify(&path, ctx, &leaf);
                            out.relations.push(fqn::TypeRelation {
                                child_fqn: child_fqn.clone(),
                                parent_fqn,
                                parent_name: leaf,
                                is_lib,
                                relation: crate::types::RelationKind::Extends,
                            });
                        }
                    }

                    if let Some(body) = child.child_by_field_name("body") {
                        walk(&body, src, lines, ctx, imports, Some(&name), out);
                    }
                }
                _ => {}
            }
        }
    }

    /// Bounded binding→type map (plan 0.7): typed params and `x = Type()`.
    fn build_bindings(fn_node: &Node, src: &[u8]) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(params) = fn_node.child_by_field_name("parameters") {
            for i in 0..params.child_count() {
                let p = params.child(i).unwrap();
                if p.kind() == "typed_parameter"
                    && let Some(ty) = p.child_by_field_name("type")
                    && let Some(nm) = (0..p.child_count())
                        .find_map(|j| p.child(j).filter(|c| c.kind() == "identifier"))
                {
                    map.insert(text(&nm, src), base_py_type(&text(&ty, src)));
                }
            }
        }
        if let Some(body) = fn_node.child_by_field_name("body") {
            for i in 0..body.child_count() {
                let stmt = body.child(i).unwrap();
                if stmt.kind() != "expression_statement" {
                    continue;
                }
                let Some(assign) = stmt.child(0).filter(|c| c.kind() == "assignment") else {
                    continue;
                };
                let (Some(l), Some(r)) =
                    (assign.child_by_field_name("left"), assign.child_by_field_name("right"))
                else {
                    continue;
                };
                if l.kind() == "identifier"
                    && r.kind() == "call"
                    && let Some(f) = r.child_by_field_name("function")
                    && f.kind() == "identifier"
                {
                    let tn = text(&f, src);
                    if is_pascal(&tn) {
                        map.insert(text(&l, src), tn);
                    }
                }
            }
        }
        map
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_calls(
        node: &Node,
        src: &[u8],
        ctx: &FileFqnContext,
        imports: &HashMap<String, String>,
        class: Option<&str>,
        bindings: &HashMap<String, String>,
        caller_fqn: &str,
        seen: &mut HashSet<String>,
        out: &mut FqnFileOutput,
    ) {
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            // A nested function's calls belong to it, not the enclosing one.
            if child.kind() == "function_definition" {
                continue;
            }
            if child.kind() == "call"
                && let Some(func) = child.child_by_field_name("function")
                && let Some((target_fqn, is_lib, target_name)) =
                    resolve_call(&func, src, ctx, imports, class, bindings)
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
        func: &Node,
        src: &[u8],
        ctx: &FileFqnContext,
        imports: &HashMap<String, String>,
        class: Option<&str>,
        bindings: &HashMap<String, String>,
    ) -> Option<(Option<String>, bool, String)> {
        match func.kind() {
            "identifier" => {
                let name = text(func, src);
                if PY_CALL_DENYLIST.contains(&name.as_str()) {
                    return None;
                }
                if let Some(dotted) = imports.get(&name) {
                    Some(classify(dotted, ctx, &name))
                } else {
                    // Local module-level function/class constructor.
                    Some((Some(fqn::item(PY_LANG, &ctx.package, &ctx.module, &name)), false, name))
                }
            }
            "attribute" => {
                let obj = func.child_by_field_name("object")?;
                let attr = func.child_by_field_name("attribute")?;
                let method = text(&attr, src);
                if PY_CALL_DENYLIST.contains(&method.as_str()) {
                    return None;
                }
                if obj.kind() == "identifier" {
                    let obj_name = text(&obj, src);
                    // self.method() / cls.method() → the enclosing class's method.
                    if obj_name == "self" || obj_name == "cls" {
                        return match class {
                            Some(cls) => Some((
                                Some(fqn::method(PY_LANG, &ctx.package, &ctx.module, cls, &method)),
                                false,
                                method,
                            )),
                            None => Some((None, false, method)),
                        };
                    }
                    // Imported module → module.func.
                    if let Some(dotted) = imports.get(&obj_name) {
                        return Some(classify(&format!("{dotted}.{method}"), ctx, &method));
                    }
                    // A bounded `x = Type()` receiver → the type's method.
                    if let Some(ty) = bindings.get(&obj_name) {
                        return Some((
                            Some(fqn::method(PY_LANG, &ctx.package, &ctx.module, ty, &method)),
                            false,
                            method,
                        ));
                    }
                }
                // Unknown receiver (out of the bounded 0.7 scope) → no wrong merge.
                Some((None, false, method))
            }
            _ => None,
        }
    }

    /// The POSITIONAL base classes in a `class_definition`'s `superclasses`.
    ///
    /// That field is an `argument_list`, so it holds keyword arguments —
    /// `metaclass=ABCMeta`, `total=False` — beside the real bases. Taking every
    /// child yields supertypes named `metaclass` or `ABCMeta`, neither of which
    /// is a base class. Only `identifier` and dotted `attribute` children count.
    ///
    /// Returns each base's dotted source text, so `classify` can resolve
    /// `module.Base` as well as a bare name.
    fn base_class_paths(superclasses: &Node, src: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        for i in 0..superclasses.child_count() {
            let Some(c) = superclasses.child(i) else { continue };
            match c.kind() {
                "identifier" | "attribute" => {
                    let t = text(&c, src);
                    if !t.is_empty() {
                        out.push(t);
                    }
                }
                // keyword_argument, comma, parens, and anything else: not a base.
                _ => {}
            }
        }
        out
    }

    /// Classify a dotted path as current-package (internal) vs a dependency, and
    /// build the target fqn. `target_name` is the bare leaf.
    fn classify(
        dotted: &str,
        ctx: &FileFqnContext,
        target_name: &str,
    ) -> (Option<String>, bool, String) {
        let segs: Vec<&str> = dotted.split('.').collect();
        let leaf = segs.last().copied().unwrap_or("");
        let first = segs.first().copied().unwrap_or("");
        if first == ctx.package {
            // Internal: module = segments between the package and the leaf.
            let module =
                if segs.len() > 2 { segs[1..segs.len() - 1].join(".") } else { String::new() };
            (Some(fqn::item(PY_LANG, &ctx.package, &module, leaf)), false, target_name.to_string())
        } else {
            let path =
                if segs.len() >= 2 { segs[..segs.len() - 1].join(".") } else { first.to_string() };
            (Some(fqn::lib(first, &path, leaf)), true, target_name.to_string())
        }
    }

    /// Resolve a Python file's FQN context: the top package (the topmost ancestor
    /// dir in the `__init__.py` chain) and the dotted module path below it. A
    /// standalone script (no package) is its own package with an empty module.
    pub(crate) fn python_file_context(abs_path: &str) -> Option<FileFqnContext> {
        let file = std::path::Path::new(abs_path);
        let stem = file.file_stem().and_then(|s| s.to_str())?.to_string();
        let mut pkg_dirs: Vec<String> = Vec::new(); // nearest-first
        let mut d = file.parent();
        while let Some(cur) = d {
            if cur.join("__init__.py").is_file() {
                if let Some(n) = cur.file_name().and_then(|n| n.to_str()) {
                    pkg_dirs.push(n.to_string());
                }
                d = cur.parent();
            } else {
                break;
            }
        }
        if pkg_dirs.is_empty() {
            return Some(FileFqnContext { package: stem, module: String::new() });
        }
        pkg_dirs.reverse(); // top-first: [package, sub, …]
        let package = pkg_dirs[0].clone();
        let mut mods: Vec<&str> = pkg_dirs[1..].iter().map(String::as_str).collect();
        if stem != "__init__" {
            mods.push(&stem);
        }
        Some(FileFqnContext { package, module: mods.join(".") })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> ParsedFile {
        PythonAdapter.parse(source, "test.py")
    }

    // ── FQN producer (Phase 6.2) ────────────────────────────────────────────
    use crate::languages::fqn::{FileFqnContext, FqnFileOutput, FqnReference};
    fn produce_py(src: &str, package: &str, module: &str) -> FqnFileOutput {
        python_fqn::produce_fqns(
            src,
            &FileFqnContext { package: package.into(), module: module.into() },
        )
    }
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
    fn py_def_fqn() {
        let out = produce_py(
            "def top():\n    pass\nclass Widget:\n    def __init__(self):\n        pass\n    def spin(self):\n        pass\n",
            "mypkg",
            "app",
        );
        assert_eq!(def_fqn(&out, "top"), "python·mypkg·app·top", "module-level function");
        assert_eq!(def_fqn(&out, "Widget"), "python·mypkg·app·Widget", "class");
        assert_eq!(
            def_fqn(&out, "spin"),
            "python·mypkg·app·Widget·spin",
            "method nests on its class"
        );
    }

    #[test]
    fn py_bases_become_relations_and_keyword_arguments_are_filtered() {
        // `superclasses` is an argument_list, so it holds POSITIONAL bases
        // alongside keyword arguments like `metaclass=ABCMeta`. Taking every
        // child yields a supertype named "metaclass" or "ABCMeta" that is not a
        // base class at all.
        let out = produce_py(
            "from mypkg.core import BaseService\nfrom abc import ABCMeta\n\nclass Widget(BaseService, Mixin, metaclass=ABCMeta):\n    pass\n",
            "mypkg",
            "app",
        );
        let rel = |n: &str| {
            out.relations
                .iter()
                .find(|r| r.parent_name == n)
                .unwrap_or_else(|| panic!("no relation to `{n}` in {:?}", out.relations))
        };

        // An imported same-package base → internal.
        let base = rel("BaseService");
        assert_eq!(base.relation, crate::types::RelationKind::Extends);
        assert_eq!(base.child_fqn, "python·mypkg·app·Widget");
        assert_eq!(base.parent_fqn.as_deref(), Some("python·mypkg·core·BaseService"));
        assert!(!base.is_lib);

        // A base with no import resolves against this module — python's own rule.
        let mixin = rel("Mixin");
        assert_eq!(mixin.relation, crate::types::RelationKind::Extends);

        // The keyword argument is NOT a base.
        assert!(
            !out.relations.iter().any(|r| r.parent_name == "ABCMeta"
                || r.parent_name == "metaclass"
                || r.parent_name.contains('=')),
            "metaclass= is not a base class: {:?}",
            out.relations
        );
        assert_eq!(out.relations.len(), 2, "exactly the two positional bases: {:?}", out.relations);
    }

    #[test]
    fn py_ref_fqn_import() {
        let out =
            produce_py("from mypkg.util import helper\ndef use():\n    helper()\n", "mypkg", "app");
        let r = ref_to(&out, "helper");
        assert_eq!(
            r.target_fqn.as_deref(),
            Some("python·mypkg·util·helper"),
            "resolved via the from-import map (same package → internal)"
        );
        assert!(!r.is_lib);
        assert_eq!(r.caller_fqn, "python·mypkg·app·use");
    }

    #[test]
    fn py_method_scope() {
        let src = "class Engine:\n    def run(self):\n        self.tick()\n        g = Gadget()\n        g.spin()\n    def tick(self):\n        pass\n";
        let out = produce_py(src, "mypkg", "engine");
        assert_eq!(
            ref_to(&out, "tick").target_fqn.as_deref(),
            Some("python·mypkg·engine·Engine·tick"),
            "self.method → enclosing class"
        );
        assert_eq!(
            ref_to(&out, "spin").target_fqn.as_deref(),
            Some("python·mypkg·engine·Gadget·spin"),
            "x = Gadget(); x.spin() → Gadget.spin (0.7 binding)"
        );
    }

    #[test]
    fn py_external_is_lib() {
        let out = produce_py("import json\ndef load(s):\n    json.loads(s)\n", "mypkg", "io");
        let r = ref_to(&out, "loads");
        assert_eq!(
            r.target_fqn.as_deref(),
            Some("lib·json·json·loads"),
            "external module call → lib node"
        );
        assert!(r.is_lib);
    }

    #[test]
    fn crlf_source_does_not_panic_on_utf8_extraction() {
        // Regression: the tree is parsed on `source` (CRLF bytes) but symbol
        // extraction used to slice `lines.join("\n")` (CR-stripped, shorter) →
        // node byte-offsets overshot the buffer and panicked in Node::utf8_text.
        // Put a symbol AFTER many CRLF lines so its offset exceeds the LF length.
        let mut source = String::new();
        for i in 0..400 {
            source.push_str(&format!("x{i} = {i}\r\n"));
        }
        source.push_str("def last_fn():\r\n    return 1\r\n");
        source.push_str("class LastClass:\r\n    def method(self):\r\n        pass\r\n");

        let pf = parse(&source); // must not panic
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(
            names.contains(&"last_fn"),
            "expected last_fn, got {:?}",
            &names[names.len().saturating_sub(5)..]
        );
        assert!(names.contains(&"LastClass"), "expected LastClass");
        assert!(names.contains(&"method"), "expected method");
    }

    #[test]
    fn parses_function() {
        let pf = parse("def hello(name: str) -> str:\n    return f'hello {name}'\n");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "hello");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
        assert_eq!(pf.symbols[0].line_start, 1);
        assert_eq!(pf.symbols[0].line_end, 2);
        assert!(pf.symbols[0].is_exported);
    }

    #[test]
    fn parses_class_with_methods() {
        let pf = parse(
            "class Foo:\n    def bar(self):\n        pass\n    def _private(self):\n        pass\n",
        );
        assert_eq!(pf.symbols.len(), 3); // Foo + bar + _private
        assert_eq!(pf.symbols[0].kind, SymbolKind::Class);
        assert_eq!(pf.symbols[0].name, "Foo");
        assert_eq!(pf.symbols[1].kind, SymbolKind::Method);
        assert_eq!(pf.symbols[1].name, "bar");
        assert!(pf.symbols[1].is_exported);
        assert_eq!(pf.symbols[2].name, "_private");
        assert!(!pf.symbols[2].is_exported);
    }

    #[test]
    fn parses_docstring() {
        let pf = parse("def hello():\n    \"\"\"Say hello.\"\"\"\n    pass\n");
        assert_eq!(pf.symbols[0].docstring, Some("Say hello.".to_string()));
    }

    #[test]
    fn parses_constant() {
        let pf = parse("TIMEOUT = 30\n");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "TIMEOUT");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Const);
    }

    #[test]
    fn private_function() {
        let pf = parse("def _internal():\n    pass\n");
        assert!(!pf.symbols[0].is_exported);
    }

    #[test]
    fn empty_file() {
        let pf = parse("");
        assert!(pf.symbols.is_empty());
    }

    #[test]
    fn complex_class() {
        let pf = parse(
            "class UserService:\n    \"\"\"Manages users.\"\"\"\n    def __init__(self, db):\n        self.db = db\n    def get_user(self, uid):\n        \"\"\"Fetch user.\"\"\"\n        return self.db.query(uid)\n",
        );
        assert_eq!(pf.symbols.len(), 3);
        assert_eq!(pf.symbols[0].name, "UserService");
        assert_eq!(pf.symbols[0].docstring, Some("Manages users.".to_string()));
        assert_eq!(pf.symbols[1].name, "__init__");
        assert!(!pf.symbols[1].is_exported); // starts with _
        assert_eq!(pf.symbols[2].name, "get_user");
        assert_eq!(pf.symbols[2].docstring, Some("Fetch user.".to_string()));
    }

    #[test]
    fn language_and_file_path() {
        let pf = parse("x = 1\n");
        assert_eq!(pf.language, "python");
        assert_eq!(pf.file_path, "test.py");
    }

    #[test]
    fn method_parent_set_on_class() {
        let pf = parse(
            "class Dog:\n    def bark(self):\n        pass\n    def sit(self):\n        pass\n",
        );
        let dog = pf.symbols.iter().find(|s| s.name == "Dog").unwrap();
        assert!(dog.parent.is_none(), "class should have no parent");
        let bark = pf.symbols.iter().find(|s| s.name == "bark").unwrap();
        assert_eq!(bark.parent.as_deref(), Some("Dog"));
        assert_eq!(bark.kind, SymbolKind::Method);
        let sit = pf.symbols.iter().find(|s| s.name == "sit").unwrap();
        assert_eq!(sit.parent.as_deref(), Some("Dog"));
    }

    #[test]
    fn method_parent_on_complex_class() {
        let pf = parse(
            "class UserService:\n    def __init__(self, db):\n        self.db = db\n    def get_user(self, uid):\n        return self.db.query(uid)\n",
        );
        let init = pf.symbols.iter().find(|s| s.name == "__init__").unwrap();
        assert_eq!(init.parent.as_deref(), Some("UserService"));
        let get_user = pf.symbols.iter().find(|s| s.name == "get_user").unwrap();
        assert_eq!(get_user.parent.as_deref(), Some("UserService"));
    }

    #[test]
    fn standalone_function_no_parent() {
        let pf = parse("def hello():\n    pass\n");
        assert!(pf.symbols[0].parent.is_none());
    }

    #[test]
    fn constant_no_parent() {
        let pf = parse("TIMEOUT = 30\n");
        assert!(pf.symbols[0].parent.is_none());
    }

    // ── IR Tests ──────────────────────────────────────────────────────

    fn parse_ir(src: &str) -> IRParsedFile {
        parse_to_ir(src, "test.py")
    }

    #[test]
    fn ir_function_with_typed_params() {
        let pf =
            parse_ir("def hello(name: str, count: int = 5) -> str:\n    return name * count\n");
        let func = &pf.modules[0].functions[0];
        assert_eq!(func.base.name, "hello");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "name");
        assert_eq!(func.params[0].type_, Some("str".into()));
        assert_eq!(func.params[1].name, "count");
        assert!(func.params[1].is_optional);
        assert_eq!(func.return_type, Some("str".into()));
    }

    #[test]
    fn ir_class_with_methods_and_inheritance() {
        let pf = parse_ir(
            "class Dog(Animal):\n    \"\"\"A dog.\"\"\"\n    def bark(self) -> str:\n        return 'woof'\n",
        );
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].base.name, "Dog");
        assert_eq!(pf.classes[0].extends, Some("Animal".into()));
        assert_eq!(pf.classes[0].base.docstring, Some("A dog.".into()));
        assert!(!pf.classes[0].methods.is_empty());
        assert_eq!(pf.classes[0].methods[0].base.name, "bark");
    }

    #[test]
    fn ir_async_function() {
        let pf = parse_ir("async def fetch(url: str) -> bytes:\n    pass\n");
        let func = &pf.modules[0].functions[0];
        assert!(func.is_async);
    }

    #[test]
    fn ir_decorator() {
        let pf = parse_ir("@app.route('/hello')\ndef hello():\n    pass\n");
        let func = &pf.modules[0].functions[0];
        assert!(func.decorators.iter().any(|d| d.contains("app.route")));
    }

    #[test]
    fn ir_constant() {
        let pf = parse_ir("TIMEOUT = 30\nMAX_RETRIES = 3\n");
        assert_eq!(pf.modules[0].constants.len(), 2);
    }

    #[test]
    fn ir_imports() {
        let pf = parse_ir("import os\nfrom typing import Optional, List\n");
        assert!(pf.modules[0].imports.len() >= 2);
    }

    /// `import numpy as np` parses to an `aliased_import`, and both import
    /// readers only matched `dotted_name` under `import_statement` — so the
    /// single most common form of a Python import produced no record at all.
    #[test]
    fn py_aliased_plain_import_is_recorded() {
        let pf = parse("import numpy as np\nimport os\n");
        assert_eq!(pf.imports.len(), 2, "`import numpy as np` must not vanish");
        assert_eq!(pf.imports[0].target_path, "numpy");
        assert_eq!(pf.imports[0].names, vec!["np"], "the alias is the name in scope");
    }

    #[test]
    fn ir_aliased_plain_import_is_recorded() {
        let pf = parse_ir("import pandas as pd\n");
        let imports = &pf.modules[0].imports;
        assert_eq!(imports.len(), 1, "`import pandas as pd` must not vanish");
        assert_eq!(imports[0].source, "pandas");
        assert_eq!(imports[0].names, vec!["pd"]);
    }

    /// `from a import b as c` binds `c`. Both readers recorded `b` — the one
    /// name that is *not* in scope.
    #[test]
    fn py_from_import_alias_keeps_both_names() {
        let pf = parse("from collections import OrderedDict as OD\n");
        assert_eq!(pf.imports[0].target_path, "collections");
        assert_eq!(
            pf.imports[0].names,
            vec!["OrderedDict as OD"],
            "the source name and the bound name are both facts"
        );
    }

    /// A dotted plain import binds its TOP name (`a`), which is what a later
    /// `a.b.c()` reference is looked up under.
    #[test]
    fn py_dotted_plain_import_binds_the_top_name() {
        let pf = parse("import os.path\n");
        assert_eq!(pf.imports[0].target_path, "os.path");
        assert_eq!(pf.imports[0].names, vec!["os"]);
    }
}
