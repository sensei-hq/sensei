//! The T-SQL walk — every declaration and every use site, from a token stream.
//!
//! # What this reads, and what it deliberately does not
//!
//! It reads STATEMENT HEADS. `CREATE PROCEDURE [dbo].[sp_X]` declares an
//! object; `FROM [dbo].[Issues]` refers to one. Both are a keyword followed by
//! a qualified name, and that is the whole grammar this needs.
//!
//! It does NOT read expressions, column lists, control flow or types. A
//! T-SQL change script has no nested scopes, no overloads and no generics —
//! the structure that makes a real parser necessary for Rust or C# is absent,
//! and pretending to more than the head would be inventing it.
//!
//! COLUMNS ARE NOT DECLARED here, and that is a stated limit rather than an
//! oversight: a column reference in this corpus is almost never qualified
//! enough to resolve (`SELECT Name FROM Issues` says nothing about which
//! table `Name` is on), so declaring columns would create thousands of nodes
//! that no reference could ever reach.
//!
//! # A statement ends where the next one begins
//!
//! T-SQL does not require `;`, and most of this corpus omits it. So the reader
//! does not look for statement ends at all — it scans for statement HEADS and
//! ignores everything between them. A head it does not know is not an error;
//! it is a statement this reader has nothing to say about.
//!
//! # Which symbol a reference belongs to
//!
//! Inside `CREATE PROCEDURE X AS …`, every table the body touches is X's. A
//! T-SQL procedure body runs to the end of its batch, so the reader attributes
//! references to the most recent declaration in the batch and falls back to the
//! FILE for a script that declares nothing — which is the ordinary shape of a
//! migration.

use std::collections::BTreeSet;

use super::lex::{self, Tok};
use super::{Dialect, SqlAdapter};
use crate::indexer::facts::{
    DeclaredType, Evidence, FileFacts, Fqn, Language, Reason, RefKind, Reference, Relation,
    RelationKind, Resolution, Rung, Span, Symbol, SymbolKind, Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};
use crate::indexer::lang::{LanguageAdapter, ReadError, Source, TypeHomes};

/// The object kinds a `CREATE` can declare, and what each is in the shared
/// vocabulary.
///
/// A VIEW takes [`SymbolKind::Struct`] like a table, and that is the same call
/// C's walk makes for a union: both are RELATIONS, the difference between
/// stored and derived is a property of the object rather than a different kind
/// of declaration, and a SQL-only `View` variant would put one language's
/// vocabulary on every other language's symbols.
fn object_kind(word: &Tok<'_>) -> Option<Object> {
    for (keyword, kind, label, alter_defines) in [
        ("procedure", SymbolKind::Function, "procedure", true),
        ("proc", SymbolKind::Function, "procedure", true),
        ("function", SymbolKind::Function, "function", true),
        ("trigger", SymbolKind::Function, "trigger", true),
        ("view", SymbolKind::Struct, "view", true),
        ("table", SymbolKind::Struct, "table", false),
        ("type", SymbolKind::TypeAlias, "type", false),
        ("synonym", SymbolKind::TypeAlias, "synonym", false),
    ] {
        if word.is(keyword) {
            return Some(Object { kind, label, alter_defines });
        }
    }
    None
}

/// A qualified name the reader has just read.
///
/// The schema is an `Option` rather than an empty string, and that is the
/// point: "the source did not qualify this" is a FACT the caller acts on — an
/// unqualified name depends on the connection's default schema, which no file
/// states — and an empty string is a value a caller cannot tell from a schema
/// genuinely spelled that way.
struct Qualified {
    /// The schema the source wrote, EMPTY when it wrote none.
    module: String,
    /// **Whether the source qualified the name at all**, kept apart from an
    /// empty `module` rather than inferred from it.
    ///
    /// Two different facts: an unqualified object still HAS a module in its
    /// identity — the empty one, which is what the source said — but it must
    /// not be PLACED, because which schema it resolves to at deploy time
    /// depends on the connection's default and no file states that. Reading
    /// the absence off an empty string would conflate them.
    qualified: bool,
    object: String,
    /// The token index just past the name.
    next: usize,
}

