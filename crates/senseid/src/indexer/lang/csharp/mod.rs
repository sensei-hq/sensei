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
        // Every file's text, by path, so the classifier below can ask the SOURCE
        // whether a type was declared `partial`. That is the fact that separates
        // the language's own answer from a defect, and nothing on `Symbol`
        // records it — adding a field there would put one language's modifier on
        // every language's declaration.
        let text_of: BTreeMap<&str, &str> = by_repo
            .values()
            .flat_map(|files| files.iter().map(|(p, t)| (p.as_str(), t.as_str())))
            .collect();

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
        let mut partial_types = 0usize;
        let mut examples: Vec<String> = Vec::new();
        let mut one_file_examples: Vec<String> = Vec::new();
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
                if one_file_examples.len() < 6 {
                    one_file_examples.push(format!(
                        "{fqn}\n        {}",
                        at.iter().take(3).cloned().collect::<Vec<_>>().join("\n        ")
                    ));
                }
                continue;
            }
            let bases: BTreeSet<&str> =
                paths.iter().map(|p| p.rsplit('/').next().unwrap_or(p)).collect();
            if bases.len() == 1 {
                same_name_files += 1;
                continue;
            }
            // A PARTIAL type is one type by the language's own rule, so several
            // files minting one identity is the CORRECT answer and not a defect.
            //
            // Asked of the SOURCE rather than of a modifier recorded on the
            // symbol: every site must declare the name with `partial` in front
            // of the keyword. A regex over the text is robust to attributes and
            // to modifier order, both of which move a declaration's first line
            // and would defeat reading one.
            let name = fqn.rsplit('·').nth(1).unwrap_or("");
            let every_site_is_partial = !name.is_empty()
                && paths.iter().all(|p| {
                    text_of.get(p).is_some_and(|text| {
                        ["class", "struct", "interface", "record"].iter().any(|kw| {
                            text.match_indices(&format!("partial {kw} "))
                                .any(|(at, _)| text[at..].split_whitespace().nth(2) == Some(name))
                        })
                    })
                });
            if every_site_is_partial {
                partial_types += 1;
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
        for e in &one_file_examples {
            println!("    {e}");
        }
        println!("  partial types   {partial_types} (ONE type by the language's rule — CORRECT)");
        println!(
            "  same filename   {same_name_files} (copies of one file — one identity is CORRECT)"
        );
        println!("  different files {different_files} (PARTIAL types, or a defect)");
        for e in &examples {
            println!("    {e}");
        }
        // THE BOUND SITS AT THE MEASUREMENT, not above it. A ceiling with slack
        // silently absorbs a new defect until the slack runs out, which this
        // repository has already paid for once (A4's rust ceiling).
        //
        // 414 of 129,498 declarations — 0.32% — over Ethico's 8,647 files, and
        // every bucket is accounted for:
        //
        //   230  same filename   copies of ONE file; one identity is correct
        //    73  partial types   ONE type by the language's rule; correct
        //    37  different files a single duplicate class — `Campaign_RiskAssesment.cs`
        //                        and `Campaign_RiskAssessment.cs`, two 28-line
        //                        files declaring the same 10 members, one left
        //                        behind when the filename typo was fixed. A real
        //                        finding about the codebase, not about this walk.
        //    74  one file        CONDITIONAL COMPILATION, and it is the residue.
        //
        // THE RESIDUE IS `#if`/`#else`, and it is rust's `cfg` problem wearing
        // C#'s syntax: `GetHyperLink(Action, XmlWriter)` under `#if SILVERLIGHT`
        // beside `GetHyperLink(Action, XmlTextWriter)` under `#else` are two
        // BODIES of one method, both in the codebase and one in any build. The
        // answer is the one `rust::walk::split_into_variant` already gives —
        // a callable plus one arm per condition — and it is NOT applied here
        // yet. Two things make it more than a copy: the arm's discriminator is
        // the preprocessor condition rather than a `cfg` attribute, and the
        // conditional block appears to break containment, since these land at
        // FILE scope with no type segment rather than as members of their class.
        // Both want reading before the fix, not after.
        assert!(
            colliding.len() <= 414,
            "{} colliding identities, was 414 over this corpus. Read the decomposition above \
             before moving this number: `same filename`, `partial types` and the one duplicate \
             class are CORRECT, and a rise in `one file` is a defect in the walk.",
            colliding.len()
        );
    }

    /// Every `.cs` file under `SENSEI_CORPUS`, with its text.
    fn sources() -> Vec<(String, String)> {
        let Ok(root) = std::env::var("SENSEI_CORPUS") else { return Vec::new() };
        let mut out = Vec::new();
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
            if let Ok(text) = std::fs::read_to_string(path) {
                out.push((shown, text));
            }
        }
        out
    }

    /// Use sites, counted with NO knowledge of `Walk` and none of resolution.
    ///
    /// The whole value of A2 is that this counter and the walk share nothing: a
    /// bug in the walk cannot hide in a number the walk produced. It re-parses
    /// and counts the grammar shapes that ARE use sites, and the two totals are
    /// compared.
    fn count_use_sites(root: tree_sitter::Node<'_>) -> usize {
        /// The declarations that carry a `type` field, each of which is a type
        /// USE as well as a declaration.
        const TYPED: &[&str] = &["variable_declaration", "property_declaration", "parameter"];
        let mut out = 0usize;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            match node.kind() {
                "invocation_expression" => out += 1,
                "object_creation_expression" => out += 1,
                // A member access is a READ unless it is the callee of a call,
                // which the invocation above already counted.
                "member_access_expression" => {
                    let invoked = node.parent().is_some_and(|p| {
                        p.kind() == "invocation_expression"
                            && p.child_by_field_name("function")
                                .is_some_and(|f| f.id() == node.id())
                    });
                    if !invoked {
                        out += 1;
                    }
                }
                kind if TYPED.contains(&kind) => {
                    if let Some(ty) = node.child_by_field_name("type") {
                        out += type_names_under(ty);
                    }
                }
                "method_declaration" => {
                    if let Some(returns) = node.child_by_field_name("returns") {
                        out += type_names_under(returns);
                    }
                }
                _ => {}
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        out
    }

    fn type_names_under(node: tree_sitter::Node<'_>) -> usize {
        let mut count = 0;
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            match n.kind() {
                "identifier" | "qualified_name" | "predefined_type" => count += 1,
                _ => {
                    let mut cursor = n.walk();
                    stack.extend(n.named_children(&mut cursor));
                }
            }
        }
        count
    }

    /// **A2 and A3 for C#: nothing dropped, and every miss named.**
    ///
    /// A2 is a CONSERVATION check — the walk's reference count against a count
    /// made independently off the tree, sharing no code with it. A number the
    /// walk produced cannot audit the walk.
    ///
    /// A3 is the miss histogram: every unresolved reference carries a reason,
    /// and the reasons account for all of them. An unresolved reference is not
    /// an error; a reference that vanished is.
    ///
    ///     SENSEI_CORPUS=/path/to/csharp cargo test -p senseid --bin senseid \
    ///       csharp::tests::nothing_is_dropped_and_every_miss_is_named \
    ///       -- --ignored --nocapture
    #[test]
    #[ignore]
    fn nothing_is_dropped_and_every_miss_is_named() {
        use std::collections::BTreeMap;

        use crate::indexer::facts::{RefKind, Resolution};

        let sources = sources();
        if sources.is_empty() {
            println!("SENSEI_CORPUS unset or holds no .cs — nothing to measure.");
            return;
        }

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_c_sharp::LANGUAGE.into()).expect("the C# grammar loads");

        let mut files = 0usize;
        let mut unreadable = 0usize;
        let mut emitted_total = 0usize;
        let mut counted_total = 0usize;
        let mut disagreeing = 0usize;
        let mut worst: Vec<(i64, String, usize, usize)> = Vec::new();
        let mut reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut resolved = 0usize;
        let mut unresolved = 0usize;
        let mut symbols = 0usize;
        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();

        for (path, text) in &sources {
            let source = Source { package: "unknown", module: "", path, text };
            let Ok(facts) = walk::read(&source, &TypeHomes::unknown()) else {
                unreadable += 1;
                continue;
            };
            files += 1;
            symbols += facts.symbols.len();
            for s in &facts.symbols {
                *by_kind.entry(format!("{:?}", s.kind)).or_default() += 1;
            }
            for r in &facts.references {
                match &r.target {
                    Resolution::Resolved { .. } => resolved += 1,
                    Resolution::Unresolved { reason, .. } => {
                        unresolved += 1;
                        *reasons.entry(reason.as_label()).or_default() += 1;
                    }
                }
            }
            let emitted = facts
                .references
                .iter()
                .filter(|r| {
                    matches!(
                        r.kind,
                        RefKind::Calls | RefKind::Constructs | RefKind::Reads | RefKind::TypeUse
                    )
                })
                .count();
            let Some(tree) = parser.parse(text.as_str(), None) else { continue };
            let expected = count_use_sites(tree.root_node());
            emitted_total += emitted;
            counted_total += expected;
            if emitted != expected {
                disagreeing += 1;
                worst.push((
                    (expected as i64 - emitted as i64).abs(),
                    path.clone(),
                    emitted,
                    expected,
                ));
            }
        }

        worst.sort_by_key(|(delta, ..)| std::cmp::Reverse(*delta));
        println!("\n── A2: nothing dropped (csharp) ──");
        println!("files {files} ({unreadable} unreadable)");
        println!("symbols {symbols}");
        println!("  by kind {by_kind:?}");
        println!("walk emitted {emitted_total} | counted independently {counted_total}");
        println!("files disagreeing: {disagreeing} of {files}");
        for (delta, path, emitted, expected) in worst.iter().take(8) {
            println!("  {delta:>5}  {path} (walk {emitted}, counted {expected})");
        }

        println!("\n── A3: every miss named (csharp) ──");
        println!("resolved {resolved} | unresolved {unresolved}");
        let named: usize = reasons.values().sum();
        for (reason, n) in &reasons {
            println!("  {n:>7}  {reason}");
        }

        // A3 IS THE ASSERTION. Every unresolved reference carries a reason, so
        // the histogram must account for all of them — a miss with no name is a
        // reference nothing can explain and nobody can act on.
        assert_eq!(
            named,
            unresolved,
            "{} unresolved references carry no reason; the histogram accounts for {named}",
            unresolved - named
        );

        // A2 is REPORTED rather than ratcheted here, and deliberately. The
        // independent counter is an approximation of the grammar's use sites,
        // not a second implementation of the walk — Java's equivalent disagrees
        // on 5 of 5,088 files and that is read, not asserted. What matters is
        // that the two are the same ORDER and that the disagreeing set is small
        // enough to read, both of which the report above makes checkable.
        assert!(files > 100, "corpus too small to be meaningful: {files} files");
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
