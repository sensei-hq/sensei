//! C# — identity rules and the grammar the ladder climbs.
//!
//! The walk is in [`walk`]; this file owns what a C# name MEANS, which is R7's
//! split.
//!
//! # C# is Java's nearest relative, and the identity model is Java's
//!
//! A namespace is a package: `namespace Ethico.Policy.Services` is the complete
//! answer to where a declaration lives, it is written in the source rather than
//! inferred from the path, and two files in one namespace see each other with no
//! `using` at all. So a C# fqn carries the declared namespace in `<package>` and
//! leaves `<module>` EMPTY, exactly as Java does, and for the same reason:
//! deriving a module from the directory would disagree with the source wherever
//! the two drift, which is legal, common, and silent.
//!
//! Nothing here can tell first-party from library either. `Ethico.Policy.Rule`
//! and `System.Text.StringBuilder` are the same shape; C# marks the boundary
//! with nothing, because the compiler answers from the assembly references and a
//! source file never states it. So every `using` is reported external naming its
//! namespace, and `Ladder::owned_by_this_scan` flips the ones this scan owns.
//!
//! # Four things C# has that Java does not
//!
//! - **A file-scoped namespace.** `namespace X;` with no braces applies to the
//!   whole file. Both forms are read; the block form can also nest.
//! - **Properties are declarations.** `public int Count { get; set; }` is
//!   reached like a field and is code, so it is [`SymbolKind::Property`] — the
//!   distinction the fact vocabulary already carries for exactly this.
//! - **`using static` and aliases.** `using static System.Math;` brings a
//!   TYPE's members into scope and `using Sb = System.Text.StringBuilder;`
//!   renames one. Neither is the plain form, and the walk emits each as its own
//!   shape rather than as a bare name — see
//!   [`super::common::specifier_names_a_module`].
//! - **Partial types.** `partial class Order` may be declared in several files
//!   of one namespace, and every part is the SAME type. That is not a collision
//!   and must not be reported as one; see [`walk`].
//!
//! # The two rules every language here has needed
//!
//! Both are built in from the start rather than discovered by measurement, which
//! is what the rust, TypeScript, Java and Python walks each had to do:
//!
//! 1. **A container is a PATH, not a leaf** — a type nested in another is
//!    `Outer.Inner`, which is also what C# calls it.
//! 2. **A body does not declare members of the type enclosing it** — a local in
//!    a method is not a field of the class.

use std::sync::LazyLock;

use super::{LanguageAdapter, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach, Segment};
use crate::indexer::resolve::Grammar;

mod walk;

/// The names `System` puts within reach of most files, as
/// `(name, namespace, path)`.
///
/// Not the whole BCL — the names a body actually writes. A list trying to be
/// complete would be a second, worse copy of the framework, and every name
/// missing from it is a miss the histogram reports honestly
/// (`NoImportInScope`) rather than a wrong edge.
///
/// `System` is not implicitly in scope the way `java.lang` is — a file must
/// write `using System;`. It is listed anyway because C# 10's IMPLICIT USINGS
/// put `System` (and more) into every file of a modern SDK-style project
/// without a line of source saying so, and a reader has no way to tell from the
/// file which regime it is under.
const PRELUDE: &[(&str, &str, &str)] = &[
    ("Object", "System", "Object"),
    ("String", "System", "String"),
    ("Int32", "System", "Int32"),
    ("Int64", "System", "Int64"),
    ("Boolean", "System", "Boolean"),
    ("Decimal", "System", "Decimal"),
    ("Double", "System", "Double"),
    ("Guid", "System", "Guid"),
    ("DateTime", "System", "DateTime"),
    ("DateTimeOffset", "System", "DateTimeOffset"),
    ("TimeSpan", "System", "TimeSpan"),
    ("Exception", "System", "Exception"),
    ("Console", "System", "Console"),
    ("Math", "System", "Math"),
    ("Convert", "System", "Convert"),
    ("Task", "System.Threading.Tasks", "Task"),
    ("List", "System.Collections.Generic", "List"),
    ("Dictionary", "System.Collections.Generic", "Dictionary"),
    ("IEnumerable", "System.Collections.Generic", "IEnumerable"),
    ("IList", "System.Collections.Generic", "IList"),
    ("ICollection", "System.Collections.Generic", "ICollection"),
    ("HashSet", "System.Collections.Generic", "HashSet"),
    ("IQueryable", "System.Linq", "IQueryable"),
];

