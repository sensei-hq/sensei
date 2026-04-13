use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser as SqlParser;
use crate::types::{ParsedFile, ParsedSymbol, SymbolKind};
use super::LanguageAdapter;

pub struct SqlAdapter;

impl LanguageAdapter for SqlAdapter {
    fn language(&self) -> &str { "sql" }
    fn extensions(&self) -> &[&str] { &[".sql"] }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let dialect = GenericDialect {};
        let stmts = match SqlParser::parse_sql(&dialect, source) {
            Ok(s) => s,
            Err(_) => return empty(file_path),
        };

        let lines: Vec<&str> = source.lines().collect();
        let mut symbols = Vec::new();

        for stmt in &stmts {
            let text = stmt.to_string();
            let upper = text.to_uppercase();

            if upper.starts_with("CREATE TABLE") {
                if let Some(name) = extract_name_after(&text, "TABLE") {
                    let line = find_line(&lines, "CREATE TABLE", &name);
                    symbols.push(make_sym(name, SymbolKind::Class, &lines, line));
                }
            } else if upper.contains("VIEW") && upper.contains("CREATE") {
                if let Some(name) = extract_name_after(&text, "VIEW") {
                    let line = find_line(&lines, "VIEW", &name);
                    symbols.push(make_sym(name, SymbolKind::Type, &lines, line));
                }
            } else if upper.contains("INDEX") && upper.contains("CREATE") {
                if let Some(name) = extract_name_after(&text, "INDEX") {
                    let line = find_line(&lines, "INDEX", &name);
                    symbols.push(make_sym(name, SymbolKind::Const, &lines, line));
                }
            } else if upper.contains("FUNCTION") && upper.contains("CREATE") {
                if let Some(name) = extract_name_after(&text, "FUNCTION") {
                    let line = find_line(&lines, "FUNCTION", &name);
                    symbols.push(make_sym(name, SymbolKind::Function, &lines, line));
                }
            }
        }

        ParsedFile {
            file_path: file_path.to_string(),
            language: "sql".to_string(),
            symbols,
            edges: vec![],
            imports: vec![],
        }
    }
}

fn empty(path: &str) -> ParsedFile {
    ParsedFile { file_path: path.into(), language: "sql".into(), symbols: vec![], edges: vec![], imports: vec![] }
}

fn make_sym(name: String, kind: SymbolKind, lines: &[&str], line: u32) -> ParsedSymbol {
    ParsedSymbol {
        name,
        kind,
        signature: lines.get(line.saturating_sub(1) as usize).map(|l| l.trim().to_string()),
        docstring: find_preceding_comment(lines, line),
        line_start: line,
        line_end: line,
        is_exported: true,
    }
}

fn extract_name_after(stmt: &str, keyword: &str) -> Option<String> {
    let upper = stmt.to_uppercase();
    let pos = upper.find(keyword)?;
    let after = &stmt[pos + keyword.len()..];
    let trimmed = after.trim()
        .trim_start_matches("IF NOT EXISTS").trim()
        .trim_start_matches("OR REPLACE").trim()
        .trim_start_matches("UNIQUE").trim();
    let name = trimmed.split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("")
        .to_string();
    if name.is_empty() { None } else { Some(name) }
}

fn find_line(lines: &[&str], keyword: &str, name: &str) -> u32 {
    let kw = keyword.to_uppercase();
    let nm = name.to_uppercase();
    for (i, line) in lines.iter().enumerate() {
        let upper = line.to_uppercase();
        if upper.contains(&kw) && upper.contains(&nm) {
            return i as u32 + 1;
        }
    }
    1
}

fn find_preceding_comment(lines: &[&str], line: u32) -> Option<String> {
    if line <= 1 { return None; }
    let mut comments = Vec::new();
    let mut idx = line as usize - 2;
    loop {
        if let Some(l) = lines.get(idx) {
            let trimmed = l.trim();
            if trimmed.starts_with("--") {
                comments.push(trimmed.trim_start_matches("--").trim().to_string());
            } else {
                break;
            }
        } else {
            break;
        }
        if idx == 0 { break; }
        idx -= 1;
    }
    if comments.is_empty() { return None; }
    comments.reverse();
    Some(comments.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile { SqlAdapter.parse(src, "schema.sql") }

    #[test]
    fn parses_create_table() {
        let pf = parse("CREATE TABLE users (\n  id TEXT PRIMARY KEY,\n  name TEXT\n);");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].name, "users");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Class);
    }

    #[test]
    fn parses_create_view() {
        let pf = parse("CREATE VIEW active_users AS SELECT * FROM users WHERE active = 1;");
        assert_eq!(pf.symbols.len(), 1);
        assert_eq!(pf.symbols[0].kind, SymbolKind::Type);
    }

    #[test]
    fn parses_create_index() {
        let pf = parse("CREATE TABLE t (id INT);\nCREATE INDEX idx_t ON t(id);");
        let indexes: Vec<_> = pf.symbols.iter().filter(|s| s.kind == SymbolKind::Const).collect();
        assert_eq!(indexes.len(), 1);
        assert_eq!(indexes[0].name, "idx_t");
    }

    #[test]
    fn parses_comment_as_docstring() {
        let pf = parse("-- User accounts table\nCREATE TABLE users (id INT);");
        assert_eq!(pf.symbols[0].docstring, Some("User accounts table".to_string()));
    }

    #[test]
    fn handles_invalid_sql() {
        let pf = parse("THIS IS NOT SQL AT ALL");
        assert!(pf.symbols.is_empty());
    }

    #[test]
    fn multiple_statements() {
        let pf = parse("CREATE TABLE a (id INT);\nCREATE TABLE b (id INT);\nCREATE VIEW v AS SELECT * FROM a;");
        assert_eq!(pf.symbols.len(), 3);
    }
}