/// What a `CREATE` or `ALTER` head names.
#[derive(Clone, Copy)]
struct Object {
    kind: SymbolKind,
    label: &'static str,
    /// **Whether `ALTER` on this kind carries the WHOLE definition.**
    ///
    /// T-SQL's `ALTER PROCEDURE` syntax REQUIRES the complete body — it
    /// replaces the object rather than editing it — and the same holds for
    /// `ALTER VIEW`, `ALTER FUNCTION` and `ALTER TRIGGER`. So a file whose only
    /// statement is one of those DECLARES the object; its body is right there.
    ///
    /// `ALTER TABLE` never carries a definition. It is `ADD`, `DROP` or `ALTER
    /// COLUMN` — an edit to a table defined elsewhere.
    ///
    /// Reading every `ALTER` alike gets one of the two wrong whichever way it
    /// goes, and MEASURED over Ethico both shapes are common: 907 procedures
    /// are shipped as `ALTER PROCEDURE` per release folder, while `ALTER TABLE`
    /// outnumbers `CREATE TABLE` 159 to 101.
    alter_defines: bool,
}

/// Read one T-SQL file.
pub fn read(source: &Source<'_>, _types: &TypeHomes) -> Result<FileFacts, ReadError> {
    // REFUSED unless the source says T-SQL. Handing a Postgres file to a
    // T-SQL reader would read `$$`-quoted bodies as statements and mint
    // declarations out of them — facts invented from the wrong grammar, which
    // is the failure `is_cpp` exists to prevent in C.
    //
    // `Unstated` is refused too: a file that names no dialect has not been
    // shown to be this one.
    let dialect = Dialect::detect(source.text);
    if dialect != Dialect::TSql {
        return Err(ReadError::GrammarUnavailable(format!(
            "{}: this reader is T-SQL and the file states {dialect:?}",
            source.path
        )));
    }

    let file = SqlAdapter
        .file_fqn(source.package, source.module, source.path)
        .map_err(ReadError::NoFileIdentity)?;

    let mut walk = Walk {
        package: source.package,
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        declared: BTreeSet::new(),
        file: file.clone(),
    };

    for (line, batch) in lex::batches(source.text) {
        walk.batch(&lex::tokens(batch), line);
    }

    walk.symbols.insert(
        0,
        crate::indexer::lang::common::file_module(file, stem_of(source.path), source.text),
    );

    Ok(FileFacts {
        language: Language::Sql,
        package: source.package.to_string(),
        module: source.module.to_string(),
        path: source.path.to_string(),
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: Vec::new(),
    })
}

fn stem_of(path: &str) -> &str {
    let f = path.rsplit('/').next().unwrap_or(path);
    f.rsplit_once('.').map_or(f, |(head, _)| head)
}

/// A span covering the statement's line.
///
/// LINE-GRAINED, because the token stream has no byte offsets back into the
/// file once a batch has been split out. The line is what a reader clicks, and
/// a column pointing into a reconstructed fragment would be worse than none.
fn at(line: u32) -> Span {
    Span { start_line: line, start_col: 1, end_line: line, end_col: 1 }
}

struct Walk<'a> {
    package: &'a str,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
    relations: Vec<Relation>,
    /// Identities this file has already declared, so a script that creates one
    /// object twice does not mint it twice.
    declared: BTreeSet<String>,
    file: Fqn,
}

impl<'a> Walk<'a> {
    /// Mint a database object's identity. The SCHEMA is the module.
    ///
    /// A `Result`, not an `Option`: the only failure is `fqn::define` refusing
    /// a segment, and that is a typed error a caller can report. Throwing the
    /// reason away would leave a walk that cannot say why it declined to name
    /// something (R2).
    fn identity(&self, schema: &str, name: &str) -> Result<Fqn, FqnError> {
        fqn::define(&Form::Item {
            lang: Language::Sql,
            package: self.package,
            module: schema,
            name,
            reach: Reach::Item,
        })
    }

