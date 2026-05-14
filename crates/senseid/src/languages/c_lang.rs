use crate::types::{ParsedFile, ParsedSymbol, SymbolKind};
use crate::ir::{IRBase, IRFunction, IRClass, IRParsedFile, ClassKind};
use super::common::{ir_module, ir_parsed_file};
use super::LanguageAdapter;

pub struct CAdapter;

impl LanguageAdapter for CAdapter {
    fn language(&self) -> &str { "c" }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        // Skip very large generated files (e.g. tree-sitter parsers)
        if source.len() > 500_000 {
            return ParsedFile {
                file_path: file_path.into(),
                language: "c".into(),
                symbols: vec![],
                edges: vec![],
                imports: vec![],
            };
        }

        let mut symbols = Vec::new();

        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            let line_num = (i + 1) as u32;

            // Function definitions: type name(params) {
            // Match patterns like: void foo(int x) {  or  int main() {
            if !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with("*")
                && !trimmed.starts_with("#")
                && trimmed.contains('(') && (trimmed.ends_with('{') || trimmed.ends_with(") {"))
                && let Some(name) = extract_c_function_name(trimmed)
                    && !name.is_empty() && name.len() < 100 && !name.contains(' ') {
                        symbols.push(ParsedSymbol {
                            name,
                            kind: SymbolKind::Function,
                            signature: Some(trimmed.trim_end_matches('{').trim().to_string()),
                            docstring: None,
                            line_start: line_num,
                            line_end: line_num,
                            is_exported: true,
                            parent: None,
                        });
                    }

            // Struct definitions: struct Name {
            if trimmed.starts_with("struct ") && trimmed.contains('{') {
                let name = trimmed.strip_prefix("struct ")
                    .and_then(|s| s.split(|c: char| !c.is_alphanumeric() && c != '_').next())
                    .unwrap_or("")
                    .to_string();
                if !name.is_empty() {
                    symbols.push(ParsedSymbol {
                        name, kind: SymbolKind::Struct,
                        signature: Some(trimmed.to_string()),
                        docstring: None,
                        line_start: line_num, line_end: line_num,
                        is_exported: true, parent: None,
                    });
                }
            }

            // Typedef
            if trimmed.starts_with("typedef ") {
                // typedef struct ... Name; or typedef type Name;
                if let Some(name) = trimmed.strip_prefix("typedef ")
                    .and_then(|s| s.trim_end_matches(';').rsplit_once(|c: char| c.is_whitespace()))
                    .map(|(_, name)| name.trim().to_string())
                    && !name.is_empty() && !name.contains('{') {
                        symbols.push(ParsedSymbol {
                            name, kind: SymbolKind::Type,
                            signature: Some(trimmed.to_string()),
                            docstring: None,
                            line_start: line_num, line_end: line_num,
                            is_exported: true, parent: None,
                        });
                    }
            }

            // #define constants
            if trimmed.starts_with("#define ") {
                let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    let name = parts[1].split('(').next().unwrap_or("").to_string();
                    if !name.is_empty() && name == name.to_uppercase() {
                        symbols.push(ParsedSymbol {
                            name, kind: SymbolKind::Const,
                            signature: Some(trimmed.to_string()),
                            docstring: None,
                            line_start: line_num, line_end: line_num,
                            is_exported: true, parent: None,
                        });
                    }
                }
            }
        }

        // Extract #include as imports
        let imports = source.lines()
            .filter(|l| l.trim().starts_with("#include"))
            .filter_map(|l| {
                let path = l.trim().strip_prefix("#include")?
                    .trim()
                    .trim_start_matches(['<', '"'])
                    .trim_end_matches(['>', '"'])
                    .to_string();
                Some(crate::types::ParsedImport { target_path: path.clone(), names: vec![path] })
            })
            .collect();

        ParsedFile {
            file_path: file_path.into(),
            language: "c".into(),
            symbols,
            edges: vec![],
            imports,
        }
    }
}

fn extract_c_function_name(line: &str) -> Option<String> {
    // Find the function name before the opening paren
    let paren_pos = line.find('(')?;
    let before_paren = line[..paren_pos].trim();
    // Last word before ( is the function name
    let name = before_paren.rsplit(|c: char| c.is_whitespace() || c == '*')
        .next()
        .unwrap_or("")
        .to_string();
    if name.is_empty() || name.starts_with('#') || ["if", "while", "for", "switch", "return"].contains(&name.as_str()) {
        None
    } else {
        Some(name)
    }
}

/// Parse C/C++ into IR — functions and structs.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let pf = CAdapter.parse(source, file_path);
    let mut functions = Vec::new();
    let mut classes = Vec::new();

    for sym in &pf.symbols {
        match sym.kind {
            SymbolKind::Function => {
                functions.push(IRFunction {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start, line_end: sym.line_end,
                        docstring: sym.docstring.clone(),
                        is_exported: sym.is_exported,
                        node_type: Some("function".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
            SymbolKind::Class => {
                classes.push(IRClass {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start, line_end: sym.line_end,
                        is_exported: sym.is_exported,
                        node_type: Some("class".into()),
                        ..Default::default()
                    },
                    class_kind: ClassKind::Struct,
                    ..Default::default()
                });
            }
            _ => {}
        }
    }

    let module = ir_module(file_path, "c", functions, Vec::new(), Vec::new(), file_path.contains("test"));
    ir_parsed_file(file_path, "c", module, classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile { CAdapter.parse(src, "test.c") }

    #[test]
    fn parses_function() {
        let pf = parse("int main(int argc, char **argv) {\n  return 0;\n}");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "main");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Function);
    }

    #[test]
    fn parses_struct() {
        let pf = parse("struct Point {\n  int x;\n  int y;\n};");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "Point");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Struct);
    }

    #[test]
    fn parses_define() {
        let pf = parse("#define MAX_SIZE 100\n#define min(a,b) ((a)<(b)?(a):(b))");
        let consts: Vec<_> = pf.symbols.iter().filter(|s| s.kind == SymbolKind::Const).collect();
        assert_eq!(consts.len(), 1);
        assert_eq!(consts[0].name, "MAX_SIZE");
    }

    #[test]
    fn parses_includes() {
        let pf = parse("#include <stdio.h>\n#include \"mylib.h\"\nint main() {");
        assert_eq!(pf.imports.len(), 2);
        assert_eq!(pf.imports[0].target_path, "stdio.h");
        assert_eq!(pf.imports[1].target_path, "mylib.h");
    }

    #[test]
    fn skips_large_generated_files() {
        let large = "a\n".repeat(300_000);
        let pf = parse(&large);
        assert!(pf.symbols.is_empty(), "should skip large generated files");
    }

    #[test]
    fn language() {
        let pf = parse("int x;");
        assert_eq!(pf.language, "c");
    }
}
