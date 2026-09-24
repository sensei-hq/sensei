//! Kotlin — identity rules and the grammar the ladder climbs.
//!
//! The walk is in [`walk`]; this file owns what a Kotlin name MEANS, which is
//! R7's split.
//!
//! # A package is the namespace, as in Java
//!
//! `package com.sg.alert.ui` is written in the source and is authoritative over
//! anything a directory suggests — Kotlin does not even require the two to
//! agree. So a Kotlin fqn carries the declared package in `<package>` and leaves
//! `<module>` EMPTY, exactly as Java and C# do.
//!
//! Nothing here can tell first-party from library either. `com.sg.alert.Repo`
//! and `kotlinx.coroutines.flow.Flow` are the same shape, so every `import` is
//! reported external naming its package and `Ladder::owned_by_this_scan` flips
//! the ones this scan owns.
//!
//! # Three things Kotlin has that Java does not
//!
//! - **Top-level declarations.** A `fun` or a `val` may sit outside any class,
//!   which is why [`walk`]'s file container carries real weight here — in Java
//!   it never does.
//! - **A primary constructor that declares members.** `class User(val id: Int)`
//!   declares a property, the same shape C#'s positional records have, and
//!   reading only the class body misses it.
//! - **`object`, and `companion object`.** A singleton is a type declaration
//!   whose name is its own; a companion is a nested one that may be ANONYMOUS,
//!   in which case Kotlin calls it `Companion` and so does this.
//!
//! # The two rules every language here has needed
//!
//! Built in from the start, as they were for C#, rather than found by
//! measurement afterwards:
//!
//! 1. **A container is a PATH, not a leaf** — `Outer.Inner`, Kotlin's own
//!    spelling for a nested type.
//! 2. **A body does not declare members of the type enclosing it** — a local in
//!    a function is not a property of the class.

use std::sync::LazyLock;

use super::{LanguageAdapter, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach, Segment};
use crate::indexer::resolve::Grammar;

mod walk;

/// The names `kotlin` and `kotlin.collections` put in every file with no
/// import, as `(name, package, path)`.
///
/// Kotlin's default imports are a defined list, unlike C#'s implicit usings —
/// but it is a long one, and this is the part a body actually writes. Every name
/// missing from it is a miss the histogram reports honestly
/// (`NoImportInScope`) rather than a wrong edge.
const PRELUDE: &[(&str, &str, &str)] = &[
    ("Any", "kotlin", "Any"),
    ("Unit", "kotlin", "Unit"),
    ("Nothing", "kotlin", "Nothing"),
    ("String", "kotlin", "String"),
    ("Int", "kotlin", "Int"),
    ("Long", "kotlin", "Long"),
    ("Double", "kotlin", "Double"),
    ("Float", "kotlin", "Float"),
    ("Boolean", "kotlin", "Boolean"),
    ("Char", "kotlin", "Char"),
    ("Throwable", "kotlin", "Throwable"),
    ("Exception", "kotlin", "Exception"),
    ("Result", "kotlin", "Result"),
    ("Pair", "kotlin", "Pair"),
    ("Triple", "kotlin", "Triple"),
    ("Lazy", "kotlin", "Lazy"),
    ("List", "kotlin.collections", "List"),
    ("MutableList", "kotlin.collections", "MutableList"),
    ("Map", "kotlin.collections", "Map"),
    ("MutableMap", "kotlin.collections", "MutableMap"),
    ("Set", "kotlin.collections", "Set"),
    ("MutableSet", "kotlin.collections", "MutableSet"),
];

/// Members on `kotlin.Any`, and the standard-library extensions that sit on
/// every receiver. A call to one says nothing about the receiver's type.
const PLUMBING: &[&str] = &[
    // kotlin.Any — on every receiver there is.
    "toString",
    "equals",
    "hashCode",
    // The scope functions, which are extensions on everything.
    "let",
    "run",
    "apply",
    "also",
    "takeIf",
    "takeUnless",
    // Collection extensions, which are members of nothing this scan declares.
    "map",
    "filter",
    "forEach",
    "first",
    "firstOrNull",
    "toList",
    "toSet",
    "isEmpty",
    "isNotEmpty",
];

/// Whether a use site's spelling NAMES a type rather than a value.
///
/// Kotlin's convention is PascalCase for types, and it is followed closely — but
/// a convention is a lint, not a rule, so this answers only what the grammar
/// makes certain and leaves the rest to the ladder.
fn names_a_type(_raw: &str) -> bool {
    false
}

