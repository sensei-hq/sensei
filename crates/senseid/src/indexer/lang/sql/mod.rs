//! SQL — which dialect, and what a database object's identity is.
//!
//! **The reading is dbd's; the IDENTITY is this indexer's.** `parse_sql_as`
//! answers for the two dialects dbd has a grammar for — PostgreSQL and T-SQL —
//! and [`facts`] maps what it returns onto symbols, references and relations.
//!
//! MySQL and SQLite are still refused. dbd's `ParserChoice::for_dialect_typed`
//! sends SQLite to `Verbatim` and **MySQL to `PgQuery`**, so passing them
//! through would parse MySQL as PostgreSQL and report `syntax error` for a file
//! that is not broken. "We lack your grammar" and "your file is malformed" are
//! different answers and only one of them is the author's problem.
//!
//! This module used to carry its own T-SQL lexer and statement-head reader
//! (`lex.rs` + `tsql.rs`, 1,058 lines), written because nothing off the shelf
//! could declare a stored procedure — `tree-sitter-sequel`'s grammar says
//! `// TODO: procedure`, and `sqlparser`'s `MsSqlDialect` fails on the
//! parenless `CREATE PROCEDURE @p int AS` form that is 28% of one corpus. dbd
//! 0.15.0 ships that reader, having reached the same two rules below, so
//! keeping a second copy would only mean two things to re-measure. They are
//! deleted.
//!
//! The rules survive because they are what makes the identities correct, and
//! they are recorded here as the reason this adapter trusts that parser:
//!
//! # THE DIALECT IS STATED, OR IT IS DETECTED
//!
//! The same split every language here has turned on. Java writes `package`;
//! C says `static` and means the file. SQL says which dialect it is in two
//! ways, and only one of them is authoritative:
//!
//! - **STATED.** A `design.yaml` beside the tree carries `source.dialect:
//!   postgresql`. That is a manifest declaring it, and a manifest outranks
//!   anything read out of a file. See `DbdManifestAdapter`.
//! - **DETECTED.** Everything else — 4,832 SQL files in one repository alone
//!   with no manifest of any kind — has to be read. [`Dialect::detect`] scores
//!   markers that exist in exactly one dialect.
//!
//! Detection FAILS CLOSED. A file with no marker is [`Dialect::Unstated`], not
//! a guess: `CREATE TABLE t (id int)` is valid in every dialect and says
//! nothing about which one it is in.
//!
//! # A CHANGE SCRIPT IS NOT A DECLARATION, AND THE KEYWORDS SAY SO
//!
//! This is the question that blocked SQL for a long time. A `.rs` file declares
//! `fn foo`. What does `Alter_Table_Email_Template.sql` declare? Nothing — it
//! EDITS a table that exists elsewhere. Two such scripts are not two
//! declarations of one table.
//!
//! The source already answers it:
//!
//! - `CREATE <object> <name>` DECLARES it.
//! - `ALTER <object> <name>` and `DROP <object> <name>` REFER to it.
//!
//! That is what the words mean, and it makes a migration tree fall out
//! correctly: the table is declared wherever `CREATE TABLE` is, and every
//! script that alters it points there. MEASURED over this corpus, `ALTER TABLE`
//! outnumbers `CREATE TABLE` 159 to 101 — so reading `ALTER` as a declaration
//! would have made the commonest statement in the corpus mint a duplicate
//! identity for a table it does not define.
//!
//! `CREATE OR ALTER <object>` is a declaration: it defines the object whether
//! or not one was there.
//!
//! # A QUALIFIED CALL IS A HARD EDGE; A BARE ONE IS A BUILT-IN
//!
//! dbd calls its function references SOFT: a view body is full of `now()`,
//! `sum()` and `coalesce()` that look exactly like a call to a project-managed
//! function, so one that does not resolve is dropped rather than warned about
//! (`dbd-core`'s `REF_TYPE_FUNCTION`). Postgres gives it no way to tell them
//! apart at parse time.
//!
//! T-SQL does. A scalar user-defined function MUST be schema-qualified —
//! `SELECT dbo.fnIssues(@x)` — and a built-in never is. So the qualification IS
//! the soft/hard signal, read off the grammar rather than out of a table of
//! known built-in names, and the reader emits a call only for the qualified
//! form. Worth 4,628 function-to-function edges over Ethico, every one of them
//! placed, and zero dangling built-ins.
//!
//! # The schema is the module
//!
//! `[dbo].[sp_GetIssues]` lives in schema `dbo`, and a schema is a namespace
//! that objects hang in — the same shape as a Java package, so it takes the
//! same segment. ~70% of the declarations in this corpus are qualified; the
//! rest are bare and take an empty module, which is what the source said.

