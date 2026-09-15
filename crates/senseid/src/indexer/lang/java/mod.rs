//! Java — identity rules and the grammar the ladder climbs.
//!
//! The walk is in [`walk`]; this file owns what a Java name MEANS, which is R7's
//! split.
//!
//! # In Java the package IS the namespace
//!
//! Rust has crates containing module trees; JavaScript has packages containing
//! file paths. Both need two segments — `<package>` and `<module>` — to say
//! where a declaration lives. Java has one. `package com.sg.dayamed.service;`
//! at the top of a file is the complete answer, it is written in the source
//! rather than inferred from the path, and two files in one package see each
//! other with no import at all.
//!
//! So a Java fqn carries the declared package in `<package>` and leaves
//! `<module>` EMPTY:
//!
//! Reading the segments in order: the language `java`, the package
//! `com.sg.dayamed.service`, an EMPTY module, the type `PatientService`, the
//! member `loadNursePractitioner`, and the reach `item`.
//!
//! This is not a shortcut. Deriving the module from the directory instead would
//! disagree with the source in every repository where the two drift — which is
//! legal Java, common in generated trees, and silent — and a declaration filed
//! under the directory while a reference is filed under the declared package is
//! two identities that never meet (spec §2).
//!
//! # Nothing here can tell first-party from library
//!
//! `com.sg.dayamed.service.PatientService` and
//! `org.springframework.http.ResponseEntity` are the same shape. Rust marks the
//! boundary with `crate::`, JavaScript with a leading `.` or `/`; Java marks it
//! with nothing, because a Java compiler answers the question from the
//! classpath and a source file never states it.
//!
//! So the walk calls EVERY import external and names the package it came from,
//! and the ladder flips the ones this scan owns — `Ladder::owned_by_this_scan`,
//! the rung that already exists for Rust's sibling crates. `World::first_party`
//! must therefore hold every Java package the scan declares, which is exactly
//! the set of `package` declarations in it. Precise, and no prefix is guessed:
//! a package we declare is ours and one we do not is somebody's library.

use std::sync::LazyLock;

use super::{LanguageAdapter, Source, TypeHomes};
use crate::indexer::facts::FileFacts;
use crate::indexer::facts::{Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach, Segment};
use crate::indexer::resolve::Grammar;

mod walk;

/// The names `java.lang` puts in every file with no import, as
/// `(name, package, path)`.
///
/// Not the whole of `java.lang` — the members a body actually names. A list
/// that tried to be complete would be a second, worse copy of the JDK, and
/// every name missing from it is a miss the histogram reports honestly
/// (`NoImportInScope`) rather than a wrong edge.
const PRELUDE: &[(&str, &str, &str)] = &[
    ("Object", "java.lang", "Object"),
    ("String", "java.lang", "String"),
    ("CharSequence", "java.lang", "CharSequence"),
    ("StringBuilder", "java.lang", "StringBuilder"),
    ("StringBuffer", "java.lang", "StringBuffer"),
    ("Integer", "java.lang", "Integer"),
    ("Long", "java.lang", "Long"),
    ("Short", "java.lang", "Short"),
    ("Byte", "java.lang", "Byte"),
    ("Double", "java.lang", "Double"),
    ("Float", "java.lang", "Float"),
    ("Boolean", "java.lang", "Boolean"),
    ("Character", "java.lang", "Character"),
    ("Number", "java.lang", "Number"),
    ("Math", "java.lang", "Math"),
    ("System", "java.lang", "System"),
    ("Thread", "java.lang", "Thread"),
    ("Runnable", "java.lang", "Runnable"),
    ("Class", "java.lang", "Class"),
    ("Enum", "java.lang", "Enum"),
    ("Iterable", "java.lang", "Iterable"),
    ("Comparable", "java.lang", "Comparable"),
    ("Throwable", "java.lang", "Throwable"),
    ("Exception", "java.lang", "Exception"),
    ("RuntimeException", "java.lang", "RuntimeException"),
    ("IllegalArgumentException", "java.lang", "IllegalArgumentException"),
    ("IllegalStateException", "java.lang", "IllegalStateException"),
    ("NullPointerException", "java.lang", "NullPointerException"),
    ("UnsupportedOperationException", "java.lang", "UnsupportedOperationException"),
    ("Error", "java.lang", "Error"),
    ("Override", "java.lang", "Override"),
    ("Deprecated", "java.lang", "Deprecated"),
    ("SuppressWarnings", "java.lang", "SuppressWarnings"),
    ("FunctionalInterface", "java.lang", "FunctionalInterface"),
    ("SafeVarargs", "java.lang", "SafeVarargs"),
    ("AutoCloseable", "java.lang", "AutoCloseable"),
    ("Void", "java.lang", "Void"),
];

