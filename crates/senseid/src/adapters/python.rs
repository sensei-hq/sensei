use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport, SymbolKind};
use crate::ir::{IRBase, IRModule, IRFunction, IRClass, IRMethod, IRParam, IRImport, IRConstant, IRParsedFile, ClassKind, Visibility};
use super::common::{field_text, ir_function, ir_method, ir_class, ir_module, ir_parsed_file, node_text};
use super::LanguageAdapter;

pub struct PythonAdapter;

impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> &str { "python" }

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

        extract_symbols(&root, &lines, &mut symbols, None);
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
        None => return IRParsedFile { file_path: file_path.into(), language: "python".into(), ..Default::default() },
    };

    let lines: Vec<&str> = source.lines().collect();
    let root = tree.root_node();
    let src = source.as_bytes();

    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();
    let mut constants = Vec::new();

    walk_ir_py(&root, src, &lines, &mut functions, &mut classes, &mut imports, &mut constants, None);

    let is_test = file_path.contains("test") || source.contains("import pytest") || source.contains("import unittest");
    let module = ir_module(file_path, "python", functions, constants, imports, is_test);
    ir_parsed_file(file_path, "python", module, classes)
}

fn walk_ir_py(
    node: &Node, src: &[u8], lines: &[&str],
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
                            if let Some(cls) = (0..child.child_count())
                                .find_map(|j| child.child(j).filter(|c| c.kind() == "class_definition"))
                            {
                                let name = cls.child_by_field_name("name")
                                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                                    .unwrap_or_default();
                                let mut class = ir_class(name, &cls, ClassKind::Class,
                                    !cls.child_by_field_name("name").map(|n| n.utf8_text(src).unwrap_or_default().starts_with('_')).unwrap_or(false),
                                    extract_docstring(&cls, lines), collect_py_decorators(&child, src));
                                class.extends = extract_py_base_class(&cls, src);
                                if let Some(body) = cls.child_by_field_name("body") {
                                    walk_ir_py_methods(&body, src, lines, &mut class);
                                }
                                classes.push(class);
                            }
                            continue;
                        }
                    }
                } else {
                    (child, Vec::new())
                };

                let name = func_node.child_by_field_name("name")
                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let is_exported = !name.starts_with('_');
                let params = extract_py_params(&func_node, src);
                let return_type = extract_py_return_type(&func_node, src);
                let docstring = extract_docstring(&func_node, lines);
                let is_async = node_text(&func_node, src).starts_with("async ");
                let body_text = node_text(&func_node, src);

                if class_ctx.is_none() {
                    functions.push(ir_function(name, &func_node, lines, is_exported, is_async, params, return_type, docstring, decorators, &body_text));
                }
                // Methods are handled in walk_ir_py_methods
            }
            "class_definition" => {
                let name = child.child_by_field_name("name")
                    .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let is_exported = !name.starts_with('_');
                let mut class = ir_class(name, &child, ClassKind::Class, is_exported,
                    extract_docstring(&child, lines), Vec::new());
                class.extends = extract_py_base_class(&child, src);
                if let Some(body) = child.child_by_field_name("body") {
                    walk_ir_py_methods(&body, src, lines, &mut class);
                }
                classes.push(class);
            }
            "expression_statement" if class_ctx.is_none() => {
                if let Some(expr) = child.child(0) {
                    if expr.kind() == "assignment" {
                        if let Some(left) = expr.child_by_field_name("left") {
                            let name = left.utf8_text(src).unwrap_or_default().to_string();
                            if left.kind() == "identifier" && name == name.to_uppercase() && name.len() > 1 {
                                constants.push(IRConstant {
                                    base: IRBase {
                                        name, is_exported: true,
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
                }
            }
            "import_statement" | "import_from_statement" => {
                extract_py_imports(&child, src, imports);
            }
            _ => {}
        }
    }
}

fn walk_ir_py_methods(body: &Node, src: &[u8], lines: &[&str], class: &mut IRClass) {
    for i in 0..body.child_count() {
        let child = body.child(i).unwrap();
        let (func_node, decorators) = match child.kind() {
            "function_definition" => (child, Vec::new()),
            "decorated_definition" => {
                let decos = collect_py_decorators(&child, src);
                match (0..child.child_count()).find_map(|j| child.child(j).filter(|c| c.kind() == "function_definition")) {
                    Some(f) => (f, decos),
                    None => continue,
                }
            }
            _ => continue,
        };

        let name = func_node.child_by_field_name("name")
            .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
            .unwrap_or_default();
        let is_exported = !name.starts_with('_');
        let is_static = decorators.iter().any(|d| d.contains("staticmethod"));
        let is_async = node_text(&func_node, src).starts_with("async ");
        let body_text = node_text(&func_node, src);

        class.methods.push(ir_method(
            name, &func_node, is_exported, is_async, is_static,
            extract_py_params(&func_node, src),
            extract_py_return_type(&func_node, src),
            extract_docstring(&func_node, lines),
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
                            params.push(IRParam { name, type_: Some("Self".into()), ..Default::default() });
                        }
                    }
                    "typed_parameter" => {
                        let name = p.child_by_field_name("name")
                            .or_else(|| p.child(0))
                            .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                            .unwrap_or_default();
                        let type_ = p.child_by_field_name("type")
                            .map(|t| t.utf8_text(src).unwrap_or_default().to_string());
                        params.push(IRParam { name, type_, ..Default::default() });
                    }
                    "default_parameter" | "typed_default_parameter" => {
                        let name = p.child_by_field_name("name")
                            .or_else(|| p.child(0))
                            .map(|n| n.utf8_text(src).unwrap_or_default().to_string())
                            .unwrap_or_default();
                        let type_ = p.child_by_field_name("type")
                            .map(|t| t.utf8_text(src).unwrap_or_default().to_string());
                        let default = p.child_by_field_name("value")
                            .map(|v| v.utf8_text(src).unwrap_or_default().to_string());
                        params.push(IRParam { name, type_, default_value: default, is_optional: true });
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
        if let Some(c) = decorated_node.child(i) {
            if c.kind() == "decorator" {
                decos.push(c.utf8_text(src).unwrap_or_default().trim().to_string());
            }
        }
    }
    decos
}

fn extract_py_imports(node: &Node, src: &[u8], imports: &mut Vec<IRImport>) {
    match node.kind() {
        "import_statement" => {
            for j in 0..node.child_count() {
                if let Some(c) = node.child(j) {
                    if c.kind() == "dotted_name" {
                        let text = c.utf8_text(src).unwrap_or_default().to_string();
                        let name = text.rsplit('.').next().unwrap_or(&text).to_string();
                        imports.push(IRImport { source: text, names: vec![name], is_reexport: false });
                    }
                }
            }
        }
        "import_from_statement" => {
            let mut target = String::new();
            let mut names = Vec::new();
            for j in 0..node.child_count() {
                if let Some(c) = node.child(j) {
                    match c.kind() {
                        "dotted_name" | "relative_import" => {
                            let text = c.utf8_text(src).unwrap_or_default().to_string();
                            if target.is_empty() { target = text; } else { names.push(text); }
                        }
                        "aliased_import" => {
                            if let Some(n) = c.child_by_field_name("name") {
                                names.push(n.utf8_text(src).unwrap_or_default().to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            if !target.is_empty() {
                imports.push(IRImport { source: target, names, is_reexport: false });
            }
        }
        _ => {}
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

fn extract_symbols(node: &Node, lines: &[&str], symbols: &mut Vec<ParsedSymbol>, class_name: Option<&str>) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_definition" => {
                let name = child.child_by_field_name("name")
                    .map(|n| n.utf8_text(lines.join("\n").as_bytes()).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let kind = if class_name.is_some() { SymbolKind::Method } else { SymbolKind::Function };
                let sig = lines.get(child.start_position().row)
                    .map(|l| l.trim().to_string());
                let docstring = extract_docstring(&child, lines);
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
                let name = child.child_by_field_name("name")
                    .map(|n| n.utf8_text(lines.join("\n").as_bytes()).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let sig = lines.get(child.start_position().row)
                    .map(|l| l.trim().to_string());
                let docstring = extract_docstring(&child, lines);
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
                    extract_symbols(&body, lines, symbols, Some(&name));
                }
            }
            "expression_statement" if class_name.is_none() => {
                // Top-level constant: FOO = ...
                if let Some(expr) = child.child(0) {
                    if expr.kind() == "assignment" {
                        if let Some(left) = expr.child_by_field_name("left") {
                            let src = lines.join("\n");
                            let name = left.utf8_text(src.as_bytes()).unwrap_or_default().to_string();
                            if left.kind() == "identifier" && name == name.to_uppercase() && name.len() > 1 {
                                symbols.push(ParsedSymbol {
                                    name,
                                    kind: SymbolKind::Const,
                                    signature: lines.get(child.start_position().row).map(|l| l.trim().to_string()),
                                    docstring: None,
                                    line_start: child.start_position().row as u32 + 1,
                                    line_end: child.end_position().row as u32 + 1,
                                    is_exported: true,
                                    parent: None,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn extract_docstring(node: &Node, lines: &[&str]) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let first = body.child(0)?;
    if first.kind() != "expression_statement" { return None; }
    let str_node = first.child(0)?;
    if str_node.kind() != "string" { return None; }

    let src = lines.join("\n");
    let text = str_node.utf8_text(src.as_bytes()).ok()?;
    let trimmed = if text.starts_with("\"\"\"") || text.starts_with("'''") {
        text[3..text.len()-3].trim()
    } else if text.starts_with('"') || text.starts_with('\'') {
        text[1..text.len()-1].trim()
    } else {
        text.trim()
    };
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn extract_imports(root: &Node, src: &[u8], imports: &mut Vec<ParsedImport>) {
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        match child.kind() {
            "import_statement" => {
                for j in 0..child.child_count() {
                    let c = child.child(j).unwrap();
                    if c.kind() == "dotted_name" {
                        let text = c.utf8_text(src).unwrap_or_default().to_string();
                        let name = text.rsplit('.').next().unwrap_or(&text).to_string();
                        imports.push(ParsedImport { target_path: text, names: vec![name] });
                    }
                }
            }
            "import_from_statement" => {
                let mut target = String::new();
                let mut names = Vec::new();
                for j in 0..child.child_count() {
                    let c = child.child(j).unwrap();
                    match c.kind() {
                        "dotted_name" | "relative_import" => {
                            let text = c.utf8_text(src).unwrap_or_default().to_string();
                            if target.is_empty() {
                                target = text;
                            } else {
                                names.push(text);
                            }
                        }
                        "aliased_import" => {
                            if let Some(n) = c.child_by_field_name("name") {
                                names.push(n.utf8_text(src).unwrap_or_default().to_string());
                            }
                        }
                        _ => {}
                    }
                }
                if !target.is_empty() {
                    imports.push(ParsedImport { target_path: target, names });
                }
            }
            _ => {}
        }
    }
}

fn extract_edges(root: &Node, symbols: &[ParsedSymbol]) -> Vec<ParsedEdge> {
    let known_names: std::collections::HashSet<&str> = symbols.iter()
        .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
        .map(|s| s.name.as_str())
        .collect();

    let mut edges = Vec::new();
    for sym in symbols {
        if !matches!(sym.kind, SymbolKind::Function | SymbolKind::Method) { continue; }
        // Walk the tree to find call expressions within this symbol's range
        find_calls(root, sym, &known_names, &mut edges);
    }
    edges
}

fn find_calls(node: &Node, caller: &ParsedSymbol, known: &std::collections::HashSet<&str>, edges: &mut Vec<ParsedEdge>) {
    if node.kind() == "call" {
        if let Some(func) = node.child_by_field_name("function") {
            if func.kind() == "identifier" {
                // We need the text — for now skip (requires source bytes)
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> ParsedFile {
        PythonAdapter.parse(source, "test.py")
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
        let pf = parse("class Foo:\n    def bar(self):\n        pass\n    def _private(self):\n        pass\n");
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
            "class UserService:\n    \"\"\"Manages users.\"\"\"\n    def __init__(self, db):\n        self.db = db\n    def get_user(self, uid):\n        \"\"\"Fetch user.\"\"\"\n        return self.db.query(uid)\n"
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
        let pf = parse("class Dog:\n    def bark(self):\n        pass\n    def sit(self):\n        pass\n");
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
            "class UserService:\n    def __init__(self, db):\n        self.db = db\n    def get_user(self, uid):\n        return self.db.query(uid)\n"
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

    fn parse_ir(src: &str) -> IRParsedFile { parse_to_ir(src, "test.py") }

    #[test]
    fn ir_function_with_typed_params() {
        let pf = parse_ir("def hello(name: str, count: int = 5) -> str:\n    return name * count\n");
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
        let pf = parse_ir("class Dog(Animal):\n    \"\"\"A dog.\"\"\"\n    def bark(self) -> str:\n        return 'woof'\n");
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.classes[0].base.name, "Dog");
        assert_eq!(pf.classes[0].extends, Some("Animal".into()));
        assert_eq!(pf.classes[0].base.docstring, Some("A dog.".into()));
        assert!(pf.classes[0].methods.len() >= 1);
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
}