use std::sync::LazyLock;

use super::{LanguageAdapter, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach, Segment};
use crate::indexer::resolve::Grammar;

mod facts;

/// WHICH SQL THIS FILE IS WRITTEN IN — dbd's, not a second copy.
///
/// This module carried its own `Dialect` with its own scored `detect`. dbd
/// 0.15.0 has the same enum and the same detection, including the lesson that
/// cost the most to learn: a BACKTICK is not a MySQL marker, because it is also
/// what everybody writes around a word in a comment. Measured here before that
/// was known — 2,132 backticks across 178 files of sensei's own Postgres DDL
/// turned a third of it into MySQL.
///
/// Two copies of a scoring table drift, and the one that drifts is the one
/// nobody re-measures. So the dialect is dbd's answer now, and
/// `Dialect::from_label` still reads what a `design.yaml` states.
pub use dbd_core::parser::Dialect;

/// The names every T-SQL database has with nothing written.
///
/// The package is `mssql` — the server, which is the real thing that ships
/// them. `sys` objects are reached bare or as `sys.x`, and a codebase queries
/// them constantly.
const PRELUDE: &[(&str, &str, &str)] = &[
    ("sysobjects", "mssql", "sys.sysobjects"),
    ("syscolumns", "mssql", "sys.syscolumns"),
    ("sysindexes", "mssql", "sys.sysindexes"),
    ("sys", "mssql", "sys"),
    ("information_schema", "mssql", "information_schema"),
    ("getdate", "mssql", "getdate"),
    ("newid", "mssql", "newid"),
    ("isnull", "mssql", "isnull"),
    ("scope_identity", "mssql", "scope_identity"),
    ("sp_executesql", "mssql", "sp_executesql"),
    ("sp_rename", "mssql", "sp_rename"),
    ("sp_addextendedproperty", "mssql", "sp_addextendedproperty"),
];

/// EMPTY. Every other language's plumbing list names members that say nothing
/// about a receiver's type. SQL has no receivers and no member calls — an
/// object is named outright — so there is no shape to exclude.
const PLUMBING: &[&str] = &[];

/// SQL has no spelling that tells a schema from an object.
///
/// `dbo.Issues` and `Sales.Orders` put the schema first; nothing in the casing
/// says which half is which, and T-SQL is case-insensitive throughout. So this
/// is the predicate that never fires, which [`Grammar::names_a_type`] names as
/// the honest answer where a language does not state it — the walk reads the
/// POSITION instead, which it can, because a qualified name has exactly two
/// parts and the schema is the first.
fn names_a_type(_segment: &str) -> bool {
    false
}

pub static GRAMMAR: LazyLock<Grammar> = LazyLock::new(|| Grammar {
    language: Language::Sql,
    path_separator: ".",
    module_separator: ".",
    // No `crate::`, no `./`. A SQL name is absolute within its database.
    roots: &[],
    module_segment: crate::indexer::lang::common::already_a_module_segment,
    relative_depth_prefix: None,
    names_the_binding: None,
    wildcard: None,
    // FALSE. SQL has no import at all — an object is named in full or reached
    // through the default schema — so a multi-segment path never arrives
    // carrying a package, and the rung this enables could only misfire.
    paths_name_packages: false,
    relative_to_directory: false,
    names_a_type,
    prelude: PRELUDE,
    plumbing: PLUMBING,
});

/// The object a qualified SQL name refers to.
///
/// `[dbo].[Issues]` names `Issues`; the schema in front of it is WHERE it
/// lives, which the walk records as the module rather than as part of the name.
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
    let bare = raw.trim().trim_matches(['[', ']', '"', '`'].as_ref()).trim();
    let last = bare.rsplit('.').next().unwrap_or(bare).trim();
    let last = last.trim_matches(['[', ']', '"', '`'].as_ref()).trim();
    if last.is_empty() {
        return Err(FqnError::EmptySegment { segment: Segment::Type });
    }
    Ok(last.to_string())
}

/// SQL, from `.sql` and `.ddl`.
pub struct SqlAdapter;

impl LanguageAdapter for SqlAdapter {
    fn language(&self) -> Language {
        Language::Sql
    }