/// Members so ubiquitous that tracking them buries every real dependency.
///
/// `Object`'s methods are on literally every receiver, and Java's collection
/// and stream vocabulary is nearly as universal. Filtered with its own
/// [`crate::indexer::facts::Reason`], so a reader can exclude them without also
/// excluding genuine misses.
const PLUMBING: &[&str] = &[
    // java.lang.Object — on every receiver there is.
    "toString",
    "equals",
    "hashCode",
    "getClass",
    "clone",
    "notify",
    "notifyAll",
    "wait",
    "finalize",
    // Collection and stream vocabulary, on everything that holds anything.
    "add",
    "addAll",
    "get",
    "put",
    "putAll",
    "remove",
    "contains",
    "containsKey",
    "containsValue",
    "size",
    "isEmpty",
    "clear",
    "iterator",
    "stream",
    "forEach",
    "map",
    "filter",
    "collect",
    "toList",
    "keySet",
    "values",
    "entrySet",
    "getKey",
    "getValue",
    // Optional, which Java code threads through everything.
    "of",
    "ofNullable",
    "orElse",
    "orElseGet",
    "orElseThrow",
    "isPresent",
    "ifPresent",
    // String, likewise.
    "length",
    "charAt",
    "substring",
    "trim",
    "split",
    "format",
    "valueOf",
    "concat",
    "startsWith",
    "endsWith",
    "indexOf",
    "toLowerCase",
    "toUpperCase",
    "compareTo",
    "matches",
    "replace",
    "replaceAll",
    "join",
];

/// A Java type name starts with an upper-case letter.
///
/// A convention rather than a rule, and the strongest one any of the three
/// languages has: the JLS does not enforce it, but it is universal in practice
/// and the whole ecosystem's tooling assumes it. A lower-case class would be
/// filed as a module segment, which is a miss and not a wrong edge.
fn names_a_type(segment: &str) -> bool {
    segment.chars().next().is_some_and(char::is_uppercase)
}

pub static GRAMMAR: LazyLock<Grammar> = LazyLock::new(|| Grammar {
    language: Language::Java,
    // One separator for both, because Java has one namespace. `java.util.List`
    // is a path and `com.sg.dayamed.service` is a module, spelled identically
    // because they are the same kind of thing.
    path_separator: ".",
    module_separator: ".",
    // EMPTY, and that is a fact about Java rather than an omission. There is no
    // `crate::`, no `super::`, no `./` — every import is an absolute name from
    // the root of the classpath, so no spelling roots a path anywhere but there.
    roots: &[],
    // Java has no `import x as y`. A name is imported under its own last
    // segment or not at all.
    names_the_binding: None,
    wildcard: Some("*"),
    paths_name_packages: true,
    relative_to_directory: false,
    names_a_type,
    prelude: PRELUDE,
    plumbing: PLUMBING,
});

/// The type a Java type expression NAMES.
///
/// A free function because the walk needs it too, and a second copy is how the
/// declaration side and the reference side drift apart.
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
    // A generic instantiation names its own type: `List<Patient>` is a use
    // of `List`. The parameter is a separate use site the walk emits
    // separately, so dropping it here loses nothing.
    let bare = raw.split_once('<').map_or(raw, |(head, _)| head);
    // An array names its element type.
    let bare = bare.trim_end_matches("[]").trim();
    // `java.util.List` at a use site names `List`; the path in front of it
    // is how it was reached, not what it is.
    let last = bare.rsplit('.').next().unwrap_or(bare).trim();
    if last.is_empty() {
        return Err(FqnError::EmptySegment { segment: Segment::Type });
    }
    Ok(last.to_string())
}

/// Java, from `.java`.
pub struct JavaAdapter;

impl LanguageAdapter for JavaAdapter {
    fn language(&self) -> Language {
        Language::Java
    }

    fn name(&self) -> &'static str {
        "java"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".java"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, super::ReadError> {
        walk::read(source, types)
    }