/// Members on `System.Object`, and the LINQ extension methods that sit on every
/// sequence. A call to one of these says nothing about the receiver's type, so
/// counting it as a first-party edge would be noise.
const PLUMBING: &[&str] = &[
    // System.Object — on every receiver there is.
    "ToString",
    "Equals",
    "GetHashCode",
    "GetType",
    "MemberwiseClone",
    "ReferenceEquals",
    // Disposal and async plumbing, on a large fraction of receivers.
    "Dispose",
    "DisposeAsync",
    "ConfigureAwait",
    "GetAwaiter",
    // LINQ, which is an extension method on IEnumerable rather than a member of
    // anything the scan declares.
    "Select",
    "Where",
    "FirstOrDefault",
    "SingleOrDefault",
    "ToList",
    "ToArray",
    "Any",
    "All",
    "Count",
    "OrderBy",
    "OrderByDescending",
];

/// Whether a use site's spelling NAMES a type rather than a value.
///
/// C#'s convention is PascalCase for types and camelCase for locals and
/// parameters, and it is followed with unusual consistency — but a convention
/// is a lint, not a rule, so this answers only what the grammar makes certain:
/// a name reached through `new` or standing in a type position is a type, and
/// the walk knows which position it read the name from. Everything else is left
/// to the ladder.
fn names_a_type(_raw: &str) -> bool {
    false
}

pub static GRAMMAR: LazyLock<Grammar> = LazyLock::new(|| Grammar {
    language: Language::CSharp,
    // One separator for both, because C# has one namespace. `System.Text` is a
    // path and a namespace, spelled identically because they are the same kind
    // of thing — the property Java has and Rust does not.
    path_separator: ".",
    module_separator: ".",
    // EMPTY. C# has `global::` as an escape hatch, but it roots at the same
    // place an unqualified name already resolves from, so it names no separate
    // origin for the ladder to climb to.
    roots: &[],
    // A `using` names a namespace, never a file, so there is no extension in it
    // — and C#'s separator is the dot a stem rule would cut.
    module_segment: crate::indexer::lang::common::already_a_module_segment,
    relative_depth_prefix: None,
    names_the_binding: None,
    // C# has no wildcard `using`. `using System.Text;` already brings every type
    // of that namespace into scope — the whole directive IS the wildcard, which
    // is why the plain form is a module specifier rather than an item one.
    wildcard: None,
    paths_name_packages: true,
    relative_to_directory: false,
    names_a_type,
    prelude: PRELUDE,
    plumbing: PLUMBING,
});

/// The type a C# type expression NAMES.
///
/// A free function because the walk needs it too, and a second copy is how the
/// declaration side and the reference side drift apart.
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
    let bare = raw.trim();
    // A NULLABLE names the type it makes nullable. `string?` is a use of
    // `string`, and `Order?` of `Order`.
    let bare = bare.strip_suffix('?').unwrap_or(bare);
    // A generic instantiation names its own type: `List<Order>` is a use of
    // `List`. The argument is a separate use site the walk emits separately, so
    // dropping it here loses nothing.
    let bare = bare.split_once('<').map_or(bare, |(head, _)| head);
    // An array names its element type.
    let bare = bare.trim_end_matches("[]").trim();
    // Nullable again, because `List<int>?` puts the `?` outside the generic.
    let bare = bare.strip_suffix('?').unwrap_or(bare).trim();
    // `System.Text.StringBuilder` at a use site names `StringBuilder`; the path
    // in front of it is how it was reached, not what it is.
    let last = bare.rsplit('.').next().unwrap_or(bare).trim();
    // `global::Ethico.Order` — the escape hatch is not part of the name.
    let last = last.rsplit("::").next().unwrap_or(last).trim();
    if last.is_empty() {
        return Err(FqnError::EmptySegment { segment: Segment::Type });
    }
    Ok(last.to_string())
}