    fn name(&self) -> &'static str {
        "sql"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".sql", ".ddl"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    /// THE DIALECT CHOOSES THE GRAMMAR, never the identity.
    ///
    /// There is one adapter rather than four because `adapter_for_ext`
    /// dispatches on the EXTENSION: `.sql` and `.ddl` cannot be claimed by a
    /// `tsql` adapter and a `postgres` adapter both. So the split lives here,
    /// below the registry — and since dbd 0.15.0 it is no longer a split at
    /// all. `parse_sql_as` answers for PostgreSQL, T-SQL, MySQL and SQLite
    /// behind one entry point, and every dialect comes back as the same
    /// `ParsedFile`, so one mapping names them all.
    ///
    /// Every dialect therefore mints the SAME identity shape — language,
    /// package, schema, object, reach — and a reference written in one can meet
    /// a declaration read by another.
    ///
    /// **TWO GRAMMARS EXIST, AND THE OTHER DIALECTS ARE STILL REFUSED.** dbd
    /// 0.15.0 ships `pg` and `tsql`; `ParserChoice::for_dialect_typed` sends
    /// SQLite to `Verbatim` (which `parse_sql_with` errors on) and **MySQL to
    /// `PgQuery`**. So handing MySQL to `parse_sql_as` does not read it — it
    /// parses it as PostgreSQL and fails on the first backtick, reporting
    /// `syntax error` for a file that is not broken.
    ///
    /// That is worse than saying nothing, so the refusal is kept HERE, where
    /// the honest reason is available: no reader for this dialect. A parse
    /// error means the file is malformed; it must not also mean we lack its
    /// grammar.
    ///
    /// `Unstated` is refused for the other reason:
    /// `CREATE TABLE t (id int)` is valid in every dialect and says nothing
    /// about which it is in, so parsing it as any of them would read structure
    /// out of a grammar the file is not written in.
    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, super::ReadError> {
        match Dialect::detect(source.text) {
            readable @ (Dialect::PostgreSql | Dialect::TSql) => {
                facts::read(readable, source, types)
            }
            Dialect::Unstated => Err(super::ReadError::GrammarUnavailable(format!(
                "{}: the file states no dialect",
                source.path
            ))),
            other => Err(super::ReadError::GrammarUnavailable(format!(
                "{}: no reader for {other:?}",
                source.path
            ))),
        }
    }

    fn file_fqn(&self, package: &str, module: &str, path: &str) -> Result<Fqn, FqnError> {
        let stem = path.rsplit('/').next().unwrap_or(path);
        let stem = stem.rsplit_once('.').map_or(stem, |(head, _)| head);
        fqn::define(&Form::Item {
            lang: Language::Sql,
            package,
            module,
            name: stem,
            reach: Reach::Mod,
        })
    }

    /// The file's own path, minus its extension.
    ///
    /// NOT the schema. A file's module and an OBJECT's module are different
    /// questions: the object lives in the schema it names, which the walk reads
    /// per declaration, while the FILE lives where it sits on disk. Two scripts
    /// called `001_init.sql` in different directories are two files.
    fn module_path(&self, file: &str, package_root: &str) -> String {
        file.strip_prefix(package_root).unwrap_or(file).trim_start_matches('/').to_string()
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        type_segment(raw)
    }