    /// A Java file is named by its primary type, which the language guarantees
    /// matches the file stem: a public class MUST live in a file of its own
    /// name. So the stem is not a guess about the contents, it is a rule the
    /// compiler enforces — unlike Rust, where a file's module name and the
    /// types in it are unrelated.
    fn file_fqn(&self, package: &str, _module: &str, path: &str) -> Result<Fqn, FqnError> {
        let stem = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".java");
        fqn::define(&Form::Item {
            lang: Language::Java,
            package,
            module: "",
            name: stem,
            reach: Reach::Mod,
        })
    }

    /// EMPTY, always.
    ///
    /// The module a Java file belongs to is the `package` it declares, which the
    /// walk reads and hands back as the package. Deriving a second one from the
    /// directory would produce a segment the source never wrote, and the two
    /// disagree wherever a tree is laid out loosely — which is legal, common in
    /// generated code, and silent.
    fn module_path(&self, _file: &str, _package_root: &str) -> String {
        String::new()
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        type_segment(raw)
    }

    /// A Java file's identity does not move when the file does.
    ///
    /// The package is declared IN the file, so renaming or relocating it mints
    /// nothing new — unlike Rust and JavaScript, where the path is the module.
    /// The one thing that re-mints an identity is editing the `package` line,
    /// which is a content change and reaches this function as neither argument.
    fn rename_remints_identity(&self, _from: &str, _to: &str, _package_root: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(raw: &str) -> String {
        JavaAdapter.type_segment(raw).expect("the fixture names a type")
    }

    #[test]
    fn a_type_use_is_named_by_its_own_last_segment() {
        assert_eq!(seg("PatientService"), "PatientService");
        assert_eq!(seg("java.util.List"), "List", "the path is how it was reached, not what it is");
        assert_eq!(seg("List<Patient>"), "List", "the parameter is its own use site");
        assert_eq!(seg("Map<String, List<Patient>>"), "Map");
        assert_eq!(seg("byte[]"), "byte");
        assert_eq!(seg("Patient[][]"), "Patient");
        assert_eq!(seg("com.sg.dayamed.vo.UserVO[]"), "UserVO");
        assert!(JavaAdapter.type_segment("").is_err(), "a name with nothing in it is not a type");
        assert!(JavaAdapter.type_segment("<>").is_err());
    }

    #[test]
    fn the_module_segment_is_empty_because_the_package_carries_everything() {
        assert_eq!(
            JavaAdapter.module_path("server/src/main/java/com/sg/dayamed/Svc.java", "server"),
            "",
            "the package is declared in the file; a second one from the path would be a \
             segment the source never wrote"
        );
    }

    /// The convention the whole module leans on, stated once as a test.
    #[test]
    fn a_leading_capital_is_what_names_a_type() {
        assert!(names_a_type("PatientService"));
        assert!(names_a_type("UserVO"));
        assert!(!names_a_type("com"));
        assert!(!names_a_type("dayamed"));
        assert!(!names_a_type("loadNursePractitioner"));
        assert!(!names_a_type(""), "nothing is not a type");
    }

    #[test]
    fn moving_a_file_does_not_remint_its_identity() {
        assert!(
            !JavaAdapter.rename_remints_identity(
                "server/src/main/java/com/sg/dayamed/A.java",
                "server/src/main/java/com/sg/dayamed/B.java",
                "server",
            ),
            "the package line is what names a Java declaration, and a rename does not touch it"
        );
    }

    #[test]
    fn the_grammar_states_java_has_no_path_roots() {
        assert!(
            GRAMMAR.roots.is_empty(),
            "no `crate::`, no `super::`, no `./` — every Java import is absolute"
        );
        assert_eq!(GRAMMAR.wildcard, Some("*"));
        assert_eq!(GRAMMAR.names_the_binding, None, "Java has no `import x as y`");
        assert_eq!(GRAMMAR.path_separator, GRAMMAR.module_separator, "one namespace, one dot");
    }

    #[test]
    fn the_prelude_and_the_plumbing_hold_no_duplicates() {
        let mut names: Vec<&str> = PRELUDE.iter().map(|(n, _, _)| *n).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "a name listed twice in the prelude");

        let mut plumbing = PLUMBING.to_vec();
        plumbing.sort_unstable();
        let before = plumbing.len();
        plumbing.dedup();
        assert_eq!(before, plumbing.len(), "a name listed twice in the plumbing");
    }
}