    /// Read a qualified name at `i`: `a`, `a.b`, or `a.b.c`.
    ///
    /// Returns `(schema, object, next)`. For `db.schema.object` the DATABASE is
    /// dropped — this indexer scans one codebase, and carrying a database name
    /// into the module would put the same table under two identities depending
    /// on whether the script spelled it three-part.
    fn qualified(&self, toks: &[Tok<'a>], i: usize) -> Option<Qualified> {
        let mut parts: Vec<String> = Vec::new();
        let mut at = i;
        loop {
            let name = toks.get(at)?.name()?;
            parts.push(name.to_string());
            at += 1;
            // A dot CONTINUES the name only when a name follows it. `t.*` ends
            // the name at `t`.
            match (toks.get(at), toks.get(at + 1)) {
                (Some(Tok::Punct('.')), Some(next)) if next.name().is_some() => at += 1,
                _ => break,
            }
            if parts.len() >= 3 {
                break;
            }
        }
        let object = parts.pop()?;
        // BOTH FACTS AT ONCE, so neither is derived from the other: the module
        // string, and whether the source wrote one.
        let (module, qualified) = match parts.pop() {
            Some(schema) => (schema, true),
            None => (String::new(), false),
        };
        Some(Qualified { module, qualified, object, next: at })
    }

    /// Record a use site.
    fn refer(&mut self, from: &Fqn, kind: RefKind, name: &Qualified, line: u32) {
        let object = name.object.as_str();
        // A TEMP TABLE is local to the batch that made it and is not an object
        // any other file can name. `#t` lexes as a `Var`, so it never reaches
        // here — but a quoted `[#t]` would, and it is no more nameable.
        if object.starts_with('#') {
            return;
        }
        // PLACED ONLY when the source qualified it. An unqualified name
        // depends on the connection's default schema, which is a per-user
        // setting no file states — so it goes up unplaced and the ladder
        // answers, rather than being filed under `dbo` on an assumption.
        let target = match self.identity(&name.module, object) {
            Ok(fqn) if name.qualified => Resolution::Resolved { fqn, via: Rung::DeclaredHere },
            _ => Resolution::Unresolved {
                reason: Reason::Unplaced,
                evidence: Evidence {
                    name: object.to_string(),
                    node_kind: "object_reference".to_string(),
                    reach: Reach::Item,
                    saw: Vec::new(),
                },
            },
        };
        self.references.push(Reference { from: from.clone(), kind, at: at(line), target });
    }

    /// Declare an object, and return the identity references should hang off.
    fn declare(
        &mut self,
        name: &Qualified,
        kind: SymbolKind,
        label: &str,
        line: u32,
    ) -> Result<Fqn, FqnError> {
        let object = name.object.as_str();
        // A declaration the source left unqualified lands in the EMPTY module,
        // which is what it said. Resolving it against a default schema is a
        // deploy-time fact this file does not carry.
        let fqn = self.identity(&name.module, object)?;
        // ONCE. `DROP PROCEDURE X … CREATE PROCEDURE X` in one file is the
        // T-SQL idiom for a redeploy, and some scripts do it twice.
        if self.declared.insert(fqn.as_str().to_string()) {
            self.symbols.push(Symbol {
                fqn: fqn.clone(),
                kind,
                name: object.to_string(),
                span: at(line),
                // SQL states no visibility on the object — access is granted
                // separately with `GRANT`, which is a different statement and a
                // different fact.
                visibility: Visibility::Public,
                docstring: None,
                declared_type: DeclaredType::Stated(label.to_string()),
                params: Vec::new(),
            });
            self.relations.push(Relation {
                kind: RelationKind::Contains,
                child: fqn.clone(),
                parent: Resolution::Resolved { fqn: self.file.clone(), via: Rung::DeclaredHere },
                at: at(line),
            });
        }
        Ok(fqn)
    }

    /// Scan one batch for statement heads.
    fn batch(&mut self, toks: &[Tok<'a>], first_line: u32) {
        // References belong to the most recent declaration in this batch, and
        // to the FILE before there is one — which is the whole of a migration
        // script that declares nothing.
        let mut from = self.file.clone();
        let line = first_line;
        let mut i = 0usize;
        while i < toks.len() {
            let t = &toks[i];

            // `CREATE [OR ALTER] <object> <name>` — a DECLARATION.
            if t.is("create") {
                let mut head = i + 1;
                if toks.get(head).is_some_and(|x| x.is("or"))
                    && toks.get(head + 1).is_some_and(|x| x.is("alter"))
                {
                    head += 2;
                }
                // `CREATE UNIQUE NONCLUSTERED INDEX` — modifiers before the
                // object word. Skipped rather than matched, because the list is
                // open-ended and the object word is what decides.
                while toks.get(head).is_some_and(|x| {
                    x.is("unique")
                        || x.is("clustered")
                        || x.is("nonclustered")
                        || x.is("columnstore")
                }) {
                    head += 1;
                }
                if let Some(word) = toks.get(head)
                    && let Some(obj) = object_kind(word)
                    && let Some(name) = self.qualified(toks, head + 1)
                    && let Ok(fqn) = self.declare(&name, obj.kind, obj.label, line)
                {
                    from = fqn;
                    i = name.next;
                    continue;
                }
            }

            // `ALTER <object>` DECLARES when the statement carries the whole
            // definition and REFERS when it is an edit — see `alter_defines`.
            // `DROP <object>` always refers: it names something defined
            // elsewhere.
            if t.is("alter") || t.is("drop") {
                let dropping = t.is("drop");
                let mut head = i + 1;
                if let Some(word) = toks.get(head)
                    && let Some(obj) = object_kind(word)
                {
                    head += 1;
                    // `DROP TABLE IF EXISTS x`
                    if toks.get(head).is_some_and(|x| x.is("if")) {
                        head += 1;
                        if toks.get(head).is_some_and(|x| x.is("exists")) {
                            head += 1;
                        }
                    }
                    if let Some(name) = self.qualified(toks, head) {
                        if !dropping && obj.alter_defines {
                            if let Ok(fqn) = self.declare(&name, obj.kind, obj.label, line) {
                                from = fqn;
                            }
                        } else {
                            self.refer(&from.clone(), RefKind::Writes, &name, line);
                        }
                        i = name.next;
                        continue;
                    }
                }
            }

            // The reference heads. Each is a keyword whose next token starts a
            // qualified name.
            let reference = if t.is("from") || t.is("join") {
                Some(RefKind::Reads)
            } else if t.is("into") || t.is("update") || t.is("merge") {
                Some(RefKind::Writes)
            } else if t.is("exec") || t.is("execute") {
                Some(RefKind::Calls)
            } else if t.is("references") {
                // A FOREIGN KEY names the table it points at. `TypeUse` is the
                // kind for "names the target structurally", which is what a
                // key constraint does.
                Some(RefKind::TypeUse)
            } else {
                None
            };
            if let Some(kind) = reference {
                // `DELETE FROM x` and `INSERT INTO x` are reached through their
                // own keyword, so `FROM`/`INTO` alone carries them.
                //
                // A SUBQUERY opens with `FROM (`, which names no object.
                if let Some(name) = self.qualified(toks, i + 1) {
                    self.refer(&from.clone(), kind, &name, line);
                    i = name.next;
                    continue;
                }
            }

            // **A QUALIFIED CALL IN AN EXPRESSION.** `SELECT dbo.fnIssues(@x)`
            // invokes a user-defined function, and T-SQL REQUIRES a scalar UDF
            // to be schema-qualified. A built-in never is — `GETDATE()`,
            // `ISNULL()`, `LEN()` — so the qualification is the whole
            // distinction, and it is read off the grammar rather than out of a
            // table of known built-in names.
            //
            // This is dbd's soft/hard split, which it has to defer to a
            // resolver because Postgres does not require the qualification.
            // T-SQL does, so the answer is available here.
            if t.name().is_some()
                && matches!(toks.get(i + 1), Some(Tok::Punct('.')))
                && let Some(name) = self.qualified(toks, i)
                && name.qualified
                && matches!(toks.get(name.next), Some(Tok::Punct('(')))
            {
                let next = name.next;
                self.refer(&from.clone(), RefKind::Calls, &name, line);
                i = next;
                continue;
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_sql(text: &str) -> FileFacts {
        let source =
            Source { package: "pkg", module: "scripts/x.sql", path: "scripts/x.sql", text };
        read(&source, &TypeHomes::unknown()).expect("the fixture is T-SQL")
    }

    /// Prefix every fixture so `Dialect::detect` answers T-SQL — the reader
    /// refuses anything else, deliberately.
    fn tsql(body: &str) -> String {
        format!("SET ANSI_NULLS ON\nGO\n{body}")
    }

    fn declared(f: &FileFacts) -> Vec<&str> {
        f.symbols.iter().filter(|s| s.kind != SymbolKind::Module).map(|s| s.fqn.as_str()).collect()
    }

    fn refs(f: &FileFacts, kind: RefKind) -> Vec<String> {
        f.references
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                Resolution::Unresolved { evidence, .. } => format!("?{}", evidence.name),
            })
            .collect()
    }

    /// **CREATE DECLARES; ALTER AND DROP REFER.**
    ///
    /// The rule that makes a migration tree work. `ALTER TABLE` outnumbers
    /// `CREATE TABLE` 159 to 101 in this corpus, so reading `ALTER` as a
    /// declaration would make the commonest statement mint a duplicate identity
    /// for a table it does not define.
    ///
    /// MUTATION: treat `ALTER` as a declaration — every change script declares
    /// the table it edits and A7 becomes a count of migrations.
    #[test]
    fn create_declares_and_alter_refers_to_what_it_changes() {
        let created = read_sql(&tsql("CREATE TABLE [dbo].[Issues] (Id int)"));
        assert_eq!(declared(&created), ["sql·pkg·dbo·Issues·item"]);

        let altered = read_sql(&tsql("ALTER TABLE [dbo].[Issues] ADD [Name] nvarchar(50)"));
        assert!(declared(&altered).is_empty(), "a change script declares nothing");
        assert_eq!(refs(&altered, RefKind::Writes), ["sql·pkg·dbo·Issues·item"]);

        let dropped = read_sql(&tsql("DROP TABLE IF EXISTS [dbo].[Issues]"));
        assert!(declared(&dropped).is_empty());
        assert_eq!(refs(&dropped, RefKind::Writes), ["sql·pkg·dbo·Issues·item"]);

        // `CREATE OR ALTER` DOES declare: it defines the object either way.
        let both = read_sql(&tsql("CREATE OR ALTER PROCEDURE [dbo].[sp_X] AS SELECT 1"));
        assert_eq!(declared(&both), ["sql·pkg·dbo·sp_X·item"]);
    }

    /// **`ALTER` MEANS TWO DIFFERENT THINGS, AND THE OBJECT KIND SAYS WHICH.**
    ///
    /// `ALTER PROCEDURE X AS <body>` carries the COMPLETE new definition —
    /// T-SQL's syntax requires it — so the object is defined right here. The
    /// same is true of `ALTER VIEW`, `ALTER FUNCTION` and `ALTER TRIGGER`.
    ///
    /// `ALTER TABLE X ADD COLUMN y` carries no definition at all. It is an
    /// edit to a table defined elsewhere.
    ///
    /// Found by measuring: `sp_FetchDisclosureReports.sql` is a 982-byte file
    /// whose entire content is one `ALTER PROCEDURE` with its body, and a rule
    /// that read every `ALTER` as a reference declared nothing for it. Ethico
    /// ships a release per folder that way.
    ///
    /// MUTATION: make every `ALTER` refer — a redeployed procedure stops being
    /// a node and no C# call can reach it. Make every `ALTER` declare — every
    /// `ADD COLUMN` migration mints a duplicate table.
    #[test]
    fn alter_declares_a_body_carrying_object_and_refers_to_a_table() {
        let proc = read_sql(&tsql(
            "ALTER PROCEDURE [dbo].[sp_Fetch]\n@userId INT\nAS\nBEGIN\nSELECT 1\nEND",
        ));
        assert_eq!(declared(&proc), ["sql·pkg·dbo·sp_Fetch·item"], "the body is right here");

        for body_carrying in [
            "VIEW [dbo].[v_X] AS SELECT 1",
            "FUNCTION [dbo].[fn_X]() RETURNS int AS BEGIN RETURN 1 END",
        ] {
            let f = read_sql(&tsql(&format!("ALTER {body_carrying}")));
            assert_eq!(declared(&f).len(), 1, "ALTER {body_carrying} defines it");
        }

        // A TABLE is the other case, and it is the commoner one.
        let table = read_sql(&tsql("ALTER TABLE [dbo].[Issues] ADD [Name] nvarchar(50)"));
        assert!(declared(&table).is_empty(), "an ADD COLUMN defines no table");
        assert_eq!(refs(&table, RefKind::Writes), ["sql·pkg·dbo·Issues·item"]);
    }

    /// **A STORED PROCEDURE IS DECLARABLE**, which is the whole reason this
    /// reader exists: `tree-sitter-sequel` has no `create_procedure` node
    /// (`grammar.js` says `// TODO: procedure`) and `sqlparser`'s MsSql dialect
    /// fails on the parenless parameter form.
    ///
    /// MUTATION: drop `procedure` from `object_kind` — the object a T-SQL
    /// codebase is mostly made of stops being a node, and no C# call can point
    /// at it.
    #[test]
    fn a_stored_procedure_declares_even_with_parenless_parameters() {
        let f = read_sql(&tsql(
            "CREATE PROCEDURE [dbo].[sp_GetIssues]\n\
             \x20   @CustomerId int,\n\
             \x20   @From datetime = NULL\n\
             AS\n\
             BEGIN\n\
             \x20   SELECT * FROM [dbo].[Issues]\n\
             END",
        ));
        assert_eq!(declared(&f), ["sql·pkg·dbo·sp_GetIssues·item"]);
        // And the body's table belongs to the PROCEDURE, not to the file.
        let read_ref =
            f.references.iter().find(|r| r.kind == RefKind::Reads).expect("reads Issues");
        assert_eq!(read_ref.from.as_str(), "sql·pkg·dbo·sp_GetIssues·item");
    }

    /// Every reference head, and what kind each carries.
    #[test]
    fn each_statement_head_carries_the_kind_it_means() {
        let f = read_sql(&tsql(
            "SELECT * FROM dbo.A JOIN dbo.B ON 1=1\n\
             INSERT INTO dbo.C SELECT 1\n\
             UPDATE dbo.D SET x = 1\n\
             DELETE FROM dbo.E\n\
             EXEC dbo.sp_F",
        ));
        assert_eq!(
            refs(&f, RefKind::Reads),
            ["sql·pkg·dbo·A·item", "sql·pkg·dbo·B·item", "sql·pkg·dbo·E·item"]
        );
        assert_eq!(refs(&f, RefKind::Writes), ["sql·pkg·dbo·C·item", "sql·pkg·dbo·D·item"]);
        assert_eq!(refs(&f, RefKind::Calls), ["sql·pkg·dbo·sp_F·item"]);
    }

    /// **A QUALIFIED CALL IS A HARD EDGE; A BARE ONE IS A BUILT-IN.**
    ///
    /// dbd calls its function references SOFT because a view body is full of
    /// `now()` and `coalesce()` that look exactly like a call to a managed
    /// function, and only a resolver can tell them apart. T-SQL removes the
    /// ambiguity at the grammar level: a scalar user-defined function MUST be
    /// schema-qualified — `SELECT dbo.fnIssues(@x)` — while a built-in never
    /// is. So the qualification IS the soft/hard signal, and no table of known
    /// built-in names is needed to read it.
    ///
    /// MUTATION: emit a call for a bare `NAME(` too — `GETDATE()`, `ISNULL()`
    /// and `LEN()` become thousands of dangling edges that a reader cannot
    /// tell from a genuinely missing function.
    #[test]
    fn a_schema_qualified_call_is_an_edge_and_a_bare_one_is_a_builtin() {
        let f = read_sql(&tsql(
            "CREATE VIEW [dbo].[v_X] AS\n\
             SELECT [dbo].[fnIssues](Id) AS a, GETDATE() AS b, ISNULL(x, 0) AS c\n\
             FROM [dbo].[Issues]",
        ));
        assert_eq!(
            refs(&f, RefKind::Calls),
            ["sql·pkg·dbo·fnIssues·item"],
            "the qualified UDF is the only call; the built-ins are not edges"
        );
        // And the view still reads its table.
        assert_eq!(refs(&f, RefKind::Reads), ["sql·pkg·dbo·Issues·item"]);
    }

    /// A qualified call belongs to the routine it sits in.
    #[test]
    fn a_call_inside_a_procedure_belongs_to_that_procedure() {
        let f = read_sql(&tsql("CREATE PROCEDURE [dbo].[sp_A] AS SELECT [dbo].[fnB](1)"));
        let call = f.references.iter().find(|r| r.kind == RefKind::Calls).expect("calls fnB");
        assert_eq!(call.from.as_str(), "sql·pkg·dbo·sp_A·item");
    }

    /// **AN UNQUALIFIED NAME IS NOT FILED UNDER `dbo`.**
    ///
    /// The default schema is a per-user setting no file states. Assuming one
    /// would place a reference on an object it may not name.
    ///
    /// MUTATION: default the schema to `dbo` — references start resolving onto
    /// objects the source never qualified, which R4 ranks below a miss.
    #[test]
    fn an_unqualified_name_goes_to_the_ladder_rather_than_assuming_a_schema() {
        let f = read_sql(&tsql("SELECT * FROM Issues"));
        assert_eq!(refs(&f, RefKind::Reads), ["?Issues"], "unplaced, carrying the bare name");
    }

    /// A three-part name drops the DATABASE.
    ///
    /// MUTATION: keep it as the module — `mydb.dbo.Issues` and `dbo.Issues`
    /// become two identities for one table, depending only on how a script
    /// spelled it.
    #[test]
    fn a_three_part_name_is_the_same_object_as_a_two_part_one() {
        let f = read_sql(&tsql("SELECT * FROM mydb.dbo.Issues JOIN dbo.Issues ON 1=1"));
        assert_eq!(
            refs(&f, RefKind::Reads),
            ["sql·pkg·dbo·Issues·item", "sql·pkg·dbo·Issues·item"]
        );
    }

    /// A temp table is local to its batch and is nobody's object.
    #[test]
    fn a_temp_table_is_not_a_reference() {
        let f = read_sql(&tsql("SELECT * INTO #tmp FROM dbo.Real\nSELECT * FROM #tmp"));
        assert_eq!(refs(&f, RefKind::Reads), ["sql·pkg·dbo·Real·item"]);
        assert!(refs(&f, RefKind::Writes).is_empty(), "#tmp is not an object");
    }

    /// One object, declared once, however many times the script writes it.
    #[test]
    fn a_drop_then_create_redeploy_declares_the_object_once() {
        let f = read_sql(&tsql(
            "DROP PROCEDURE IF EXISTS [dbo].[sp_X]\n\
             GO\n\
             CREATE PROCEDURE [dbo].[sp_X] AS SELECT 1\n\
             GO\n\
             CREATE PROCEDURE [dbo].[sp_X] AS SELECT 2",
        ));
        assert_eq!(declared(&f), ["sql·pkg·dbo·sp_X·item"]);
    }

    /// **THE READER REFUSES A DIALECT IT IS NOT.**
    ///
    /// MUTATION: drop the guard — a Postgres `$$`-quoted body is read as
    /// statements and mints declarations out of a grammar this reader does not
    /// speak.
    #[test]
    fn a_file_that_is_not_t_sql_is_refused_rather_than_guessed_at() {
        let pg = Source {
            package: "pkg",
            module: "m",
            path: "m.sql",
            text: "CREATE FUNCTION f() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN END $$;",
        };
        assert!(read(&pg, &TypeHomes::unknown()).is_err(), "postgres is not this reader's");

        let bare =
            Source { package: "pkg", module: "m", path: "m.sql", text: "CREATE TABLE t (id int);" };
        assert!(read(&bare, &TypeHomes::unknown()).is_err(), "no dialect stated is not T-SQL");
    }

    /// A commented-out statement declares nothing — the lexer already drops it,
    /// and this pins that the reader depends on that.
    #[test]
    fn a_commented_out_declaration_is_not_one() {
        let f = read_sql(&tsql(
            "-- CREATE TABLE [dbo].[Ghost] (x int)\nCREATE TABLE [dbo].[Real] (x int)",
        ));
        assert_eq!(declared(&f), ["sql·pkg·dbo·Real·item"]);
    }

    /// A FOREIGN KEY names the table it points at.
    #[test]
    fn a_foreign_key_references_the_table_it_points_at() {
        let f = read_sql(&tsql(
            "CREATE TABLE [dbo].[Child] (ParentId int REFERENCES [dbo].[Parent](Id))",
        ));
        assert_eq!(declared(&f), ["sql·pkg·dbo·Child·item"]);
        assert_eq!(refs(&f, RefKind::TypeUse), ["sql·pkg·dbo·Parent·item"]);
    }

    /// Malformed input neither panics nor invents.
    #[test]
    fn a_truncated_statement_neither_panics_nor_declares() {
        for body in ["CREATE PROCEDURE", "CREATE", "ALTER TABLE", "SELECT * FROM", "CREATE TABLE ["]
        {
            let f = read_sql(&tsql(body));
            assert!(declared(&f).is_empty() || !declared(&f)[0].is_empty());
        }
    }
}