pub static GRAMMAR: LazyLock<Grammar> = LazyLock::new(|| Grammar {
    language: Language::Kotlin,
    // One separator for both, because Kotlin has one namespace — the property
    // Java has and Rust does not.
    path_separator: ".",
    module_separator: ".",
    // EMPTY. Kotlin has no `crate::`, no `./` — an import is an absolute name.
    roots: &[],
    module_segment: crate::indexer::lang::common::already_a_module_segment,
    relative_depth_prefix: None,
    names_the_binding: None,
    // `import a.b.*` brings a package into scope.
    wildcard: Some("*"),
    paths_name_packages: true,
    relative_to_directory: false,
    names_a_type,
    prelude: PRELUDE,
    plumbing: PLUMBING,
});

/// The type a Kotlin type expression NAMES.
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
    let bare = raw.trim();
    // A NULLABLE names the type it makes nullable: `String?` is a use of
    // `String`.
    let bare = bare.strip_suffix('?').unwrap_or(bare).trim();
    // A generic instantiation names its own type. The argument is a separate use
    // site the walk emits separately.
    let bare = bare.split_once('<').map_or(bare, |(head, _)| head);
    let bare = bare.strip_suffix('?').unwrap_or(bare).trim();
    // `kotlinx.coroutines.flow.Flow` at a use site names `Flow`.
    let last = bare.rsplit('.').next().unwrap_or(bare).trim();
    if last.is_empty() {
        return Err(FqnError::EmptySegment { segment: Segment::Type });
    }
    Ok(last.to_string())
}

/// Kotlin, from `.kt` and `.kts`.
pub struct KotlinAdapter;

impl LanguageAdapter for KotlinAdapter {
    fn language(&self) -> Language {
        Language::Kotlin
    }

    fn name(&self) -> &'static str {
        "kotlin"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".kt", ".kts"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, super::ReadError> {
        walk::read(source, types)
    }

    fn file_fqn(&self, package: &str, _module: &str, path: &str) -> Result<Fqn, FqnError> {
        let stem = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .trim_end_matches(".kts")
            .trim_end_matches(".kt");
        fqn::define(&Form::Item {
            lang: Language::Kotlin,
            package,
            module: "",
            name: stem,
            reach: Reach::Mod,
        })
    }

    /// EMPTY, always — the package the walk reads IS where a declaration lives,
    /// and Kotlin does not require it to agree with the directory.
    fn module_path(&self, _file: &str, _package_root: &str) -> String {
        String::new()
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        type_segment(raw)
    }