#[cfg(test)]
mod corpus {
    //! A2 for Java, over a codebase nobody here wrote.
    //!
    //! There is no Java in this repository, so there is no in-repo corpus to
    //! measure against — and that is the right constraint rather than a
    //! limitation. The foreign-corpus finding was that agreement with code
    //! written alongside the indexer is weak evidence; Java has none of that
    //! problem because every line of it is somebody else's.
    //!
    //!     SENSEI_CORPUS=~/Work/Dayamed cargo test -p senseid --bin senseid -- \
    //!       --ignored --nocapture indexer::lang::java::corpus

    use std::collections::BTreeMap;

    use tree_sitter::Node;

    use super::*;
    use crate::indexer::facts::{RefKind, RelationKind, Resolution, SymbolKind};
    use crate::indexer::lang::{Source, TypeHomes};

    /// Every `.java` file under `SENSEI_CORPUS`, and whether it looks generated.
    ///
    /// The generated ones are counted SEPARATELY, never dropped. 2,922 of
    /// Dayamed's 8,013 files are JAXB output — 36% — so folding them in would
    /// make every number a property of a code generator rather than of the
    /// reader, and dropping them would hide files a user's scan will really
    /// meet.
    fn sources() -> Vec<(String, String, bool)> {
        let Ok(root) = std::env::var("SENSEI_CORPUS") else { return Vec::new() };
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| e.file_name() != "build" && e.file_name() != "target")
        {
            let Ok(entry) = entry else { continue };
            if entry.path().extension().is_none_or(|e| e != "java") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
            // `@Generated` as a SUBSTRING matches `@GeneratedValue`, which is
            // on the id column of every JPA entity there is. That excluded
            // every entity class from the corpus while other files went on
            // resolving TO them — 41,990 edges naming a declaration the harness
            // had quietly dropped, headed by `Patient`, `Prescription` and
            // `UserDetails`. The annotation, not a prefix of it.
            let generated = text.contains("This file was generated by the JavaTM Architecture")
                || text.contains("@Generated(")
                || text.contains("@Generated\n")
                || text.contains("@Generated ");
            out.push((entry.path().display().to_string(), text, generated));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn parse(text: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_java::LANGUAGE.into()).expect("the java grammar loads");
        parser.parse(text, None).expect("a parse produces a tree")
    }

    /// The four fields whose `type` the walk reads as a use site. Structural
    /// uses — `extends`, `implements`, an annotation — are RELATIONS, not
    /// references, so they are deliberately not here.
    const TYPED: &[&str] = &[
        "method_declaration",
        "field_declaration",
        "formal_parameter",
        "spread_parameter",
        "local_variable_declaration",
        "annotation_type_element_declaration",
    ];

