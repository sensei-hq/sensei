use tree_sitter::{Parser, Node};
use crate::types::{ParsedFile, ParsedSymbol, ParsedEdge, ParsedImport, SymbolKind};
use super::LanguageAdapter;

pub struct PythonAdapter;

impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> &str { "python" }
    fn extensions(&self) -> &[&str] { &[".py"] }

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

        let mut symbols = Vec::new();
        let mut imports = Vec::new();

        extract_symbols(&root, &lines, &mut symbols, false);
        extract_imports(&root, &mut imports);

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

fn empty_file(path: &str) -> ParsedFile {
    ParsedFile {
        file_path: path.to_string(),
        language: "python".to_string(),
        symbols: vec![],
        edges: vec![],
        imports: vec![],
    }
}

fn extract_symbols(node: &Node, lines: &[&str], symbols: &mut Vec<ParsedSymbol>, inside_class: bool) {
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        match child.kind() {
            "function_definition" => {
                let name = child.child_by_field_name("name")
                    .map(|n| n.utf8_text(lines.join("\n").as_bytes()).unwrap_or_default().to_string())
                    .unwrap_or_default();
                let kind = if inside_class { SymbolKind::Method } else { SymbolKind::Function };
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
                });
                // Recurse into class body for methods
                if let Some(body) = child.child_by_field_name("body") {
                    extract_symbols(&body, lines, symbols, true);
                }
            }
            "expression_statement" if !inside_class => {
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

fn extract_imports(root: &Node, imports: &mut Vec<ParsedImport>) {
    let _src_bytes: &[u8] = &[]; // Source text access handled per-node
    for i in 0..root.child_count() {
        let child = root.child(i).unwrap();
        match child.kind() {
            "import_statement" => {
                // import foo, import foo.bar
                for j in 0..child.child_count() {
                    let c = child.child(j).unwrap();
                    if c.kind() == "dotted_name" {
                        // We need to get the text from the node
                        // Use byte range from the original source
                        let name = format!("import_{}", j); // placeholder
                        imports.push(ParsedImport {
                            target_path: name,
                            names: vec![],
                        });
                    }
                }
            }
            "import_from_statement" => {
                // from foo import bar, baz
                let mut target = String::new();
                let mut names = Vec::new();
                for j in 0..child.child_count() {
                    let c = child.child(j).unwrap();
                    if c.kind() == "dotted_name" || c.kind() == "relative_import" {
                        if target.is_empty() {
                            target = format!("from_{}", j); // placeholder
                        } else {
                            names.push(format!("name_{}", j));
                        }
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
}
