use super::LanguageAdapter;
use super::common::{ir_module, ir_parsed_file};
use crate::ir::{ClassKind, IRBase, IRClass, IRFunction, IRParsedFile};
use crate::types::{ParsedFile, ParsedSymbol, SymbolKind};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser as SqlParser;

pub struct SqlAdapter;

impl LanguageAdapter for SqlAdapter {
    fn supports_fqn(&self) -> bool {
        true
    }

    fn extensions(&self) -> &[&'static str] {
        &[".sql", ".ddl"]
    }

    fn language(&self) -> &str {
        "sql"
    }
    fn display_name(&self) -> &str {
        "SQL"
    }

    fn fqn_output(
        &self,
        abs_path: &str,
        _rel_path: &str,
        content: &str,
    ) -> Option<super::fqn::FqnFileOutput> {
        Some(sql_fqn::produce_fqns(content, &sql_fqn::schema_from_path(abs_path)))
    }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

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
            } else if upper.contains("FUNCTION")
                && upper.contains("CREATE")
                && let Some(name) = extract_name_after(&text, "FUNCTION")
            {
                let line = find_line(&lines, "FUNCTION", &name);
                symbols.push(make_sym(name, SymbolKind::Function, &lines, line));
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
    ParsedFile {
        file_path: path.into(),
        language: "sql".into(),
        symbols: vec![],
        edges: vec![],
        imports: vec![],
    }
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
        parent: None,
    }
}

fn extract_name_after(stmt: &str, keyword: &str) -> Option<String> {
    let upper = stmt.to_uppercase();
    let pos = upper.find(keyword)?;
    let after = &stmt[pos + keyword.len()..];
    let trimmed = after
        .trim()
        .trim_start_matches("IF NOT EXISTS")
        .trim()
        .trim_start_matches("OR REPLACE")
        .trim()
        .trim_start_matches("UNIQUE")
        .trim();
    let name =
        trimmed.split(|c: char| c.is_whitespace() || c == '(').next().unwrap_or("").to_string();
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
    if line <= 1 {
        return None;
    }
    let mut comments = Vec::new();
    let mut idx = line as usize - 2;
    while let Some(l) = lines.get(idx) {
        let trimmed = l.trim();
        if trimmed.starts_with("--") {
            comments.push(trimmed.trim_start_matches("--").trim().to_string());
        } else {
            break;
        }
        if idx == 0 {
            break;
        }
        idx -= 1;
    }
    if comments.is_empty() {
        return None;
    }
    comments.reverse();
    Some(comments.join("\n"))
}

