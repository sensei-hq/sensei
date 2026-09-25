//! The PostgreSQL half of SQL — dbd reads it, this maps what it read.
//!
//! # Why this calls out rather than parsing
//!
//! dbd already owns SQL reference extraction and its model is RICHER than a
//! call graph. `Entity` carries `reads` and `writes` SEPARATELY, which no other
//! language's walk here produces: a Rust `Calls` edge says a function was
//! invoked, while dbd says whether a routine SELECTed from a table or wrote to
//! it. Re-implementing that would be a second, worse copy of a thing that
//! exists.
//!
//! `parse_sql(&str) -> ParsedFile` is the seam, and it is PATH-FREE. That
//! matters: `parse_entity` derives type, schema and name from
//! `ddl/<type>/<schema>/<name>.ddl`, and outside that layout it does not fail —
//! it falls back to `EntityType::Table` and names the entity after a directory,
//! so a stored procedure reads as a table. Measured before dbd exposed this:
//! 325 of 325 SQL Server files and 394 of 394 others "parsed OK" that way,
//! every one misclassified — and the schema came out as the first segment of
//! the absolute path, which on a checkout under a home directory is the word
//! `Users`.
//!
//! # SOFT AND HARD, which is dbd's distinction and is kept
//!
//! A `Reference` whose `ref_type` is `function` is SOFT: a view body is full of
//! `now()`, `sum()` and `coalesce()` that look exactly like calls to a managed
//! function, and Postgres gives no way to tell them apart at parse time. dbd
//! drops an unresolved one rather than warning. Here it becomes a reference
//! the LADDER may fail to place, which is the same statement in this
//! indexer's vocabulary — a miss with a reason, never a fabricated edge.
//!
//! T-SQL needs none of this because it REQUIRES a scalar UDF to be
//! schema-qualified; see `lang::sql::tsql`.
//!
//! # One identity scheme, both dialects
//!
//! A Postgres table and a T-SQL table take the SAME identity shape — language,
//! package, schema, object, reach — because `fqn::define` mints both from
//! `Form::Item` with the schema as the module. The dialect decides who READS a
//! file, never how the result is named: two naming schemes under one language
//! would mean a reference from one dialect could never meet a declaration from
//! the other.

use dbd_core::entity::{Entity, EntityType, REF_TYPE_FUNCTION};

use super::SqlAdapter;
use crate::indexer::facts::{
    DeclaredType, Evidence, FileFacts, Fqn, Language, Reason, RefKind, Reference, Relation,
    RelationKind, Resolution, Rung, Span, Symbol, SymbolKind, Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};
use crate::indexer::lang::{LanguageAdapter, ReadError, Source, TypeHomes};

/// What each dbd entity type is in the shared vocabulary.
///
/// A VIEW and a MATERIALIZED VIEW both take [`SymbolKind::Struct`] alongside a
/// table, the same call C's walk makes for a union: all three are RELATIONS,
/// and stored-versus-derived is a property of the object rather than a
/// different kind of declaration. The `declared_type` carries dbd's own word,
/// so the distinction is queryable without putting a SQL-only variant on every
/// language's symbols.
///
/// `None` means an entity this indexer does not model as a node. A SCHEMA is a
/// module segment rather than a thing declarations hang off; an EXTENSION is a
/// dependency, which `DbdManifestAdapter` already records from `design.yaml`;
/// `External` and `Import` are dbd's own bookkeeping.
fn symbol_kind(entity_type: EntityType) -> Option<(SymbolKind, &'static str)> {
    match entity_type {
        EntityType::Table => Some((SymbolKind::Struct, "table")),
        EntityType::View => Some((SymbolKind::Struct, "view")),
        EntityType::MaterializedView => Some((SymbolKind::Struct, "materialized_view")),
        EntityType::Function => Some((SymbolKind::Function, "function")),
        EntityType::Procedure => Some((SymbolKind::Function, "procedure")),
        // A trigger is executable and declared by name, so it is a Function the
        // same way a procedure is; `declared_type` keeps dbd's own word. New in
        // dbd 0.15.0 — the exhaustive match is what surfaced it.
        EntityType::Trigger => Some((SymbolKind::Function, "trigger")),
        EntityType::Enum => Some((SymbolKind::Enum, "enum")),
        EntityType::Sequence => Some((SymbolKind::Static, "sequence")),
        EntityType::Role => Some((SymbolKind::Const, "role")),
        EntityType::Schema | EntityType::Extension | EntityType::External | EntityType::Import => {
            None
        }
    }
}