    /// Use sites, counted with no knowledge of `Walk` and none of resolution.
    fn count_use_sites(root: Node<'_>) -> BTreeMap<&'static str, usize> {
        let mut out: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            match node.kind() {
                "method_invocation" => *out.entry("calls").or_default() += 1,
                "object_creation_expression" => *out.entry("constructs").or_default() += 1,
                "field_access" => *out.entry("reads").or_default() += 1,
                kind if TYPED.contains(&kind) => {
                    if let Some(ty) = node.child_by_field_name("type") {
                        *out.entry("type_use").or_default() += type_names_under(ty);
                    }
                }
                _ => {}
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        out
    }

    fn type_names_under(node: Node<'_>) -> usize {
        let mut count = 0;
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if matches!(n.kind(), "type_identifier" | "scoped_type_identifier") {
                count += 1;
                continue;
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
        count
    }

    fn walked(counts: &BTreeMap<&'static str, usize>) -> usize {
        counts.values().sum()
    }

    /// **A2 for Java.** Every use site the grammar has, counted from the other
    /// side, against what the walk emitted.
    #[test]
    #[ignore]
    fn the_reference_count_equals_an_independent_count_of_use_sites() {
        let sources = sources();
        if sources.is_empty() {
            println!("SENSEI_CORPUS unset or holds no .java — nothing to measure.");
            return;
        }

        let mut files = 0usize;
        let mut generated_files = 0usize;
        let mut unreadable = Vec::new();
        let mut walk_total = 0usize;
        let mut counted_total = 0usize;
        let mut worst: Vec<(i64, String, usize, usize)> = Vec::new();
        let mut reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut symbols = 0usize;
        let mut relations = 0usize;
        let mut owned_members = 0usize;
        let mut owns = 0usize;

        for (path, text, generated) in &sources {
            if *generated {
                generated_files += 1;
                continue;
            }
            files += 1;
            let source = Source { package: "unknown", module: "", path, text };
            let facts = match walk::read(&source, &TypeHomes::unknown()) {
                Ok(facts) => facts,
                Err(e) => {
                    unreadable.push(format!("{path}: {e:?}"));
                    continue;
                }
            };
            symbols += facts.symbols.len();
            relations += facts.relations.len();
            owned_members += facts
                .symbols
                .iter()
                .filter(|s| {
                    matches!(
                        s.kind,
                        SymbolKind::Method | SymbolKind::Field | SymbolKind::EnumVariant
                    )
                })
                .count();
            owns += facts.relations.iter().filter(|r| r.kind == RelationKind::Owns).count();
            for r in &facts.references {
                if let Resolution::Unresolved { reason, .. } = &r.target {
                    *reasons.entry(reason.as_label()).or_default() += 1;
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
            let expected = walked(&count_use_sites(parse(text).root_node()));
            walk_total += emitted;
            counted_total += expected;
            if emitted != expected {
                worst.push((
                    (expected as i64 - emitted as i64).abs(),
                    path.clone(),
                    emitted,
                    expected,
                ));
            }
        }

        worst.sort_by_key(|(d, _, _, _)| std::cmp::Reverse(*d));
        println!(
            "\n## A2 — Java, over {files} hand-written files ({generated_files} generated, counted separately)\n"
        );
        println!("unreadable {} | symbols {symbols} | relations {relations}", unreadable.len());
        println!(
            "walk {walk_total} | independent {counted_total} | delta {}",
            counted_total as i64 - walk_total as i64
        );
        println!("files disagreeing: {} of {files}", worst.len());
        for (_, path, emitted, expected) in worst.iter().take(10) {
            let short = path.rsplit('/').take(2).collect::<Vec<_>>().join("/");
            println!("  {short}: walk {emitted}, independent {expected}");
        }
        println!("\nmisses by reason:");
        for (reason, n) in &reasons {
            println!("  {n:>8}  {reason}");
        }
        for u in unreadable.iter().take(5) {
            println!("  UNREADABLE {u}");
        }

        assert!(files > 0, "no hand-written .java under SENSEI_CORPUS");

        // Every member must say WHICH TYPE owns it. The cross-language guard in
        // `acceptance` cannot check Java — this repository holds no Java — so
        // the assertion has to live where the Java is.
        //
        // `RelationKind::Owns` is the vocabulary a member rung needs, and no
        // `SymbolKind` filter substitutes for it: which kinds are type-owned
        // differs per language, which is what made `World::first_party_members`
        // unusable as an inventory and cost Rust 2,112 edges when it was tried.
        println!("\ntype-owned declarations {owned_members} | Owns relations {owns}");
        assert!(
            owns > 0,
            "Java declares {owned_members} type-owned members and emits NO Owns relation, so \
             the graph cannot say which type owns any of them"
        );

        assert!(unreadable.is_empty(), "a file the grammar could not read is a fact to act on");

        // RATCHETED, not zero. §6's first line: a gate that fails on its first
        // run gets waived, and these are the numbers the walk actually reaches
        // on 4,675 files of somebody else's Java. Four passes took it from 436:
        // enum-constant arguments, annotation arguments, a field's annotations
        // walked twice, and an annotation element's `default` clause.
        //
        // The bound is on the CORPUS, so it only means anything when one is
        // pointed at. Tighten it when a pass closes more; never loosen it
        // without saying what shape was found.
        if files > 4_000 {
            assert!(
                counted_total.saturating_sub(walk_total) <= 9,
                "the walk is now missing {} use sites, up from the 9 this was measured at",
                counted_total.saturating_sub(walk_total)
            );
            assert!(
                walk_total <= counted_total,
                "the walk emitted MORE than an independent count — it is inventing references, \
                 which is worse than dropping them and is how a field's annotations came to be \
                 walked twice"
            );
            assert!(worst.len() <= 5, "{} files disagree, up from the 5 measured", worst.len());
        }
    }

    /// The ladder over Java, and the first real Java resolve rate.
    ///
    /// The BARRIER is the whole point: every file is read once with no type
    /// table so the declarations can be collected, then read again with it, so
    /// a member's identity does not depend on which file came first (R6).
    ///
    /// `first_party` is every package the scan DECLARES. That is what flips an
    /// import from library to local — a Java source file states nothing about
    /// which side of the boundary a name is on, so `owned_by_this_scan` answers
    /// it from the manifest of declarations rather than from a prefix guess.
    #[test]
    #[ignore]
    fn the_ladder_places_what_this_corpus_declares() {
        use std::collections::BTreeSet;

        use crate::indexer::facts::{FileFacts, SymbolKind};
        use crate::indexer::resolve::{World, resolve};

        let sources = sources();
        if sources.is_empty() {
            println!("SENSEI_CORPUS unset or holds no .java — nothing to measure.");
            return;
        }
        let hand_written: Vec<&(String, String, bool)> =
            sources.iter().filter(|(_, _, generated)| !generated).collect();

        let read_all = |types: &TypeHomes| -> Vec<FileFacts> {
            hand_written
                .iter()
                .filter_map(|(path, text, _)| {
                    // The package is READ from the source, so what is handed in
                    // is only the default-package fallback.
                    let source = Source { package: "unnamed", module: "", path, text };
                    walk::read(&source, types).ok()
                })
                .collect()
        };

        let first = read_all(&TypeHomes::unknown());
        let homes = TypeHomes::of(
            first.iter().flat_map(|f| f.symbols.iter().map(|s| (f.package.as_str(), s))),
        );
        // Every package this scan declares. In Java that IS the namespace, so
        // the set is exact rather than a prefix.
        let first_party: BTreeSet<String> = first.iter().map(|f| f.package.clone()).collect();
        let first_party_members: BTreeSet<String> = first
            .iter()
            .flat_map(|f| f.symbols.iter())
            .filter(|s| matches!(s.kind, SymbolKind::Method | SymbolKind::Field))
            .map(|s| s.name.clone())
            .collect();
        let scanned = BTreeSet::new();
        let world = World {
            first_party: &first_party,
            first_party_members: &first_party_members,
            scanned: &scanned,
        };

        let mut reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut rungs: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut resolved = 0usize;
        let mut total = 0usize;
        let mut local_imports = 0usize;
        let mut library_imports = 0usize;
        let mut nodes = 0usize;
        let mut placed_relations = 0usize;
        for facts in read_all(&homes) {
            let placed = resolve(facts, &GRAMMAR, &world);
            nodes += placed.symbols.len();
            placed_relations += placed
                .relations
                .iter()
                .filter(|r| matches!(r.parent, Resolution::Resolved { .. }))
                .count();
            // What the WALK said, which for Java is always external — it is
            // the ladder that flips the ones this scan owns, per reference, and
            // it does not rewrite the Import fact. Counting these as "the
            // classification" would report 0 first-party imports for a corpus
            // whose own packages account for most of them.
            library_imports += placed.imports.len();
            local_imports += placed
                .references
                .iter()
                .filter(|r| match &r.target {
                    Resolution::Resolved { fqn, .. } => !fqn.as_str().starts_with("lib"),
                    Resolution::Unresolved { .. } => false,
                })
                .count();
            for r in &placed.references {
                total += 1;
                match &r.target {
                    Resolution::Resolved { via, .. } => {
                        resolved += 1;
                        *rungs.entry(via.as_label()).or_default() += 1;
                    }
                    Resolution::Unresolved { reason, .. } => {
                        *reasons.entry(reason.as_label()).or_default() += 1;
                    }
                }
            }
        }

        println!(
            "\n## Java, resolved — {} files, {} packages\n",
            hand_written.len(),
            first_party.len()
        );
        println!(
            "references {total} | RESOLVED {resolved} ({:.1}%)",
            100.0 * resolved as f64 / total.max(1) as f64
        );
        println!(
            "import statements {library_imports} | placed edges that stayed first-party {local_imports}\n"
        );
        // The same ratio `acceptance::report` prints for the other two. An EDGE
        // is a fact with both ends known; an unresolved reference is a row with
        // a reason and no target, so counting it would make a graph look richer
        // the worse it resolved.
        println!(
            "resolved edges {} | nodes {nodes} | edges per node {:.2}\n",
            resolved + placed_relations,
            (resolved + placed_relations) as f64 / nodes.max(1) as f64
        );

        println!("by rung:");
        for (rung, n) in &rungs {
            println!("  {n:>8}  {rung}");
        }
        println!("\nby reason:");
        let mut head: BTreeMap<String, usize> = BTreeMap::new();
        for facts in read_all(&homes) {
            for r in &resolve(facts, &GRAMMAR, &world).references {
                if let Resolution::Unresolved { reason, evidence } = &r.target
                    && reason.as_label() == "no_import_in_scope"
                {
                    *head.entry(evidence.name.clone()).or_default() += 1;
                }
            }
        }
        let mut ranked_head: Vec<_> = head.iter().collect();
        ranked_head.sort_by_key(|(n, c)| (std::cmp::Reverse(**c), *n));
        println!("  no_import_in_scope head: {:?}\n", &ranked_head[..ranked_head.len().min(14)]);
        let mut ranked: Vec<_> = reasons.iter().collect();
        ranked.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (reason, n) in ranked {
            println!("  {n:>8}  {reason}");
        }

        // THE CHECK THAT CAUGHT THE RUST VERSION OF THIS. Handing the ladder a
        // qualified path is only safe if an unbound one falls through to a miss
        // rather than being placed first-party by another rung. A first-party
        // identity no declaration mints is that failure, visible.
        let minted: BTreeSet<String> = read_all(&homes)
            .iter()
            .flat_map(|f| f.symbols.iter())
            .map(|s| s.fqn.as_str().to_string())
            .collect();
        let mut dangling: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_rung: BTreeMap<&'static str, usize> = BTreeMap::new();
        for facts in read_all(&homes) {
            for r in &resolve(facts, &GRAMMAR, &world).references {
                if let Resolution::Resolved { fqn, via } = &r.target
                    && !fqn.as_str().starts_with("lib")
                    && !minted.contains(fqn.as_str())
                {
                    *dangling.entry(fqn.as_str().to_string()).or_default() += 1;
                    *by_kind.entry(format!("{:?}", r.kind)).or_default() += 1;
                    *by_rung.entry(via.as_label()).or_default() += 1;
                }
            }
        }
        let dangling_total: usize = dangling.values().sum();
        let mut worst_dangling: Vec<_> = dangling.iter().collect();
        worst_dangling.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!(
            "\nfirst-party edges naming no declaration: {dangling_total} across {} identities",
            dangling.len()
        );
        for (fqn, n) in worst_dangling.iter().take(6) {
            println!("  {n:>6}  {fqn}");
        }

        // Edges naming a member that is not in the source. Two guesses were
        // checked and only the second held.
        //
        // WRONG: that the two sides spell the empty module differently. They do
        // not — both produce `[java, com.sg.dayamed.entity, Patient, item]`,
        // compared segment by segment.
        //
        // RIGHT, and it was this harness's own bug: `@Generated` as a SUBSTRING
        // matches `@GeneratedValue`, so every JPA entity was dropped from the
        // corpus while other files went on resolving to it. 41,990 -> 26,995,
        // and 413 files came back.
        //
        // WHAT IS LEFT IS MOSTLY REAL. The head is now members rather than
        // types: `Patient.getUserDetails` is a Lombok `@Getter`, generated at
        // compile time and in no source file; `UserDetailsRepository.findById`
        // is inherited from Spring Data's `JpaRepository`. Both are targets that
        // exist only after something else runs, which is what
        // `Reason::MacroExpansion` names — and the ladder currently mints a
        // first-party identity for them instead, which is a wrong edge (R4).
        println!(
            "\nKNOWN BROKEN: {dangling_total} first-party edges name a member no source \
             declares — chiefly Lombok accessors and inherited Spring Data methods."
        );
        println!("  by ref kind: {by_kind:?}");
        println!("  by rung:     {by_rung:?}");

        assert_eq!(
            resolved + reasons.values().sum::<usize>(),
            total,
            "every reference is resolved or carries a reason (A3)"
        );
        assert!(
            reasons.get("unplaced").is_none_or(|n| *n == 0),
            "{} references reached no verdict — the ladder returned without answering",
            reasons.get("unplaced").copied().unwrap_or(0)
        );
    }
}