/// Parse SQL into IR — tables as classes, functions/procedures as functions.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let pf = SqlAdapter.parse(source, file_path);
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let constants = Vec::new();

    for sym in &pf.symbols {
        match sym.kind {
            SymbolKind::Class => {
                // tables, views
                classes.push(IRClass {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start,
                        line_end: sym.line_end,
                        is_exported: true,
                        node_type: Some("class".into()),
                        ..Default::default()
                    },
                    class_kind: ClassKind::Class,
                    ..Default::default()
                });
            }
            SymbolKind::Function => {
                functions.push(IRFunction {
                    base: IRBase {
                        name: sym.name.clone(),
                        line_start: sym.line_start,
                        line_end: sym.line_end,
                        is_exported: true,
                        node_type: Some("function".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            }
            _ => {}
        }
    }

    let module = ir_module(file_path, "sql", functions, constants, Vec::new(), false);
    ir_parsed_file(file_path, "sql", module, classes)
}

// ── FQN producer (plan Phase 6.7) ────────────────────────────────────────────
// SQL's "package" is the schema. A text scan (robust to the PG-specific DDL that a
// generic SQL grammar rejects) maps `create <obj> [schema.]<name>` → sql·schema·name
// and `references [schema.]<table>` (foreign keys) → resolved node→node edges,
// attributed to the enclosing CREATE object.
pub(crate) mod sql_fqn {
    use super::super::fqn::{self, FqnDefinition, FqnFileOutput, FqnReference};
    use crate::types::SymbolKind;

    const SQL_LANG: &str = "sql";
    // Object kinds we track as definitions (the leading create-modifiers are stripped first).
    fn kind_of(word: &str) -> Option<SymbolKind> {
        match word {
            "table" => Some(SymbolKind::Class),
            "view" => Some(SymbolKind::Type),
            "function" | "procedure" => Some(SymbolKind::Function),
            "type" | "domain" => Some(SymbolKind::Type),
            "index" | "trigger" | "sequence" => Some(SymbolKind::Const),
            _ => None,
        }
    }

    /// Schema from `set search_path to <schema>` (the sensei DDL idiom); falls back
    /// to the caller-supplied default (the dbd `ddl/<type>/<schema>/` dir name).
    fn schema_of(source: &str, default_schema: &str) -> String {
        for line in source.lines() {
            let lower = line.to_lowercase();
            if let Some(pos) = lower.find("search_path to ") {
                let after = &line[pos + "search_path to ".len()..];
                let sch = after
                    .trim()
                    .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                    .next()
                    .unwrap_or("")
                    .trim();
                if !sch.is_empty() {
                    return sch.to_string();
                }
            }
        }
        default_schema.to_string()
    }

    /// Split `schema.name` → (schema, name); a bare name uses the file's schema.
    fn split_qualified(name: &str, default_schema: &str) -> (String, String) {
        match name.rsplit_once('.') {
            Some((s, n)) => (s.trim_matches('"').to_string(), n.trim_matches('"').to_string()),
            None => (default_schema.to_string(), name.trim_matches('"').to_string()),
        }
    }

    /// Parse a `create …` line into (kind_word, object_name), skipping the
    /// or-replace / unique / materialized modifiers and `if not exists`.
    fn parse_create(line: &str) -> Option<(SymbolKind, String)> {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.first().map(|w| w.to_lowercase()) != Some("create".to_string()) {
            return None;
        }
        let mut i = 1;
        while i < words.len()
            && ["or", "replace", "unique", "materialized"]
                .contains(&words[i].to_lowercase().as_str())
        {
            i += 1;
        }
        let kind = kind_of(&words.get(i)?.to_lowercase())?;
        i += 1;
        while i < words.len() && ["if", "not", "exists"].contains(&words[i].to_lowercase().as_str())
        {
            i += 1;
        }
        let name = words.get(i)?.split('(').next().unwrap_or("").trim_matches('"').to_string();
        if name.is_empty() { None } else { Some((kind, name)) }
    }

    /// Every `references <name>` target on a line (foreign keys).
    fn references_on(line: &str) -> Vec<String> {
        let lower = line.to_lowercase();
        let mut out = Vec::new();
        let mut from = 0;
        while let Some(pos) = lower[from..].find("references ") {
            let start = from + pos + "references ".len();
            let name = line[start..]
                .trim()
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or("")
                .trim_matches('"');
            if !name.is_empty()
                && name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_' || c == '"')
            {
                out.push(name.to_string());
            }
            from = start;
        }
        out
    }

    /// Produce canonical FQNs for a SQL file. `default_schema` is used when the file
    /// has no `set search_path` (the dbd directory schema).
    pub fn produce_fqns(source: &str, default_schema: &str) -> FqnFileOutput {
        let schema = schema_of(source, default_schema);
        let mut out =
            FqnFileOutput { package: schema.clone(), module: String::new(), ..Default::default() };
        let mut current: Option<String> = None; // fqn of the enclosing CREATE object
        for (i, line) in source.lines().enumerate() {
            // Skip `--` comments so a prose mention of "references"/"create" isn't parsed.
            if line.trim_start().starts_with("--") {
                continue;
            }
            if let Some((kind, raw)) = parse_create(line) {
                let (sch, name) = split_qualified(&raw, &schema);
                let fqn_str = fqn::item(SQL_LANG, &sch, "", &name);
                out.defs.push(FqnDefinition {
                    fqn: fqn_str.clone(),
                    name,
                    kind,
                    line_start: i as u32 + 1,
                    line_end: i as u32 + 1,
                    is_exported: true,
                    signature: Some(line.trim().to_string()),
                    docstring: None,
                    parent_type: None,
                    parent_fqn: None,
                });
                current = Some(fqn_str);
            }
            if let Some(caller) = &current {
                for target in references_on(line) {
                    let (tsch, tname) = split_qualified(&target, &schema);
                    out.refs.push(FqnReference {
                        caller_fqn: caller.clone(),
                        caller_line: i as u32 + 1,
                        target_fqn: Some(fqn::item(SQL_LANG, &tsch, "", &tname)),
                        target_name: tname,
                        is_lib: false,
                    });
                }
            }
        }
        out
    }

    /// The schema a SQL file belongs to when it has no `set search_path`: the dbd
    /// layout is `…/ddl/<type>/<schema>/<name>.ddl`, so the parent directory names it.
    pub(crate) fn schema_from_path(abs_path: &str) -> String {
        std::path::Path::new(abs_path)
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("public")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        SqlAdapter.parse(src, "schema.sql")
    }

    // ── FQN producer (Phase 6.7) ────────────────────────────────────────────
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
    fn sql_def_fqn() {
        let out = sql_fqn::produce_fqns(
            "set search_path to sensei, extensions;\ncreate table if not exists nodes (id uuid);\ncreate view active_nodes as select * from nodes;",
            "public",
        );
        assert_eq!(
            def_fqn(&out, "nodes"),
            "sql·sensei·nodes",
            "table → schema.name (schema from search_path)"
        );
        assert_eq!(def_fqn(&out, "active_nodes"), "sql·sensei·active_nodes", "view");
    }

    #[test]
    fn sql_ref_fqn() {
        let out = sql_fqn::produce_fqns(
            "set search_path to sensei;\ncreate table edges (\n  source_id uuid references nodes(id)\n);",
            "public",
        );
        let r = ref_to(&out, "nodes");
        assert_eq!(
            r.target_fqn.as_deref(),
            Some("sql·sensei·nodes"),
            "foreign key resolves to the referenced table's fqn"
        );
        assert_eq!(r.caller_fqn, "sql·sensei·edges", "attributed to the enclosing table");
    }

    #[test]
    fn sql_schema_from_path_uses_dbd_layout() {
        assert_eq!(sql_fqn::schema_from_path("/x/database/ddl/table/sensei/nodes.ddl"), "sensei");
    }

    #[test]
    fn sql_producer_handles_real_ddl() {
        // Exercise the producer against a real, PG-specific DDL file (the corpus the
        // text scan exists for) — a generic SQL grammar would reject it wholesale.
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("database/ddl/table/sensei/nodes.ddl");
        if !root.exists() {
            return;
        }
        let content = std::fs::read_to_string(&root).unwrap();
        let out = sql_fqn::produce_fqns(&content, "sensei");
        assert!(
            out.defs.iter().any(|d| d.fqn == "sql·sensei·nodes"),
            "the nodes table def, got: {:?}",
            out.defs.iter().map(|d| &d.fqn).collect::<Vec<_>>()
        );
        assert!(
            out.refs.iter().any(|r| r.target_fqn.as_deref() == Some("sql·sensei·folders")),
            "the folder_id → sensei.folders foreign-key edge, got: {:?}",
            out.refs.iter().map(|r| &r.target_fqn).collect::<Vec<_>>()
        );
    }

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
        let pf = parse(
            "CREATE TABLE a (id INT);\nCREATE TABLE b (id INT);\nCREATE VIEW v AS SELECT * FROM a;",
        );
        assert_eq!(pf.symbols.len(), 3);
    }
}