    /// A Kotlin file's identity does not move when the file does — the package
    /// is declared IN the file.
    fn rename_remints_identity(&self, _from: &str, _to: &str, _package_root: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A7 for Kotlin: no two declarations mint one identity.**
    ///
    /// Built BEFORE the flip, as it was for C# — the absence of exactly this
    /// measurement is what let TypeScript reach 511 collisions, Java 409 and
    /// Python 72. This repository holds no Kotlin, so the corpus is somebody
    /// else's checkout named by `SENSEI_CORPUS`.
    ///
    /// PARTITIONED BY REPOSITORY, because an identity is scoped to a FOLDER —
    /// the scan indexes per repo, and pooling asks a question production never
    /// asks.
    ///
    ///     SENSEI_CORPUS=/path/to/kotlin cargo test -p senseid --bin senseid \
    ///       kotlin::tests::no_two_declarations_in_this_corpus_mint_one_identity \
    ///       -- --ignored --nocapture
    #[test]
    #[ignore]
    fn no_two_declarations_in_this_corpus_mint_one_identity() {
        use std::collections::{BTreeMap, BTreeSet};

        let Ok(root) = std::env::var("SENSEI_CORPUS") else {
            println!("SENSEI_CORPUS unset — nothing to read. See this test's docs.");
            return;
        };
        let mut by_repo: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "kt" && e != "kts") {
                continue;
            }
            let shown = path.to_string_lossy().to_string();
            if ["/build/", "/.gradle/"].iter().any(|skip| shown.contains(skip)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else { continue };
            let mut dir = path.parent();
            let mut repo = root.clone();
            while let Some(d) = dir {
                if d.join(".git").exists() {
                    repo = d.to_string_lossy().to_string();
                    break;
                }
                dir = d.parent();
            }
            by_repo.entry(repo).or_default().push((shown, text));
        }
        if by_repo.is_empty() {
            println!("no Kotlin under SENSEI_CORPUS — nothing to measure.");
            return;
        }

        let (mut files, mut unreadable, mut declarations, mut identities) = (0, 0, 0, 0);
        let mut colliding: Vec<(String, BTreeSet<String>)> = Vec::new();
        for (repo, sources) in &by_repo {
            let package = repo.rsplit('/').next().unwrap_or("pkg").to_string();
            let read_all = |types: &TypeHomes| -> Vec<(String, FileFacts)> {
                sources
                    .iter()
                    .filter_map(|(path, text)| {
                        let source = Source { package: &package, module: "", path, text };
                        walk::read(&source, types).ok().map(|f| (path.clone(), f))
                    })
                    .collect()
            };
            let first = read_all(&TypeHomes::unknown());
            unreadable += sources.len() - first.len();
            let homes = TypeHomes::of(
                first.iter().flat_map(|(_, f)| f.symbols.iter().map(|s| (f.package.as_str(), s))),
            );
            let anchored = read_all(&homes);
            files += anchored.len();
            let mut sites: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for (path, facts) in &anchored {
                for symbol in &facts.symbols {
                    declarations += 1;
                    sites.entry(symbol.fqn.as_str().to_string()).or_default().insert(format!(
                        "{:?} {} at {path}:{}",
                        symbol.kind, symbol.name, symbol.span.start_line
                    ));
                }
            }
            identities += sites.len();
            colliding.extend(sites.into_iter().filter(|(_, at)| at.len() > 1));
        }

        println!("\n── A7: one declaration, one identity (kotlin) ──");
        println!("repositories {} | files {files} ({unreadable} unreadable)", by_repo.len());
        println!("declarations {declarations}");
        println!("identities   {identities}");
        println!("COLLIDING    {}", colliding.len());

        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        let (mut one_file, mut across) = (0usize, 0usize);
        let mut examples = 0usize;
        for (fqn, at) in &colliding {
            if let Some(kind) = at.iter().next().and_then(|s| s.split_whitespace().next()) {
                *by_kind.entry(kind.to_string()).or_default() += 1;
            }
            let paths: BTreeSet<&str> = at
                .iter()
                .filter_map(|s| s.rsplit_once(" at "))
                .filter_map(|(_, f)| f.rsplit_once(':'))
                .map(|(p, _)| p)
                .collect();
            if paths.len() <= 1 {
                one_file += 1;
            } else {
                across += 1;
            }
            if examples < 10 {
                examples += 1;
                println!("  {fqn}");
                for site in at.iter().take(3) {
                    println!("      {site}");
                }
            }
        }
        println!("  by kind   {by_kind:?}");
        println!("  one file  {one_file} | across files {across}");

        // THE BOUND SITS AT THE MEASUREMENT. 39 of 3,038 declarations over 245
        // files, and two shapes account for it — NEITHER of which the walk can
        // answer on its own, which is why Kotlin is not in
        // `lang::PRODUCTION_LANGUAGES` yet.
        //
        // ANDROID PRODUCT FLAVOURS are the larger. `app/src/cityofdoral/`,
        // `app/src/ecuador/`, `app/src/guatemala/` and `app/src/panama/` each
        // hold a `Color.kt` declaring the same package and the same top-level
        // properties; exactly one is compiled per build. That is rust's `cfg`
        // problem again — variants of one declaration selected at build time —
        // but expressed through the DIRECTORY rather than a keyword in the
        // source. The callable-plus-arm split cannot see it, because the walk is
        // handed one file and a Gradle source set is a fact about the build.
        // It belongs to placement, which is what tells a file its package.
        //
        // AN ANONYMOUS OBJECT EXPRESSION is the other. `private val M_5_6 =
        // object : Migration(5, 6) { override fun migrate(..) }` declares
        // `migrate` on an object with no name, and three of them in one file
        // mint one identity. The scope wants naming by the property the object
        // is assigned to — the shape TypeScript's "function assigned to a name"
        // already has — and that is a walk-level fix, not yet made.
        assert!(
            colliding.len() <= 39,
            "{} colliding identities, was 39 over this corpus. The two known shapes are Android \
             product flavours and anonymous object expressions; a rise outside them is a defect \
             in the walk.",
            colliding.len()
        );
    }

    fn seg(raw: &str) -> String {
        KotlinAdapter.type_segment(raw).expect("the fixture names a type")
    }

    /// Every shape a Kotlin type expression takes, reduced to what it NAMES.
    ///
    /// MUTATION: drop either `strip_suffix('?')` and a nullable mints a segment
    /// ending in `?`, which no declaration can equal.
    #[test]
    fn a_type_expression_names_its_own_type() {
        assert_eq!(seg("User"), "User");
        assert_eq!(seg("kotlinx.coroutines.flow.Flow"), "Flow");
        assert_eq!(seg("List<User>"), "List");
        assert_eq!(seg("String?"), "String");
        assert_eq!(seg("List<User>?"), "List");
        assert_eq!(seg("Map<String, List<User>>"), "Map");
    }

    #[test]
    fn a_type_expression_that_names_nothing_is_an_error() {
        assert!(KotlinAdapter.type_segment("").is_err());
        assert!(KotlinAdapter.type_segment("  ").is_err());
    }
}
