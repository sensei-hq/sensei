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
    // An import names a class on the classpath, never a file, so there is no
    // extension in it — and Java's separator is the dot a stem rule would cut.
    module_segment: crate::indexer::lang::common::already_a_module_segment,
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

    /// **An interface's supertypes are inheritance too, and the grammar does
    /// not put them in a field.**
    ///
    /// `class_declaration` carries `superclass` and `interfaces` as named
    /// fields, so `child_by_field_name` finds them. `interface_declaration`
    /// carries neither: its supertypes sit in an `extends_interfaces` CHILD
    /// with no field name at all, so a reader that only asks for fields sees an
    /// interface as having no parents.
    ///
    /// It is not a corner. Every Spring Data repository in a Java codebase is
    /// `interface XRepository extends JpaRepository<..>`, and with the
    /// inheritance missing there is no way to tell an inherited `findById` from
    /// a method nothing declares — which is exactly how 2,756 dangling edges in
    /// the Dayamed corpus came to be labelled "unexplained".
    #[test]
    fn an_interface_states_its_supertypes_where_no_field_name_reaches() {
        use crate::indexer::facts::{RelationKind, Resolution};

        let text = "package p;\n\
                    public interface UserRepository extends JpaRepository<User, Long>, Audited {\n\
                    \x20 User findByName(String name);\n\
                    }\n";
        let facts = walk::read(
            &Source { package: "unnamed", module: "", path: "src/main/java/p/R.java", text },
            &TypeHomes::unknown(),
        )
        .expect("the fixture parses");

        let parents: Vec<String> = facts
            .relations
            .iter()
            .filter(|r| matches!(r.kind, RelationKind::Extends | RelationKind::Implements))
            .map(|r| match &r.parent {
                Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                Resolution::Unresolved { evidence, .. } => evidence.name.clone(),
            })
            .collect();
        assert_eq!(
            parents,
            vec!["JpaRepository".to_string(), "Audited".to_string()],
            "an interface extends a LIST, in source order, and the type argument is not one \
             of them"
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

    /// One hand-written file of the corpus, walked, anchored and PLACED.
    pub(super) struct Placed {
        pub(super) path: String,
        pub(super) text: String,
        pub(super) facts: crate::indexer::facts::FileFacts,
    }

    /// The whole hand-written corpus, read twice and placed ONCE.
    ///
    /// The BARRIER is the whole point: every file is read with no type table so
    /// the declarations can be collected, then read again with it, so a
    /// member's identity does not depend on which file came first (R6).
    ///
    /// `first_party` is every package the scan DECLARES. That is what flips an
    /// import from library to local — a Java source file states nothing about
    /// which side of the boundary a name is on, so the answer comes from the
    /// manifest of declarations rather than from a prefix guess.
    ///
    /// Shared, and resolving ONCE, because two measurements now read this
    /// corpus and the ladder test used to run the whole placement four times
    /// over to print four sections of one report. `resolve` is pure, so four
    /// runs could only ever produce the same facts at four times the cost.
    fn placed_corpus() -> Vec<Placed> {
        use std::collections::BTreeSet;

        use crate::indexer::facts::FileFacts;
        use crate::indexer::resolve::{World, resolve};

        let sources = sources();
        let hand_written: Vec<&(String, String, bool)> =
            sources.iter().filter(|(_, _, generated)| !generated).collect();
        if hand_written.is_empty() {
            return Vec::new();
        }

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
        // The second pass, COMPLETE, before anything is placed — every barrier
        // artifact below comes off it. `declared_members` in particular cannot
        // come off `first`: a member's identity carries its TYPE's module, so
        // the pre-barrier pass spells those members differently.
        let anchored = read_all(&homes);
        // Every package this scan declares. In Java that IS the namespace, so
        // the set is exact rather than a prefix.
        let first_party: BTreeSet<String> = anchored.iter().map(|f| f.package.clone()).collect();
        let first_party_members = crate::indexer::resolve::member_names_of(anchored.iter());
        let declared_members = crate::indexer::resolve::members_declared_by(anchored.iter());
        let supplied_members = crate::indexer::resolve::SuppliedMembers::of(anchored.iter());
        let returns = crate::indexer::resolve::returns_declared_by(anchored.iter());
        let scanned = BTreeSet::new();
        let world = World {
            first_party: &first_party,
            first_party_members: &first_party_members,
            declared_members: &declared_members,
            supplied_members: &supplied_members,
            returns: &returns,
            scanned: &scanned,
        };

        // Joined back onto the source it came from BY PATH, not by index:
        // `read_all` skips a file the grammar rejects, so the two lists need
        // not line up and a positional zip would slide every text one file
        // along from the facts it belongs to.
        let text_of: BTreeMap<&str, &str> =
            hand_written.iter().map(|(path, text, _)| (path.as_str(), text.as_str())).collect();
        anchored
            .into_iter()
            .map(|facts| {
                let text = text_of
                    .get(facts.path.as_str())
                    .unwrap_or_else(|| {
                        panic!("{} came out of the walk, not the corpus", facts.path)
                    })
                    .to_string();
                Placed { path: facts.path.clone(), text, facts: resolve(facts, &GRAMMAR, &world) }
            })
            .collect()
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
    /// The corpus itself — the two-pass barrier and the placement — is
    /// [`placed_corpus`]; what is here is the report and its gates.
    #[test]
    #[ignore]
    fn the_ladder_places_what_this_corpus_declares() {
        use std::collections::BTreeSet;

        let corpus = placed_corpus();
        if corpus.is_empty() {
            println!("SENSEI_CORPUS unset or holds no .java — nothing to measure.");
            return;
        }

        let mut reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut rungs: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut resolved = 0usize;
        let mut total = 0usize;
        let mut local_imports = 0usize;
        let mut library_imports = 0usize;
        let mut nodes = 0usize;
        let mut placed_relations = 0usize;
        let mut packages: BTreeSet<&str> = BTreeSet::new();
        let mut head: BTreeMap<&str, usize> = BTreeMap::new();
        for Placed { facts: placed, .. } in &corpus {
            packages.insert(placed.package.as_str());
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
                    Resolution::Unresolved { reason, evidence } => {
                        *reasons.entry(reason.as_label()).or_default() += 1;
                        if reason.as_label() == "no_import_in_scope" {
                            *head.entry(evidence.name.as_str()).or_default() += 1;
                        }
                    }
                }
            }
        }

        println!("\n## Java, resolved — {} files, {} packages\n", corpus.len(), packages.len());
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
        let mut ranked_head: Vec<_> = head.iter().collect();
        ranked_head.sort_by_key(|(n, c)| (std::cmp::Reverse(**c), *n));
        println!("  no_import_in_scope head: {:?}\n", &ranked_head[..ranked_head.len().min(14)]);
        let mut ranked: Vec<_> = reasons.iter().collect();
        ranked.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (reason, n) in ranked {
            println!("  {n:>8}  {reason}");
        }

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

    /// **The two barriers, over a corpus nobody here wrote.**
    ///
    /// The same measurement `acceptance` runs over this repository's Rust and
    /// TypeScript, pointed at Java — see [`crate::indexer::barrier`] for what
    /// the two barriers are and why the measurement is shared rather than
    /// copied.
    ///
    /// Java is the sharpest of the three corpora for this. Its tests are 871
    /// JUnit files under `src/test/java` written by somebody else years before
    /// this indexer existed, so a test edge here is evidence about the reader
    /// and not about a convention the reader's authors also wrote.
    ///
    ///     SENSEI_CORPUS=~/Work/Dayamed cargo test -p senseid --bin senseid -- \
    ///       --ignored --nocapture indexer::lang::java::corpus::the_two_barriers
    #[test]
    #[ignore]
    fn the_two_barriers_over_a_corpus_nobody_here_wrote() {
        use crate::indexer::barrier::{Unit, two_barriers};

        let corpus = placed_corpus();
        if corpus.is_empty() {
            println!("SENSEI_CORPUS unset or holds no .java — nothing to measure.");
            return;
        }
        let units: Vec<Unit<'_>> = corpus
            .iter()
            .map(|p| Unit { path: p.path.as_str(), text: p.text.as_str(), facts: &p.facts })
            .collect();
        let per = two_barriers(&units);
        let java = per.get("java").expect("a java corpus produced java nodes");

        // RATCHETS, measured, in the direction that can only improve. Shares
        // rather than counts where the population moves: the corpus is a
        // checkout that a person may add a module to, and a count floor would
        // fail on growth or pass on regression depending on which moved faster.
        let share = |n: usize| 100.0 * n as f64 / java.nodes as f64;
        assert!(java.nodes > 10_000, "{} java source nodes is not this corpus", java.nodes);
        // The CATEGORICAL check first, and it is the one that mattered for
        // TypeScript: a language whose tests reach nothing at all is a reader
        // defect, never a property of the code. 871 JUnit files cannot name
        // nought.
        assert!(
            java.no_test < java.nodes,
            "not one java source node is named by a test, and {} JUnit files are in the \
             corpus — that is the reader, not the code",
            units.iter().filter(|u| u.path.contains("/src/test/")).count()
        );
        // MEASURED over 5,088 hand-written files: 23,593 nodes, 14,854 (63.0%)
        // with no direct test edge, 9,478 (40.2%) transitively exercised,
        // 14,561 (61.7%) with no source edge, 11,395 (48.3%) reached by
        // nothing at all. A point of slack on each, because the corpus is a
        // checkout somebody may add a module to.
        assert!(
            share(java.no_test) <= 64.0,
            "{:.1}% of java source nodes have no direct test edge, up from the 63.0% measured",
            share(java.no_test)
        );
        assert!(
            share(java.neither) <= 49.0,
            "{:.1}% of java source nodes are reached by NOTHING, up from the 48.3% measured",
            share(java.neither)
        );
    }

    /// **The 26,995 first-party edges that name a member no source declares,
    /// SPLIT.**
    ///
    /// The number was recorded as "chiefly Lombok accessors and inherited
    /// Spring Data methods" and that reading was never checked. A count nobody
    /// has decomposed is a count nobody can act on, and the head of this one —
    /// `ApplicationConstants.SUCCESS` at 456 — is neither Lombok nor Spring
    /// Data: it is a `public static final String` sitting in plain sight in
    /// `util/ApplicationConstants.java`.
    ///
    /// Every bucket below is computed from the facts, in this order, so a
    /// reference lands in exactly one and the first match wins:
    ///
    /// 1. **the declaration is there, under a different REACH.** The walk mints
    ///    a field declaration at [`Reach::Field`] and `refer_to_member` mints
    ///    every member use site at [`Reach::Item`], so a first-party field READ
    ///    can never meet its own declaration. Checked by minting the `field`
    ///    form of the same identity through `fqn::refer` and asking whether the
    ///    scan declares THAT.
    /// 2. **a first-party SUPERTYPE declares it.** `extends`/`implements` are
    ///    recorded, and a member reached through a subtype names the subtype.
    /// 3. **Lombok.** The owning type carries `@Getter`/`@Data`/`@Builder`/…
    ///    and the member is one of the accessors that annotation synthesises.
    ///    Generated at compile time, in no source file.
    /// 4. **Spring Data.** The owning type is an interface extending one of
    ///    `JpaRepository` and friends, and the method comes from there.
    /// 5. **the owning type declares no members at all here.** An INSTRUMENT
    ///    gap rather than a corpus fact — an interface's constants are
    ///    `constant_declaration` in the grammar and the walk reads only
    ///    `field_declaration`.
    /// 6. **the owning type is not declared by this scan.**
    /// 7. the remainder, with its head printed.
    ///
    /// ```text
    /// SENSEI_CORPUS=~/Work/Dayamed cargo test -p senseid --bin senseid -- \
    ///   --ignored --nocapture indexer::lang::java::corpus::the_first_party_edges
    /// ```
    #[test]
    #[ignore]
    fn the_first_party_edges_that_name_no_declaration_are_a_measured_and_split_set() {
        use std::collections::BTreeSet;

        use crate::indexer::facts::{RelationKind, SymbolKind};
        use crate::indexer::fqn::{self, Form, Reach};

        let corpus = placed_corpus();
        if corpus.is_empty() {
            println!("SENSEI_CORPUS unset or holds no .java — nothing to measure.");
            return;
        }

        let declared: BTreeSet<&str> =
            corpus.iter().flat_map(|p| p.facts.symbols.iter().map(|s| s.fqn.as_str())).collect();
        // Two indexes read off the DECLARATIONS, both keyed by the type's
        // SIMPLE NAME rather than by its identity, because a supertype is
        // written as a bare name and may live in any package of the scan.
        // Keyed by identity, every supertype in another package reads as
        // unknown and the inheritance buckets collapse into the remainder.
        let mut declares_member: BTreeSet<(&str, &str)> = BTreeSet::new();
        let mut kind_of: BTreeMap<&str, SymbolKind> = BTreeMap::new();
        for Placed { facts, .. } in &corpus {
            for symbol in &facts.symbols {
                let Ok(parsed) = fqn::parse(symbol.fqn.as_str()) else { continue };
                match parsed.tail.as_slice() {
                    [ty, member] => {
                        declares_member.insert((ty, member));
                    }
                    [name] => {
                        kind_of.insert(name, symbol.kind);
                    }
                    _ => {}
                }
            }
        }
        // Which types own a member at all, and which annotations and
        // supertypes each type carries. All three are read off RELATIONS, which
        // is the vocabulary that states them — no `SymbolKind` filter
        // substitutes, because which kinds are type-owned differs per language.
        let mut owns_something: BTreeSet<&str> = BTreeSet::new();
        let mut decorated: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        let mut supertypes: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for Placed { facts, .. } in &corpus {
            for relation in &facts.relations {
                // The other end's NAME, wherever it ended up: a resolved parent
                // carries it as the segment before the reach, an unresolved one
                // carries it as the evidence the walk wrote down. Reading only
                // the resolved side would lose every annotation whose package
                // the file did not import.
                let named = match &relation.parent {
                    Resolution::Resolved { fqn, .. } => {
                        fqn::parse(fqn.as_str()).ok().and_then(|p| p.tail.last().copied())
                    }
                    Resolution::Unresolved { evidence, .. } => Some(evidence.name.as_str()),
                };
                match relation.kind {
                    RelationKind::Owns => {
                        if let Resolution::Resolved { fqn, .. } = &relation.parent {
                            owns_something.insert(fqn.as_str());
                        }
                    }
                    RelationKind::Decorates => {
                        if let Some(name) = named {
                            decorated.entry(relation.child.as_str()).or_default().insert(name);
                        }
                    }
                    RelationKind::Extends | RelationKind::Implements => {
                        if let Some(name) = named {
                            supertypes.entry(relation.child.as_str()).or_default().insert(name);
                        }
                    }
                    _ => {}
                }
            }
        }

        /// What Lombok writes for you. `@Data` is the umbrella and implies most
        /// of the others.
        const LOMBOK: &[&str] = &[
            "Getter",
            "Setter",
            "Data",
            "Value",
            "Builder",
            "SuperBuilder",
            "ToString",
            "EqualsAndHashCode",
            "AllArgsConstructor",
            "NoArgsConstructor",
            "RequiredArgsConstructor",
            "Accessors",
        ];
        /// The members those annotations synthesise, beyond `get*`/`set*`/`is*`.
        const SYNTHESISED: &[&str] =
            &["builder", "toBuilder", "toString", "equals", "hashCode", "canEqual"];
        /// What javac writes onto every enum declaration, in no source file.
        const ENUM_SYNTHESISED: &[&str] =
            &["values", "valueOf", "ordinal", "name", "compareTo", "getDeclaringClass"];
        /// Spring Data's repository supertypes: every method on one of these is
        /// inherited, and the interface that extends it declares none of them.
        const SPRING_DATA: &[&str] = &[
            "JpaRepository",
            "CrudRepository",
            "PagingAndSortingRepository",
            "MongoRepository",
            "JpaSpecificationExecutor",
            "Repository",
            "ReactiveCrudRepository",
        ];

        let mut buckets: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut remainder: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_rung: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        let mut identities: BTreeSet<String> = BTreeSet::new();
        let mut total = 0usize;
        // A sample per bucket, so a reading can be checked against the source
        // rather than believed.
        let mut sample: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();

        for Placed { facts, .. } in &corpus {
            for reference in &facts.references {
                let Resolution::Resolved { fqn, via } = &reference.target else { continue };
                let Ok(parsed) = fqn::parse(fqn.as_str()) else { continue };
                if parsed.origin == fqn::Origin::Lib || declared.contains(fqn.as_str()) {
                    continue;
                }
                total += 1;
                identities.insert(fqn.as_str().to_string());
                *by_rung.entry(via.as_label()).or_default() += 1;
                *by_kind.entry(format!("{:?}", reference.kind)).or_default() += 1;

                // A one-segment tail names a TYPE, not a member of one.
                let [ty, member] = parsed.tail.as_slice() else {
                    let bucket = "names a TYPE this scan does not declare";
                    *buckets.entry(bucket).or_default() += 1;
                    sample.entry(bucket).or_default().push(parsed.tail.join(" of "));
                    continue;
                };
                // The SAME identity at the field reach. Minted through the
                // grammar rather than spelled, so the two sides cannot disagree
                // about how a member is encoded.
                let as_a_field = fqn::refer(&Form::Member {
                    lang: Language::Java,
                    package: parsed.package,
                    module: "",
                    ty,
                    member,
                    reach: Reach::Field,
                })
                .map(|f| declared.contains(f.as_str()))
                .unwrap_or(false);
                let owner = fqn::refer(&Form::Item {
                    lang: Language::Java,
                    package: parsed.package,
                    module: "",
                    name: ty,
                    reach: Reach::Item,
                });
                let owner = owner.as_ref().map(|f| f.as_str()).unwrap_or("");
                let annotations = decorated.get(owner);
                let lombok_on_the_type =
                    annotations.is_some_and(|a| a.iter().any(|n| LOMBOK.contains(n)));
                let an_accessor = member.starts_with("get")
                    || member.starts_with("set")
                    || member.starts_with("is")
                    || SYNTHESISED.contains(member);
                let parents = supertypes.get(owner);
                // A supertype of OURS that declares this member. ONE level,
                // stated rather than implied: a deeper chain is a closure, and
                // this is a split rather than a resolver.
                let a_parent_declares_it = parents
                    .is_some_and(|ns| ns.iter().any(|n| declares_member.contains(&(*n, member))));
                // Every supertype names a type this scan never opened, so the
                // member can only have come from outside it — `getClass` off
                // `java.lang.Object`, `findColumn` off `java.sql.ResultSet`.
                let all_parents_are_foreign =
                    parents.is_some_and(|ns| ns.iter().all(|n| !kind_of.contains_key(n)));
                // What javac writes onto every enum, in no source file.
                let enum_synthesised =
                    kind_of.get(ty) == Some(&SymbolKind::Enum) && ENUM_SYNTHESISED.contains(member);

                let bucket = if as_a_field {
                    "the declaration IS here, at the field reach"
                } else if a_parent_declares_it {
                    "a first-party SUPERTYPE declares it"
                } else if lombok_on_the_type && an_accessor {
                    "Lombok synthesises it"
                } else if parents.is_some_and(|s| s.iter().any(|n| SPRING_DATA.contains(n))) {
                    "Spring Data's repository declares it"
                } else if enum_synthesised {
                    "javac synthesises it on every enum"
                } else if !declared.contains(owner) {
                    "the owning type is not declared by this scan"
                } else if all_parents_are_foreign {
                    "inherited from a supertype this scan never opened"
                } else if !owns_something.contains(owner) {
                    "the owning type declares no member at all — the walk's gap"
                } else {
                    "unexplained"
                };
                *buckets.entry(bucket).or_default() += 1;
                let entry = sample.entry(bucket).or_default();
                if entry.len() < 4 {
                    entry.push(format!("{ty}.{member}"));
                }
                if bucket == "unexplained" {
                    *remainder.entry(format!("{ty}.{member}")).or_default() += 1;
                }
            }
        }

        println!(
            "\n## Java: first-party edges naming no declaration — {total} across {} identities\n",
            identities.len()
        );
        println!("  by ref kind: {by_kind:?}");
        println!("  by rung:     {by_rung:?}\n");
        let mut ranked: Vec<(&&str, &usize)> = buckets.iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (bucket, n) in &ranked {
            println!("  {n:>7}  {:>5.1}%  {bucket}", 100.0 * **n as f64 / total.max(1) as f64);
            for example in sample.get(**bucket).into_iter().flatten() {
                println!("             {example}");
            }
        }
        // ── what the field-reach fix would cost, and it is not nothing ──────
        //
        // The second bucket is a plain defect: `field_declaration` mints a
        // declaration at `Reach::Field` and `refer_to_member` mints EVERY
        // member use site at `Reach::Item`, so a first-party field read can
        // never meet its own declaration. Passing the use site's reach through
        // would land all of them.
        //
        // It is NOT a one-line change, and this is the number that says so: an
        // `enum_constant` is declared at `Reach::Item`, so `Status.ACTIVE` is a
        // `field_access` whose declaration is an ITEM. Minting `Field` for
        // every field access would land the first group and UNLAND this one —
        // dangling edges traded for dangling edges. Both sides are measured
        // here so the trade is a decision and not a discovery.
        let mut reads_landing: BTreeMap<&'static str, usize> = BTreeMap::new();
        for Placed { facts, .. } in &corpus {
            for reference in &facts.references {
                if reference.kind != crate::indexer::facts::RefKind::Reads {
                    continue;
                }
                let Resolution::Resolved { fqn, .. } = &reference.target else { continue };
                if !declared.contains(fqn.as_str()) {
                    continue;
                }
                let Ok(parsed) = fqn::parse(fqn.as_str()) else { continue };
                let fqn::Origin::Local { reach, .. } = parsed.origin else { continue };
                *reads_landing.entry(reach.as_str()).or_default() += 1;
            }
        }
        println!("\n  field READS that land today, by the reach they land on: {reads_landing:?}");

        let mut worst: Vec<(&String, &usize)> = remainder.iter().collect();
        worst.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        println!("\n  the unexplained remainder's head, {} distinct:", remainder.len());
        for (what, n) in worst.iter().take(20) {
            println!("    {n:>5}  {what}");
        }

        // THE RATCHET. Measured; it may fall and must not rise. A dangling edge
        // is not a WRONG edge and R4 prefers it to one, but a reader following
        // "who calls this" gets the same nothing they would get if the caller
        // did not exist.
        assert!(
            total <= 27_000,
            "{total} first-party java edges name an identity no declaration mints, up from \
             the 26,995 measured"
        );
        assert_eq!(
            buckets.values().sum::<usize>(),
            total,
            "the buckets must partition every dangling edge, or the split is hiding some"
        );
    }
}