/// C#, from `.cs`.
pub struct CSharpAdapter;

impl LanguageAdapter for CSharpAdapter {
    fn language(&self) -> Language {
        Language::CSharp
    }

    fn name(&self) -> &'static str {
        "csharp"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".cs"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, super::ReadError> {
        walk::read(source, types)
    }

    fn file_fqn(&self, package: &str, _module: &str, path: &str) -> Result<Fqn, FqnError> {
        let stem = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".cs");
        fqn::define(&Form::Item {
            lang: Language::CSharp,
            package,
            module: "",
            name: stem,
            reach: Reach::Mod,
        })
    }

    /// EMPTY, always — the namespace the walk reads IS the module, and it is
    /// reported back as the package. See this module's header.
    fn module_path(&self, _file: &str, _package_root: &str) -> String {
        String::new()
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        type_segment(raw)
    }

    /// A C# file's identity does not move when the file does.
    ///
    /// The namespace is declared IN the file, so renaming or relocating it mints
    /// nothing new. The one thing that re-mints an identity is editing the
    /// `namespace` line, which is a content change and reaches this function as
    /// neither argument.
    fn rename_remints_identity(&self, _from: &str, _to: &str, _package_root: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A7 for C#: no two declarations mint one identity.**
    ///
    /// Built BEFORE the flip, not after, because the absence of exactly this
    /// measurement is what let TypeScript reach 511 collisions, Java 409 and
    /// Python 72 — each found only when someone went looking. This repository
    /// holds no C#, so acceptance cannot measure it and the answer would be an
    /// empty denominator; the corpus is somebody else's checkout named by
    /// `SENSEI_CORPUS`, which is why this is `#[ignore]`d.
    ///
    /// PARTITIONED BY REPOSITORY. An identity is scoped to a FOLDER because the
    /// scan indexes per repo; pooling asks a question production never asks, and
    /// on Java's corpus that read 16,561 collisions where the real figure was
    /// 409 purely because one repo vendored a copy of another.
    ///
    /// PARTIAL TYPES ARE NOT COLLISIONS. `partial class Order` declared across
    /// several files is ONE type by the language's own rule, so the parts are
    /// counted apart and reported rather than asserted against.
    ///
    ///     SENSEI_CORPUS=/path/to/csharp cargo test -p senseid --bin senseid \
    ///       csharp::tests::no_two_declarations_in_this_corpus_mint_one_identity \
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
            if path.extension().is_none_or(|e| e != "cs") {
                continue;
            }
            let shown = path.to_string_lossy().to_string();
            // Build output and generated designer files are a property of a
            // toolchain rather than of this reader.
            if ["/bin/", "/obj/", "/packages/", ".Designer.cs", ".g.cs", ".g.i.cs"]
                .iter()
                .any(|skip| shown.contains(skip))
            {
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
            println!("no C# under SENSEI_CORPUS — nothing to measure.");
            return;
        }

        let mut files = 0usize;
        let mut declarations = 0usize;
        let mut identities = 0usize;
        let mut colliding: Vec<(String, BTreeSet<String>)> = Vec::new();
        let mut unreadable = 0usize;
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

        println!("\n── A7: one declaration, one identity (csharp) ──");
        println!("repositories {}", by_repo.len());
        println!("files        {files} ({unreadable} unreadable)");
        println!("declarations {declarations}");
        println!("identities   {identities}");
        println!("COLLIDING    {}", colliding.len());

        // DECOMPOSE before concluding — reading a shape off the first few
        // samples has been wrong three times on this work.
        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        let mut one_file = 0usize;
        let mut same_name_files = 0usize;
        let mut different_files = 0usize;
        let mut examples: Vec<String> = Vec::new();
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
                continue;
            }
            let bases: BTreeSet<&str> =
                paths.iter().map(|p| p.rsplit('/').next().unwrap_or(p)).collect();
            if bases.len() == 1 {
                same_name_files += 1;
            } else {
                different_files += 1;
                if examples.len() < 8 {
                    examples.push(format!(
                        "{fqn}\n        {}",
                        paths.iter().take(3).cloned().collect::<Vec<_>>().join("\n        ")
                    ));
                }
            }
        }
        println!("  by kind         {by_kind:?}");
        println!("  one file        {one_file}");
        println!(
            "  same filename   {same_name_files} (copies of one file — one identity is CORRECT)"
        );
        println!("  different files {different_files} (PARTIAL types, or a defect)");
        for e in &examples {
            println!("    {e}");
        }
        // THE BOUND SITS AT THE MEASUREMENT, not above it. A ceiling with slack
        // silently absorbs a new defect until the slack runs out, which this
        // repo has already paid for once (A4's rust ceiling).
        //
        // 452 of 129,498 declarations — 0.35% — decomposed on 2026-09-23 over
        // Ethico's 8,647 files:
        //   230 same filename   copies of one file, where one identity is CORRECT
        //   110 different files almost entirely PARTIAL types, which are one type
        //                       by the language's own rule — `.partial.cs`,
        //                       `EmailTemplateDto.Forms.cs`, `_LocationDto.cs` —
        //                       plus one genuine leftover duplicate whose
        //                       filename is a misspelling of its sibling's
        //   112 one file        the residue, unclassified
        //
        // SHARPENING THIS NEEDS `partial` ON THE SYMBOL. The gate cannot tell a
        // partial type from a defect today because nothing records the modifier,
        // so the cross-file bucket mixes the language's answer with a real fault.
        // That is the next thing to do here, and it is why the bound is a total
        // rather than a per-bucket zero.
        assert!(
            colliding.len() <= 452,
            "{} colliding identities, was 452 over this corpus. Read the decomposition above \
             before moving this number: `same filename` and PARTIAL types are correct, and a \
             rise in `one file` is a defect in the walk.",
            colliding.len()
        );
    }

    fn seg(raw: &str) -> String {
        CSharpAdapter.type_segment(raw).expect("the fixture names a type")
    }

    /// Every shape a C# type expression takes, reduced to the type it NAMES.
    ///
    /// The nullable suffix is the one Java has no equivalent of, and it appears
    /// in two positions — `string?` and `List<int>?` — so it is stripped on both
    /// sides of the generic.
    ///
    /// MUTATION: drop either `strip_suffix('?')` and the nullable spellings mint
    /// a type segment ending in `?`, which no declaration can ever equal.
    #[test]
    fn a_type_expression_names_its_own_type() {
        assert_eq!(seg("Order"), "Order");
        assert_eq!(seg("System.Text.StringBuilder"), "StringBuilder");
        assert_eq!(seg("List<Order>"), "List");
        assert_eq!(seg("Order[]"), "Order");
        assert_eq!(seg("string?"), "string");
        assert_eq!(seg("List<int>?"), "List");
        assert_eq!(seg("global::Ethico.Order"), "Order");
        assert_eq!(seg("Dictionary<string, List<Order>>"), "Dictionary");
    }

    /// An empty segment is a named ERROR, never a silently-minted identity.
    #[test]
    fn a_type_expression_that_names_nothing_is_an_error() {
        assert!(CSharpAdapter.type_segment("").is_err());
        assert!(CSharpAdapter.type_segment("   ").is_err());
    }
}