/// Read one SQL file in a STATED dialect.
///
/// Dialect-agnostic since dbd 0.15.0: `parse_sql_as` answers for PostgreSQL,
/// T-SQL, MySQL and SQLite behind one entry point, and every one of them comes
/// back as the same [`dbd_core::parser::ParsedFile`]. So the mapping below —
/// entity to symbol, reference to edge — is written once and the dialect only
/// decides which grammar produced the input.
///
/// This is what let this crate's own T-SQL lexer and statement-head reader
/// (`lex.rs` + `tsql.rs`, 1,058 lines) be deleted rather than maintained beside
/// dbd's, which reached the same two rules — `ALTER PROCEDURE` carries a
/// definition where `ALTER TABLE` does not, and a qualified call is an edge
/// where a bare one is a built-in.
pub fn read(
    dialect: dbd_core::parser::Dialect,
    source: &Source<'_>,
    _types: &TypeHomes,
) -> Result<FileFacts, ReadError> {
    let parsed = dbd_core::parser::parse_sql_as(dialect, source.text)
        .map_err(|e| ReadError::NotParsedBecause(format!("{}: {e}", source.path)))?;

    // **`Ok` IS NOT SUCCESS.** `parse_sql` returns the file it managed to read
    // AND a list of file-level failures — SQL that Postgres itself rejects —
    // rather than an `Err`. Measured: `CREATE TABLE ( THIS IS NOT SQL` comes
    // back `Ok(entities: 0, errors: ["syntax error at or near \"(\""])`.
    //
    // Taking the entities anyway would make a file that FAILED TO PARSE
    // indistinguishable from one that legitimately declares nothing — a
    // migration that only INSERTs — and the second is common. So the
    // complaint is surfaced, with the text Postgres produced, which
    // `process_file` writes to `index_errors` for an operator to read.
    if !parsed.errors.is_empty() {
        return Err(ReadError::NotParsedBecause(format!(
            "{}: {}",
            source.path,
            parsed.errors.join("; ")
        )));
    }

    let file = SqlAdapter
        .file_fqn(source.package, source.module, source.path)
        .map_err(ReadError::NoFileIdentity)?;

    let mut walk = Walk {
        package: source.package,
        // dbd resolves an unqualified name against the file's first search
        // path, so the same default is used for a reference it left bare.
        default_schema: parsed.search_paths.first().cloned(),
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        file: file.clone(),
    };
    for entity in &parsed.entities {
        walk.entity(entity);
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
        // SQL HAS NO IMPORT. An object is named in full or reached through the
        // search path, and a `search_path` is a resolution setting rather than
        // a statement bringing one name into scope.
        imports: Vec::new(),
    })
}

fn stem_of(path: &str) -> &str {
    let f = path.rsplit('/').next().unwrap_or(path);
    f.rsplit_once('.').map_or(f, |(head, _)| head)
}

struct Walk<'a> {
    package: &'a str,
    default_schema: Option<String>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
    relations: Vec<Relation>,
    file: Fqn,
}