    /// FALSE. A database object's identity is its schema and its name, both
    /// written in the source — moving the script that creates it does not move
    /// the table.
    fn rename_remints_identity(&self, _from: &str, _to: &str, _package_root: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The A7 ratchet for this corpus. A RATCHET, not a budget: it sits AT the
    /// measured value so a single new collision fails the gate.
    ///
    /// MEASURED over Ethico — 9 repositories, 2,419 SQL files, 14 of them
    /// unreadable (latin-1 and unknown-8bit with no BOM to name them, which
    /// `classifiers::decode_source` refuses rather than guessing at).
    ///
    /// ```text
    /// by dialect    TSql 2154, Unstated 180, PostgreSql 73, MySql 12
    /// read as T-SQL 2154
    ///   declaring an object 1458 (68%)
    ///     2,742 objects: 1,285 procedure, 613 table, 506 view,
    ///                    220 function, 113 trigger, 5 type
    ///   declaring nothing    695 CORRECTLY — a pure INSERT, UPDATE or
    ///                        `ALTER TABLE ADD` script defines no object
    ///                          1 MISSED
    ///   references         43,737 (28,626 schema-qualified and placed)
    /// COLLIDING            544
    ///   within one file        0
    ///   identical copies      21
    ///   same object, different release folders  523
    /// ```
    ///
    /// **ZERO intra-file collisions**, which is the bucket the reader alone
    /// controls.
    ///
    /// THE ONE MISS IS DYNAMIC SQL, and the reader is right about it.
    /// `LCAMActivityReport.sql` wraps its whole definition in
    /// `EXEC dbo.sp_executesql @statement = N'…'`, so the `ALTER PROCEDURE`
    /// is inside a STRING LITERAL. The lexer consumes literals, as it must —
    /// reading their contents as code would also mint declarations out of
    /// example SQL and commented-out blocks. 4 files in Ethico use the
    /// wrapper and 1 declares through it, so it is named here rather than
    /// special-cased.
    ///
    /// THE 544 ARE THE CORPUS, not the reader. Ethico ships a folder per
    /// release — `4.2.1/2. StoredProcedures/sp_NewMCRIssue.sql` beside
    /// `4.3.0.2/…`, and a directory literally named `DO NOT USE_4.1/`.
    /// `dbo.sp_NewMCRIssue` IS one procedure; the tree holds its definition at
    /// several versions, and one identity for it is the right answer.
    ///
    /// These numbers ROSE when `decode_source` landed: 377 more files became
    /// readable (UTF-16LE from SSMS), worth +329 stored procedures and +7,560
    /// references. A rise in `within one file` is a defect in this reader; a
    /// rise in the other two is a release folder somebody added.
    const A7_BOUND: usize = 544;

    /// **COVERAGE for the PostgreSQL half, over real dbd schemas.**
    ///
    /// The T-SQL gate below asks whether a reader built on statement heads
    /// finds the declarations. This asks a different question, because the
    /// reading is dbd's: does the SEAM hold — does every entity dbd reports
    /// become a node with an identity, and does the read/write split survive?
    ///
    ///     SENSEI_CORPUS=/path/with/design.yaml cargo test -p senseid \
    ///       --bin senseid sql::tests::the_postgres_corpus_reads_through_dbd \
    ///       -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_postgres_corpus_reads_through_dbd() {
        use std::collections::{BTreeMap, BTreeSet};

        let Ok(root) = std::env::var("SENSEI_CORPUS") else {
            println!("SENSEI_CORPUS unset — nothing to read. See this test's docs.");
            return;
        };

        let (mut files, mut refused, mut declaring) = (0usize, 0usize, 0usize);
        let (mut reads, mut writes, mut calls, mut structural) = (0usize, 0usize, 0usize, 0usize);
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        let mut sites: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut why: BTreeMap<String, usize> = BTreeMap::new();

        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "ddl" && e != "sql") {
                continue;
            }
            let Ok(bytes) = std::fs::read(path) else { continue };
            let crate::classifiers::Decoded::Text(text) = crate::classifiers::decode_source(&bytes)
            else {
                continue;
            };
            if Dialect::detect(&text) != Dialect::PostgreSql {
                continue;
            }
            files += 1;
            let rel = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().to_string();
            let source = Source { package: "corpus", module: &rel, path: &rel, text: &text };
            let facts = match SqlAdapter.read(&source, &TypeHomes::unknown()) {
                Ok(f) => f,
                Err(e) => {
                    refused += 1;
                    // The FIRST LINE of the reason, grouped — a histogram of
                    // why files are refused is what says whether the seam is
                    // sound or whether dbd and this corpus disagree.
                    let reason = format!("{e:?}");
                    let head: String = reason.chars().take(70).collect();
                    *why.entry(head).or_default() += 1;
                    continue;
                }
            };
            let objects: Vec<_> = facts
                .symbols
                .iter()
                .filter(|s| s.kind != crate::indexer::facts::SymbolKind::Module)
                .collect();
            if !objects.is_empty() {
                declaring += 1;
            }
            for s in objects {
                if let crate::indexer::facts::DeclaredType::Stated(label) = &s.declared_type {
                    *kinds.entry(label.clone()).or_default() += 1;
                }
                sites
                    .entry(s.fqn.as_str().to_string())
                    .or_default()
                    .insert(format!("{} at {rel}", s.name));
            }
            for r in &facts.references {
                match r.kind {
                    crate::indexer::facts::RefKind::Reads => reads += 1,
                    crate::indexer::facts::RefKind::Writes => writes += 1,
                    crate::indexer::facts::RefKind::Calls => calls += 1,
                    _ => structural += 1,
                }
            }
        }

        let colliding: Vec<_> = sites.iter().filter(|(_, at)| at.len() > 1).collect();
        println!("\n── PostgreSQL through dbd ──");
        println!("postgres files  {files} ({refused} refused)");
        println!("  declaring     {declaring}");
        println!("  objects       {kinds:?}");
        println!("  reads {reads}  writes {writes}  calls {calls}  structural {structural}");
        println!("COLLIDING       {}", colliding.len());
        for (head, n) in &why {
            println!("  refused {n:>4}x  {head}");
        }
        for (fqn, at) in colliding.iter().take(6) {
            println!("    {fqn}  {:?}", at.iter().take(2).collect::<Vec<_>>());
        }

        // THE SPLIT IS THE POINT — but only a corpus with ROUTINES can show
        // it. A schema of tables and views has nothing to read or write from,
        // and magpie's is exactly that: 77 files, 62 tables, 15 views, zero
        // functions. Asserting `reads > 0` unconditionally made this gate fail
        // on a corpus it had no complaint about, which is a test asserting a
        // property of the CORPUS rather than of the seam.
        let routines: usize = kinds.get("function").copied().unwrap_or(0)
            + kinds.get("procedure").copied().unwrap_or(0);
        if routines == 0 {
            println!("  (no routines here, so the read/write split is untestable on this corpus)");
        } else {
            assert!(
                reads + writes > 0,
                "{routines} routines and no table access — the split dbd is called for was dropped"
            );
        }

        // EVERY entity dbd reported became a node, or the seam lost one.
        assert!(
            colliding.is_empty(),
            "{} identities minted twice: {:?}",
            colliding.len(),
            colliding.iter().take(3).collect::<Vec<_>>()
        );
    }

    /// **A7 + COVERAGE for T-SQL over a real corpus.**
    ///
    /// Two questions at once, because for this language they are the same
    /// question asked twice. A7 asks whether two declarations mint one
    /// identity; COVERAGE asks whether a reader built on statement heads finds
    /// the declarations at all — and a reader that finds none trivially passes
    /// A7, so the two have to be read together.
    ///
    ///     SENSEI_CORPUS=/path/to/sql cargo test -p senseid --bin senseid \
    ///       sql::tests::the_t_sql_corpus_declares_what_it_creates \
    ///       -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_t_sql_corpus_declares_what_it_creates() {
        use std::collections::{BTreeMap, BTreeSet};

        let Ok(root) = std::env::var("SENSEI_CORPUS") else {
            println!("SENSEI_CORPUS unset — nothing to read. See this test's docs.");
            return;
        };

        let mut by_repo: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut unreadable = 0usize;
        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "sql" && e != "ddl") {
                continue;
            }
            // THE PRODUCTION DECODER, not `read_to_string`. SSMS exports
            // UTF-16LE and `read_to_string` refuses it, so a gate using one
            // would measure a corpus the indexer no longer sees.
            let Ok(bytes) = std::fs::read(path) else {
                unreadable += 1;
                continue;
            };
            let crate::classifiers::Decoded::Text(text) = crate::classifiers::decode_source(&bytes)
            else {
                unreadable += 1;
                continue;
            };
            let mut dir = path.parent();
            let mut repo = root.clone();
            while let Some(d) = dir {
                if d.join(".git").exists() {
                    repo = d.to_string_lossy().to_string();
                    break;
                }
                dir = d.parent();
            }
            by_repo.entry(repo).or_default().push((path.to_string_lossy().to_string(), text));
        }
        if by_repo.is_empty() {
            println!("no SQL under SENSEI_CORPUS — nothing to measure.");
            return;
        }

        let mut by_dialect: BTreeMap<String, usize> = BTreeMap::new();
        let mut tsql_files = 0usize;
        let mut declared = 0usize;
        let mut with_decl = 0usize;
        let mut refs = 0usize;
        let mut qualified_refs = 0usize;
        let mut colliding: Vec<(String, BTreeSet<String>)> = Vec::new();
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        let mut silent: Vec<String> = Vec::new();
        let mut missed = 0usize;
        let mut no_declaration = 0usize;
        let mut text_of: BTreeMap<String, String> = BTreeMap::new();

        for (repo, sources) in &by_repo {
            let package = repo.rsplit('/').next().unwrap_or("pkg").to_string();
            let mut sites: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for (path, text) in sources {
                let dialect = Dialect::detect(text);
                *by_dialect.entry(format!("{dialect:?}")).or_default() += 1;
                // **ONLY THE T-SQL FILES.** This gate is labelled T-SQL and its
                // numbers are only actionable if that is what they measure.
                //
                // It used to feed EVERY file to this crate's own `tsql::read`
                // and let the reader refuse what it could not handle, which
                // filtered the corpus as a side effect. dbd's reader is a
                // lenient statement-head walk that accepts almost anything, so
                // the same loop started reading 78 PostgreSQL and 187
                // dialect-less files AS T-SQL — reporting them as misses and
                // minting a declaration called `if`. The filter has to be
                // explicit now that the reader no longer does it by accident.
                if dialect != Dialect::TSql {
                    continue;
                }
                let rel = path.strip_prefix(repo).unwrap_or(path).trim_start_matches('/');
                let source = Source { package: &package, module: rel, path: rel, text };
                let Ok(facts) = facts::read(Dialect::TSql, &source, &TypeHomes::unknown()) else {
                    continue;
                };
                tsql_files += 1;
                text_of.insert(rel.to_string(), text.clone());
                let objects: Vec<&crate::indexer::facts::Symbol> = facts
                    .symbols
                    .iter()
                    .filter(|s| s.kind != crate::indexer::facts::SymbolKind::Module)
                    .collect();
                if objects.is_empty() {
                    // **CLASSIFY THE FAILURES.** A file that declares nothing
                    // is only a MISS if it contains a declaring statement. A
                    // pure INSERT, UPDATE or `ALTER TABLE ADD` script correctly
                    // declares nothing, and counting those as misses would
                    // make the reader look broken where it is right.
                    // WHOLE WORDS, and the word right after the verb. A
                    // `contains` check said `ALTER COLUMN [ViewedBy]` declared
                    // a VIEW — the keyword sits inside the column's name — and
                    // reported three misses that were the classifier's own.
                    // The same substring-in-a-word trap as `"CXX".contains("C")`.
                    let declaring_line = text.lines().any(|l| {
                        let mut w = l.split_whitespace();
                        let verb = w.next().unwrap_or("");
                        if !verb.eq_ignore_ascii_case("create")
                            && !verb.eq_ignore_ascii_case("alter")
                        {
                            return false;
                        }
                        let mut next = w.next().unwrap_or("");
                        if next.eq_ignore_ascii_case("or") {
                            w.next();
                            next = w.next().unwrap_or("");
                        }
                        ["procedure", "proc", "function", "view", "trigger"]
                            .iter()
                            .any(|k| next.eq_ignore_ascii_case(k))
                    });
                    if declaring_line {
                        missed += 1;
                        if silent.len() < 8 {
                            silent.push(format!("MISS {rel}"));
                        }
                    } else {
                        no_declaration += 1;
                    }
                } else {
                    with_decl += 1;
                }
                for s in &objects {
                    declared += 1;
                    if let crate::indexer::facts::DeclaredType::Stated(label) = &s.declared_type {
                        *kinds.entry(label.clone()).or_default() += 1;
                    }
                    sites
                        .entry(s.fqn.as_str().to_string())
                        .or_default()
                        .insert(format!("{} at {rel}:{}", s.name, s.span.start_line));
                }
                refs += facts.references.len();
                qualified_refs += facts
                    .references
                    .iter()
                    .filter(|r| {
                        matches!(r.target, crate::indexer::facts::Resolution::Resolved { .. })
                    })
                    .count();
            }
            colliding.extend(sites.into_iter().filter(|(_, at)| at.len() > 1));
        }

        println!("\n── T-SQL over this corpus ──");
        println!("repositories  {}", by_repo.len());
        println!(
            "sql files     {} ({unreadable} unreadable — UTF-16)",
            by_repo.values().map(Vec::len).sum::<usize>()
        );
        println!("by dialect    {by_dialect:?}");
        println!("read as T-SQL {tsql_files}");
        println!(
            "  declaring an object {with_decl} ({:.0}%)",
            100.0 * with_decl as f64 / tsql_files.max(1) as f64
        );
        println!("  objects             {declared} {kinds:?}");
        println!("  references          {refs} ({qualified_refs} schema-qualified and placed)");
        println!(
            "  declaring NOTHING   {} correctly (no CREATE/ALTER of a routine), {missed} MISSED",
            no_declaration
        );
        println!("COLLIDING     {}", colliding.len());

        // A COPY is identical CONTENT. Ethico ships a folder per release, so
        // the same procedure appears in `4.2.1/` and `4.3.0.2/` — genuinely one
        // object, deployed twice.
        let (mut copies, mut versioned, mut same_file) = (0usize, 0usize, 0usize);
        for (_, at) in &colliding {
            let paths: std::collections::BTreeSet<&str> = at
                .iter()
                .filter_map(|s| s.rsplit_once(" at "))
                .filter_map(|(_, f)| f.rsplit_once(':'))
                .map(|(p, _)| p)
                .collect();
            if paths.len() <= 1 {
                same_file += 1;
                continue;
            }
            let bodies: std::collections::BTreeSet<&str> =
                paths.iter().filter_map(|p| text_of.get(*p)).map(String::as_str).collect();
            if bodies.len() == 1 {
                copies += 1;
            } else {
                versioned += 1;
            }
        }
        println!("  identical copies  {copies}");
        println!("  same object, different releases {versioned}");
        println!("  WITHIN ONE FILE   {same_file}");
        println!("  files the reader found nothing in:");
        for s in &silent {
            println!("    {s}");
        }
        for (fqn, at) in colliding.iter().take(8) {
            println!("    {fqn}");
            for site in at.iter().take(3) {
                println!("        {site}");
            }
        }
        assert!(
            colliding.len() <= A7_BOUND,
            "{} colliding identities, was {A7_BOUND}. Read the decomposition above first.",
            colliding.len()
        );
    }

    /// **THE DIALECT IS SCORED, AND A TIE IS NOT A DIALECT.**
    ///
    /// MUTATION: return on the first marker seen — a Postgres migration that
    /// mentions `nvarchar` once in a comment is read as T-SQL, and every
    /// declaration in it is handed to the wrong reader.
    /// **THE GRAMMARS dbd HAS ARE READ; THE ONES IT LACKS ARE SAID SO.**
    ///
    /// I expected 0.15.0 to add a MySQL parser and wrote this test asserting
    /// four dialects read. It does not. `ParserChoice::for_dialect_typed` maps
    /// SQLite to `Verbatim` — which `parse_sql_with` errors on — and **MySQL to
    /// `PgQuery`**, and `dbd-core/src/parser/` contains only `pg/` and
    /// `tsql.rs`.
    ///
    /// So routing MySQL through `parse_sql_as` would not read it: it would
    /// parse it as PostgreSQL and fail on the first backtick with
    /// `syntax error at or near "`"`. That reports a file as MALFORMED when the
    /// truth is that we lack its grammar, and those must stay different
    /// answers — one is the author's problem, the other is ours.
    ///
    /// MUTATION: let the `other` arm fall through to `facts::read` and the
    /// MySQL row below starts reporting a syntax error instead of a refusal.
    #[test]
    fn a_grammar_we_lack_is_refused_not_reported_as_broken() {
        let read = |label: &str, text: &str| {
            SqlAdapter.read(
                &Source {
                    package: "db",
                    module: "schema",
                    path: &format!("schema/{label}.sql"),
                    text,
                },
                &TypeHomes::unknown(),
            )
        };

        // The two dbd has a grammar for.
        for (label, text, expect) in [
            ("tsql", "CREATE TABLE [dbo].[Orders] (Id int);\nGO\n", "Orders"),
            (
                "postgres",
                "CREATE TABLE public.orders (id int);\nCREATE FUNCTION f() RETURNS int AS $$ SELECT 1 $$ LANGUAGE sql;\n",
                "orders",
            ),
        ] {
            let facts = read(label, text).unwrap_or_else(|e| panic!("{label} must be read: {e:?}"));
            let names: Vec<&str> = facts.symbols.iter().map(|s| s.name.as_str()).collect();
            assert!(names.iter().any(|n| n.eq_ignore_ascii_case(expect)), "{label}: {names:?}");
        }

        // The ones it does not. `GrammarUnavailable`, never `NotParsedBecause`.
        for (label, text) in [
            ("mysql", "CREATE TABLE `orders` (id INT) ENGINE=InnoDB;\n"),
            ("sqlite", "CREATE TABLE orders (id INTEGER) WITHOUT ROWID;\nPRAGMA foo;\n"),
            ("plain", "CREATE TABLE t (id int);\n"),
        ] {
            match read(label, text) {
                Err(crate::indexer::lang::ReadError::GrammarUnavailable(_)) => {}
                other => panic!("{label} must be refused for want of a grammar, got {other:?}"),
            }
        }
    }

    #[test]
    fn the_dialect_is_the_one_with_the_most_markers() {
        let d = Dialect::detect;
        assert_eq!(
            d("SET ANSI_NULLS ON\nGO\nCREATE TABLE [dbo].[x] (a nvarchar(10))"),
            Dialect::TSql
        );
        assert_eq!(
            d("CREATE FUNCTION f() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN END $$;"),
            Dialect::PostgreSql
        );
        assert_eq!(d("CREATE TABLE t (id int AUTO_INCREMENT) ENGINE=InnoDB;"), Dialect::MySql);
        assert_eq!(d("CREATE TABLE t (id integer PRIMARY KEY AUTOINCREMENT);"), Dialect::Sqlite);
    }

    /// **DETECTION FAILS CLOSED.** A statement valid in every dialect states no
    /// dialect.
    ///
    /// MUTATION: default to `TSql` — every ambiguous file in the corpus is fed
    /// to the T-SQL reader on an assumption the source never made.
    #[test]
    fn a_file_with_no_marker_states_no_dialect() {
        assert_eq!(Dialect::detect("CREATE TABLE t (id int);"), Dialect::Unstated);
        assert_eq!(Dialect::detect(""), Dialect::Unstated);
        assert_eq!(Dialect::detect("-- just a comment\n"), Dialect::Unstated);
    }

    /// **A QUOTE CHARACTER IS NOT A DIALECT STATEMENT.**
    ///
    /// A backtick is MySQL's identifier quote, and it was in the marker list.
    /// It is also what everybody writes around a word in a COMMENT, and a
    /// well-commented schema is full of them: measured over sensei's own
    /// Postgres DDL, 2,132 backticks across 178 files, every one in prose like
    /// `-- \`named\` = public credit`. That made 204 of 610 dbd DDL files —
    /// a third — detect as MySQL, and each would have been handed to the wrong
    /// reader.
    ///
    /// The fourth instance of one shape on this work: `"CXX".contains("C")`,
    /// `go` inside `logo`, `VIEW` inside `ViewedBy`, and now a quote character
    /// inside a sentence. A marker has to be a thing only that dialect WRITES,
    /// not a character it happens to use.
    ///
    /// MUTATION: put the backtick back — a third of every commented Postgres
    /// schema reads as MySQL.
    #[test]
    fn a_backtick_in_a_comment_does_not_make_a_schema_mysql() {
        let commented = "-- `named` = public credit; `anonymous` = rotated\n\
                         CREATE TABLE attribution (mode text);";
        assert_ne!(Dialect::detect(commented), Dialect::MySql);

        // MySQL is still detected by what it actually WRITES.
        assert_eq!(
            Dialect::detect("CREATE TABLE t (id int AUTO_INCREMENT) ENGINE=InnoDB;"),
            Dialect::MySql
        );
    }

    /// `go` is a word in a hundred names. Only a LINE that is exactly `GO`
    /// counts.
    ///
    /// MUTATION: count `go` as a substring — `category`, `logo` and `goal` all
    /// score for T-SQL, so a Postgres file full of them is misread.
    #[test]
    fn the_word_go_inside_a_name_is_not_a_batch_separator() {
        // Postgres markers present, `go` only ever inside words.
        let pg = "CREATE TABLE category (logo text, goal jsonb);\nSELECT x::text FROM category;";
        assert_eq!(Dialect::detect(pg), Dialect::PostgreSql);
    }

    /// A manifest STATES the dialect, and what it states outranks detection.
    #[test]
    fn a_manifest_label_maps_to_a_dialect_and_an_unknown_one_does_not() {
        assert_eq!(Dialect::from_label("postgresql"), Some(Dialect::PostgreSql));
        assert_eq!(Dialect::from_label("  PostgreSQL "), Some(Dialect::PostgreSql));
        assert_eq!(Dialect::from_label("sqlserver"), Some(Dialect::TSql));
        assert_eq!(Dialect::from_label("mariadb"), Some(Dialect::MySql));
        // An unrecognised label is NOT silently mapped to a default — the
        // caller gets `None` and decides.
        assert_eq!(Dialect::from_label("oracle"), None);
        assert_eq!(Dialect::from_label(""), None);
    }

    /// A qualified name's OBJECT is its last part, quotes and all removed.
    #[test]
    fn a_qualified_name_resolves_to_the_object_it_names() {
        let seg = |s: &str| SqlAdapter.type_segment(s).expect("names an object");
        assert_eq!(seg("Issues"), "Issues");
        assert_eq!(seg("dbo.Issues"), "Issues");
        assert_eq!(seg("[dbo].[Issues]"), "Issues");
        assert_eq!(seg("\"dbo\".\"Issues\""), "Issues");
        assert_eq!(seg("mydb.dbo.Issues"), "Issues");
        assert!(SqlAdapter.type_segment("").is_err());
        assert!(SqlAdapter.type_segment("[]").is_err());
    }
}
