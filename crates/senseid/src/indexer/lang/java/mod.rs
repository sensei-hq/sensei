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