impl Walk<'_> {
    /// Split a name dbd wrote into `(schema, object)`.
    ///
    /// dbd qualifies what it can against the file's search path, so most
    /// arrive as `schema.name`. One that did not is carried unqualified and
    /// placed against the file's default schema only when the file STATED one
    /// — `search_paths` is `["public"]` by dbd's own default, which is a real
    /// Postgres rule rather than a guess.
    fn split(&self, written: &str) -> (Option<String>, String) {
        match written.rsplit_once('.') {
            Some((schema, object)) if !schema.is_empty() && !object.is_empty() => {
                (Some(schema.to_string()), object.to_string())
            }
            _ => (self.default_schema.clone(), written.to_string()),
        }
    }

    /// A `Result`, not an `Option`: the only failure is `fqn::define` refusing
    /// a segment, and that is a typed error rather than an absence (R2).
    fn identity(&self, schema: &str, name: &str) -> Result<Fqn, FqnError> {
        fqn::define(&Form::Item {
            lang: Language::Sql,
            package: self.package,
            module: schema,
            name,
            reach: Reach::Item,
        })
    }

    /// A reference the walk read but could not place. The ladder takes it.
    fn unplaced(&self, object: &str) -> Resolution {
        Resolution::Unresolved {
            reason: Reason::Unplaced,
            evidence: Evidence {
                name: object.to_string(),
                node_kind: "object_reference".to_string(),
                reach: Reach::Item,
                saw: Vec::new(),
            },
        }
    }

    fn refer(&mut self, from: &Fqn, kind: RefKind, written: &str) {
        let written = written.trim();
        if written.is_empty() {
            return;
        }
        let (schema, object) = self.split(written);
        let target = match schema.as_deref().map(|s| self.identity(s, &object)) {
            Some(Ok(fqn)) => Resolution::Resolved { fqn, via: Rung::DeclaredHere },
            // The name was qualified and `fqn::define` still refused a
            // segment. Unplaced with the object's name, same as an unqualified
            // one — the reference is real either way.
            Some(Err(_)) => self.unplaced(&object),
            // NO SCHEMA the file stated, so the object cannot be placed. The
            // ladder gets the bare name rather than this picking `public` on a
            // file that never said so.
            None => self.unplaced(&object),
        };
        self.references.push(Reference {
            from: from.clone(),
            kind,
            // dbd reports no span. A line pointing at the wrong place would be
            // worse than one pointing at the file, which is what this is.
            at: Span { start_line: 1, start_col: 1, end_line: 1, end_col: 1 },
            target,
        });
    }

    fn entity(&mut self, entity: &Entity) {
        let Some((kind, label)) = symbol_kind(entity.entity_type) else {
            return;
        };
        // `Entity::name` is already qualified when the type has a schema, so it
        // is split the same way a reference is rather than read off
        // `entity.schema` — the two can disagree for an unqualified
        // declaration and the NAME is what a use site writes.
        let (schema, object) = self.split(&entity.name);
        // BOTH FACTS, so neither is derived from the other: the module string,
        // and whether anything qualified the name. An unqualified declaration
        // lands in the EMPTY module, which is what the source said — not a
        // read that failed.
        let module = match &schema {
            Some(schema) => schema.as_str(),
            None => "",
        };
        let Ok(fqn) = self.identity(module, &object) else {
            return;
        };

        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind,
            name: object,
            span: Span { start_line: 1, start_col: 1, end_line: 1, end_col: 1 },
            // SQL states no visibility on an object — access is `GRANT`, a
            // separate statement and a separate fact.
            visibility: Visibility::Public,
            docstring: None,
            // dbd's own word for the type, so `view` and `materialized_view`
            // stay distinguishable without a SQL-only `SymbolKind`.
            declared_type: DeclaredType::Stated(label.to_string()),
            params: Vec::new(),
        });
        self.relations.push(Relation {
            kind: RelationKind::Contains,
            child: fqn.clone(),
            parent: Resolution::Resolved { fqn: self.file.clone(), via: Rung::DeclaredHere },
            at: Span { start_line: 1, start_col: 1, end_line: 1, end_col: 1 },
        });

        // **THE READ/WRITE SPLIT, which is the reason to call dbd at all.**
        // Every other language here can say only that one thing referenced
        // another.
        for table in &entity.reads {
            self.refer(&fqn.clone(), RefKind::Reads, table);
        }
        for table in &entity.writes {
            self.refer(&fqn.clone(), RefKind::Writes, table);
        }
        for reference in &entity.references {
            // SOFT vs HARD. A `function` reference may be a built-in — dbd
            // cannot tell, because Postgres does not require qualification —
            // so it is emitted as a CALL and the ladder decides. Everything
            // else names a relation structurally: a foreign key, a view's
            // dependency, a trigger's table.
            let kind = match reference.ref_type.as_deref() {
                Some(REF_TYPE_FUNCTION) => RefKind::Calls,
                _ => RefKind::TypeUse,
            };
            self.refer(&fqn.clone(), kind, &reference.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbd_core::parser::Dialect;

    /// Read a fixture as PostgreSQL — the dialect these tests are written in.
    /// The dialect is an ARGUMENT now rather than something detected, so a
    /// fixture cannot silently be read by a grammar it was not written for.
    fn read_sql(text: &str) -> FileFacts {
        let source =
            Source { package: "pkg", module: "database/x.ddl", path: "database/x.ddl", text };
        read(Dialect::PostgreSql, &source, &TypeHomes::unknown()).expect("the fixture parses")
    }

    fn declared(f: &FileFacts) -> Vec<(&str, &str)> {
        f.symbols
            .iter()
            .filter(|s| s.kind != SymbolKind::Module)
            .map(|s| {
                let DeclaredType::Stated(label) = &s.declared_type else { panic!("labelled") };
                (s.fqn.as_str(), label.as_str())
            })
            .collect()
    }

    fn refs(f: &FileFacts, kind: RefKind) -> Vec<String> {
        let mut out: Vec<String> = f
            .references
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                Resolution::Unresolved { evidence, .. } => format!("?{}", evidence.name),
            })
            .collect();
        out.sort();
        out
    }

    /// dbd reads the declaration, and the identity is THIS indexer's.
    ///
    /// MUTATION: name the symbol from `entity.schema` + a bare name — an
    /// unqualified declaration and the reference that names it stop agreeing.
    #[test]
    fn a_table_is_declared_under_its_schema() {
        let f = read_sql("CREATE TABLE app.issues (id int primary key);");
        assert_eq!(declared(&f), [("sql·pkg·app·issues·item", "table")]);
    }

    /// **THE READ/WRITE SPLIT — the reason this calls dbd.**
    ///
    /// No other language's walk here distinguishes them: a Rust `Calls` edge
    /// says a function was invoked, while this says a routine SELECTed from
    /// one table and wrote to another.
    ///
    /// MUTATION: map both `reads` and `writes` to one kind — the distinction
    /// that cannot be got from any other parser is thrown away at the seam.
    #[test]
    fn a_routine_reads_and_writes_are_separate_edges() {
        let f = read_sql(
            "CREATE FUNCTION app.sync() RETURNS void LANGUAGE plpgsql AS $$\n\
             BEGIN\n\
               INSERT INTO app.audit SELECT * FROM app.issues;\n\
             END $$;",
        );
        assert_eq!(declared(&f), [("sql·pkg·app·sync·item", "function")]);
        assert_eq!(refs(&f, RefKind::Reads), ["sql·pkg·app·issues·item"]);
        assert_eq!(refs(&f, RefKind::Writes), ["sql·pkg·app·audit·item"]);
    }

    /// A view's dependency and a foreign key are STRUCTURAL references.
    #[test]
    fn a_view_names_the_tables_it_is_built_from() {
        let f =
            read_sql("CREATE VIEW app.open_issues AS SELECT * FROM app.issues WHERE done = false;");
        assert_eq!(declared(&f), [("sql·pkg·app·open_issues·item", "view")]);
        let named = [refs(&f, RefKind::TypeUse), refs(&f, RefKind::Reads)].concat();
        assert!(
            named.contains(&"sql·pkg·app·issues·item".to_string()),
            "the view names its source table: {named:?}"
        );
    }

    /// **ONE IDENTITY SCHEME ACROSS BOTH DIALECTS.**
    ///
    /// A Postgres table and a T-SQL table are the same string shape. Two
    /// schemes under one language would mean a reference written in one
    /// dialect could never meet a declaration read by the other.
    ///
    /// MUTATION: mint Postgres objects with the schema in the NAME rather than
    /// the module — the two halves of `Language::Sql` stop being one graph.
    #[test]
    fn a_postgres_object_is_named_the_way_a_t_sql_one_is() {
        let pg = read_sql("CREATE TABLE dbo.issues (id int);");
        // The same string `tsql::tests` asserts for `CREATE TABLE [dbo].[Issues]`,
        // modulo the object's own spelling.
        assert_eq!(declared(&pg), [("sql·pkg·dbo·issues·item", "table")]);
    }

    /// An entity type this indexer does not model as a node declares nothing.
    ///
    /// MUTATION: give `Schema` a kind — every `CREATE SCHEMA` mints a node
    /// that is really a module segment, and declarations hang off two parents.
    #[test]
    fn a_schema_is_a_module_segment_rather_than_a_node() {
        let f = read_sql("CREATE SCHEMA app;");
        assert!(declared(&f).is_empty(), "a schema is not a declaration here");
        // The file module is still emitted, so the file is in the graph.
        assert_eq!(f.symbols.len(), 1);
        assert_eq!(f.symbols[0].kind, SymbolKind::Module);
    }

    /// **SQL POSTGRES REJECTS IS AN ERROR, AND `Ok` IS NOT SUCCESS.**
    ///
    /// `parse_sql` hands back the entities it managed plus a list of
    /// file-level failures, rather than an `Err` — so a reader that checks
    /// only the `Result` treats a file that failed to parse exactly like a
    /// migration that legitimately declares nothing.
    ///
    /// MUTATION: drop the `parsed.errors` check — this test's broken SQL
    /// returns empty facts and reports success, and the reason Postgres gave
    /// never reaches `index_errors`.
    #[test]
    fn sql_postgres_rejects_is_an_error_rather_than_no_facts() {
        let broken = Source {
            package: "pkg",
            module: "m.ddl",
            path: "m.ddl",
            text: "CREATE TABLE ( THIS IS NOT SQL",
        };
        let Err(ReadError::NotParsedBecause(why)) =
            read(Dialect::PostgreSql, &broken, &TypeHomes::unknown())
        else {
            panic!("broken SQL must fail WITH a reason")
        };
        assert!(why.contains("syntax error"), "the reason is Postgres's own: {why}");

        // THE DIALECT ARGUMENT IS LOAD-BEARING. The same T-SQL text is refused
        // when read as PostgreSQL — `[dbo]` is not Postgres — and read fine
        // when read as T-SQL. Before dbd 0.15.0 this file could only be refused,
        // because this module had one Postgres reader and its own T-SQL walk;
        // now the grammar is chosen and being wrong about it still fails loudly
        // rather than yielding an empty file.
        let tsql = Source {
            package: "pkg",
            module: "m.sql",
            path: "m.sql",
            text: "CREATE TABLE [dbo].[Issues] ([Id] int)",
        };
        assert!(
            read(Dialect::PostgreSql, &tsql, &TypeHomes::unknown()).is_err(),
            "T-SQL read as Postgres is refused, not read as declaring nothing"
        );
        let as_tsql = read(Dialect::TSql, &tsql, &TypeHomes::unknown())
            .expect("the same text reads when the right grammar is named");
        assert!(
            as_tsql.symbols.iter().any(|s| s.name.eq_ignore_ascii_case("Issues")),
            "{:?}",
            as_tsql.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    /// A migration that only inserts declares nothing, and that is not an
    /// error — the same rule the T-SQL reader follows.
    #[test]
    fn a_pure_data_script_declares_nothing_without_failing() {
        let f = read_sql("INSERT INTO app.issues (id) VALUES (1);");
        assert!(declared(&f).is_empty());
    }
}
