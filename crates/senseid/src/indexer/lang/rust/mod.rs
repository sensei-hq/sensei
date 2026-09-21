//! Reading the Rust grammar (spec §7 D4, stages 3 and 4).
//!
//! This module owns one thing: how a Rust source file is READ (R7). It mints no
//! identity of its own — every fqn comes back through `fqn::define`/`fqn::refer`
//! — and it resolves nothing across files, because resolution is a shared rule
//! and a shared rule does not live in a language module.
//!
//! Three files, three questions:
//!
//! | | |
//! |---|---|
//! | here | what Rust IS: its identity rules, its vocabulary for the ladder |
//! | `walk.rs` | what the parser just handed us |
//! | `types.rs` | what a piece of type source text NAMES |
//!
//! Decisions this walk makes about what counts as a declaration and what counts
//! as a use site are recorded next to the code that makes them, and the
//! independent counters in the tests are written against the same definitions
//! from the other side.
//
// This module has no caller on purpose — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

mod types;
mod walk;

use super::{LanguageAdapter, ReadError, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach};
use crate::indexer::resolve::{Grammar, Root};

/// Rust, as the registry sees it.
///
/// A zero-sized type, so the registry holds `&'static dyn LanguageAdapter`
/// with no allocation and no lifetime to thread. Every method below is a thin
/// forward to a free function in this module: the FUNCTIONS are the rules, and
/// the adapter is only how a caller reaches them without a `match`.
pub struct RustAdapter;

impl LanguageAdapter for RustAdapter {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn name(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".rs"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    /// The table is IGNORED, and the parameter's name says so rather than
    /// leaving a reader to find out. Rust reads a type's home from the file
    /// (S5); the other four adapters still need theirs, and TypeScript has no
    /// other source for one — so the TRAIT keeps the parameter until the last
    /// adapter loses its need for it (§11), which is an order of work and not a
    /// flag.
    fn read(&self, source: &Source<'_>, _types: &TypeHomes) -> Result<FileFacts, ReadError> {
        read(source)
    }

    fn file_fqn(&self, package: &str, module: &str, path: &str) -> Result<Fqn, FqnError> {
        file_fqn(package, module, path)
    }

    fn module_path(&self, file: &str, package_root: &str) -> String {
        module_path(file, package_root)
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        types::type_segment(raw)
    }
}

/// The module segment standing for a file that IS a crate root, where the
/// module path is empty and no `mod` declaration anywhere names the file. It is
/// a reserved word, so no declaration can ever mint the same identity.
const CRATE_ROOT: &str = "crate";

/// What the shared resolution ladder needs to know about Rust, and nothing more
/// (R7). The rungs and the reason codes are `resolve.rs`'s; this is only the
/// spelling.
pub const GRAMMAR: crate::indexer::resolve::Grammar = crate::indexer::resolve::Grammar {
    language: Language::Rust,
    // Rust spells a use-site path and a module path alike; every other
    // language here does not, which is why these are two fields.
    path_separator: "::",
    module_separator: "::",
    roots: &[(CRATE_ROOT, Root::Package), ("self", Root::Here), ("super", Root::Up)],
    // A `use` path names modules, never files, so there is no extension in it.
    module_segment: crate::indexer::lang::common::already_a_module_segment,
    // Rust's `self::x` names a member of the module `x` is written in, not
    // a sibling file.
    relative_to_directory: false,
    // `serde_json::json!(..)` is a complete use with no `use` anywhere.
    paths_name_packages: true,
    // Rust names each level with a token: `super::super::x`.
    relative_depth_prefix: None,
    names_the_binding: Some(" as "),
    wildcard: Some("*"),
    // Rust lints every type into `CamelCase` and every module into
    // `snake_case`, so the name states which of the two a segment is. See
    // `Grammar::names_a_type` for what a misread costs.
    names_a_type: crate::indexer::lang::common::names_a_type_by_leading_case,
    prelude: PRELUDE,
    plumbing: PLUMBING,
};

/// The names Rust puts in scope with nothing written to bring them there, each
/// with the path inside [`GRAMMAR`]'s `prelude_package` that re-exports it.
///
/// The path and not the bare name, so that a file writing `use std::vec::Vec;`
/// and a file relying on the prelude land on ONE node instead of two. The set is
/// `std::prelude::v1` plus the standard macros, which are in scope on the same
/// terms.
const PRELUDE: &[(&str, &str, &str)] = &[
    ("Box", "std", "boxed::Box"),
    ("String", "std", "string::String"),
    ("ToString", "std", "string::ToString"),
    ("Vec", "std", "vec::Vec"),
    ("Option", "std", "option::Option"),
    ("Some", "std", "option::Option::Some"),
    ("None", "std", "option::Option::None"),
    ("Result", "std", "result::Result"),
    ("Ok", "std", "result::Result::Ok"),
    ("Err", "std", "result::Result::Err"),
    ("Clone", "std", "clone::Clone"),
    ("Copy", "std", "marker::Copy"),
    ("Send", "std", "marker::Send"),
    ("Sync", "std", "marker::Sync"),
    ("Sized", "std", "marker::Sized"),
    ("Unpin", "std", "marker::Unpin"),
    ("Drop", "std", "ops::Drop"),
    ("Fn", "std", "ops::Fn"),
    ("FnMut", "std", "ops::FnMut"),
    ("FnOnce", "std", "ops::FnOnce"),
    ("PartialEq", "std", "cmp::PartialEq"),
    ("PartialOrd", "std", "cmp::PartialOrd"),
    ("Eq", "std", "cmp::Eq"),
    ("Ord", "std", "cmp::Ord"),
    ("AsRef", "std", "convert::AsRef"),
    ("AsMut", "std", "convert::AsMut"),
    ("Into", "std", "convert::Into"),
    ("From", "std", "convert::From"),
    ("TryInto", "std", "convert::TryInto"),
    ("TryFrom", "std", "convert::TryFrom"),
    ("Iterator", "std", "iter::Iterator"),
    ("IntoIterator", "std", "iter::IntoIterator"),
    ("FromIterator", "std", "iter::FromIterator"),
    ("DoubleEndedIterator", "std", "iter::DoubleEndedIterator"),
    ("ExactSizeIterator", "std", "iter::ExactSizeIterator"),
    ("Extend", "std", "iter::Extend"),
    ("ToOwned", "std", "borrow::ToOwned"),
    ("drop", "std", "mem::drop"),
    ("print", "std", "print"),
    ("println", "std", "println"),
    ("eprint", "std", "eprint"),
    ("eprintln", "std", "eprintln"),
    ("format", "std", "format"),
    ("format_args", "std", "format_args"),
    ("vec", "std", "vec"),
    ("write", "std", "write"),
    ("writeln", "std", "writeln"),
    ("panic", "std", "panic"),
    ("assert", "std", "assert"),
    ("assert_eq", "std", "assert_eq"),
    ("assert_ne", "std", "assert_ne"),
    ("debug_assert", "std", "debug_assert"),
    ("debug_assert_eq", "std", "debug_assert_eq"),
    ("debug_assert_ne", "std", "debug_assert_ne"),
    ("todo", "std", "todo"),
    ("unimplemented", "std", "unimplemented"),
    ("unreachable", "std", "unreachable"),
    ("matches", "std", "matches"),
    ("dbg", "std", "dbg"),
    ("include", "std", "include"),
    ("include_str", "std", "include_str"),
    ("include_bytes", "std", "include_bytes"),
    ("concat", "std", "concat"),
    ("stringify", "std", "stringify"),
    ("env", "std", "env"),
    ("option_env", "std", "option_env"),
    ("line", "std", "line"),
    ("column", "std", "column"),
    ("file", "std", "file"),
    ("module_path", "std", "module_path"),
    ("cfg", "std", "cfg"),
    ("compile_error", "std", "compile_error"),
];

/// Members every value has, from a blanket impl or a derive.
///
/// A miss on one of these is not a gap anybody can close — no first-party
/// declaration is on the other end of it — and there are thousands, so leaving
/// them in the general bucket buries the misses that ARE worth closing. This is
/// the list the `Plumbing` reason exists for: filtering, not failure, and only
/// ever applied to a reference the ladder has already failed to place, so a
/// provable edge is never dropped by it.
const PLUMBING: &[&str] = &[
    "clone",
    "clone_from",
    "to_owned",
    "to_string",
    "to_vec",
    "into",
    "try_into",
    "as_ref",
    "as_mut",
    "as_str",
    "as_slice",
    "borrow",
    "borrow_mut",
    "deref",
    "deref_mut",
    "fmt",
    "eq",
    "ne",
    "cmp",
    "partial_cmp",
    "hash",
    "drop",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_err",
    "expect",
    "expect_err",
    "ok",
    "err",
    "is_ok",
    "is_err",
    "is_some",
    "is_none",
    "ok_or",
    "ok_or_else",
    "map",
    "map_err",
    "and_then",
    "or_else",
];

/// Parse one file once (R1) and return everything that parse saw (spec §3).
pub fn read(source: &Source<'_>) -> Result<FileFacts, ReadError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;

    // The file's own identity is minted HERE and handed down, because it is an
    // identity rule and the walk owns none.
    let from =
        file_fqn(source.package, source.module, source.path).map_err(ReadError::NoFileIdentity)?;
    let mut found = walk::walk(source, tree.root_node(), from.clone());

    // The file declares its own module, and is the only thing that does.
    // `module_item` no longer declares a body-less `mod x;`, so the file is the
    // single declarer of its own identity. See `common::file_module`.
    let its_own = from.clone();
    found.symbols.insert(
        0,
        crate::indexer::lang::common::file_module(
            from,
            &module_name_of(source.module, source.path),
            source.text,
        ),
    );

    // ...and an import that names a MODULE is this file entering it. See
    // `common::import_references`.
    let entered =
        crate::indexer::lang::common::import_references(Language::Rust, &found.imports, &its_own);
    found.references.extend(entered);

    Ok(FileFacts {
        language: Language::Rust,
        package: source.package.to_string(),
        module: source.module.to_string(),
        path: source.path.to_string(),
        symbols: found.symbols,
        references: found.references,
        relations: found.relations,
        imports: found.imports,
    })
}

/// The NAME segment of a file's own module identity — the last segment of its
/// module path, or the crate-root stem for a file the module tree does not
/// name. Reads the same two cases [`file_fqn`] does, so the name beside the
/// identity cannot disagree with the name inside it.
fn module_name_of(module: &str, path: &str) -> String {
    match module.rsplit_once("::") {
        Some((_, name)) => name.to_string(),
        None if module.is_empty() => crate_root(path).to_string(),
        None => module.to_string(),
    }
}

/// The identity of the file itself, which is the identity of the module it
/// declares. Minted exactly the way the `mod x;` that names this file mints it —
/// last segment as the name, everything before it as the module, and the same
/// [`Reach::Mod`] — so the file and its declaration are one node and not two.
///
/// This and [`Walk::module_item`] are the ONLY two producers of `mod`, and they
/// move together or not at all: one of them at `item` would put a file and the
/// `mod x;` that names it on two nodes, which is the two-half-symbols failure
/// the merge key exists to prevent.
pub fn file_fqn(package: &str, module: &str, path: &str) -> Result<Fqn, FqnError> {
    let (parent, name) = match module.rsplit_once("::") {
        Some((parent, name)) => (parent, name),
        // A crate root is the one file the module tree does not name, so its
        // identity is its own — see [`crate_root`].
        None if module.is_empty() => (CRATE_ROOT, crate_root(path)),
        None => ("", module),
    };
    fqn::define(&Form::Item { lang: Language::Rust, package, module: parent, name, reach: MODULE })
}

/// A file's crate-relative MODULE PATH, from its path alone — Rust's
/// file-as-module rule (09 S5, and the missing producer for
/// [`Source::module`](crate::indexer::facts::Source)).
///
/// Relative to `<crate_root>/src` (or the crate root itself), drop the
/// extension, drop a trailing `mod`/`lib`/`main`:
///
/// | file | module |
/// |---|---|
/// | `crates/x/src/lib.rs` | `""` (crate root) |
/// | `crates/x/src/a/b.rs` | `a::b` |
/// | `crates/x/src/a/mod.rs` | `a` |
///
/// THE ONE OWNER of this rule, and it had three. A private
/// `rust_module_path` in the legacy `languages::rust_lang`, a `#[cfg(test)]`
/// `indexer::module_of` that guessed the crate root by looking for `/src/`
/// instead of being told it, and nothing at all in this indexer's production
/// path. The test helper now delegates here; the legacy copy retires at stage 10.
///
/// It matters beyond tidiness: this string is a SEGMENT OF EVERY FQN the file
/// declares, so two implementations that disagree by one segment mint two
/// identities for one declaration — which is the identity break spec §2 exists
/// to prevent, arriving from the least likely direction.
pub fn module_path(file: &str, crate_root: &str) -> String {
    let file = std::path::Path::new(file);
    let root = std::path::Path::new(crate_root);
    let src = root.join("src");
    let rel = file.strip_prefix(&src).or_else(|_| file.strip_prefix(root)).unwrap_or(file);

    let mut segments: Vec<String> =
        rel.components().filter_map(|c| c.as_os_str().to_str().map(str::to_string)).collect();
    if let Some(last) = segments.last_mut()
        && let Some(stem) = std::path::Path::new(last.as_str()).file_stem().and_then(|s| s.to_str())
    {
        *last = stem.to_string();
    }
    // A crate root and a `mod.rs` name the DIRECTORY they sit in, not
    // themselves — that is the whole file-as-module rule, and dropping the
    // segment is what makes `a/mod.rs` and `a.rs` the same module.
    if segments.last().is_some_and(|s| s == "mod" || s == "lib" || s == "main") {
        segments.pop();
    }
    segments.join("::")
}

/// The name a CRATE ROOT is identified by: its own file stem.
///
/// Every other file is named by the `mod x;` that declares it, and its identity
/// must equal what that declaration mints or the two become separate nodes. A
/// crate root has no such declaration — its module path is empty — and a
/// package may have SEVERAL of them: `crates/mcp` has `lib.rs` beside `main.rs`,
/// and a `build.rs` would be a third. Reducing them all to one name made every
/// import and every file-scope use site of all of them one node's, which is two
/// files claiming one symbol (spec §2).
///
/// The stem sits under the module segment `crate`, which is a reserved word, so
/// no `mod` can ever mint the same identity and the "no declaration collides
/// with a file identity" property the reserved word bought is kept.
///
/// An empty stem is an ERROR and not a fallback: falling back would rebuild the
/// shared identity this exists to split, and a file the caller did not name is
/// a caller mistake, reported as [`ReadError::NoFileIdentity`].
fn crate_root(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.strip_suffix(".rs").unwrap_or(file)
}

/// The reach every module declaration is minted under — the file's own identity
/// and the `mod x;` that names it (spec §2.1).
///
/// A constant so the two producers cannot be changed apart, and so the reason
/// has one home: no reference EVER mints `mod`, because a module is spelled as
/// the `module` segment of other identities and never as a target. That is what
/// makes the separation free, and what stops `crate::installer::install(..)` —
/// a call to a function re-exported from a module of the same name — from
/// landing on the module. 7 such references in this repo, 6 distinct identities:
/// a wrong edge where the collapse would otherwise put one.
const MODULE: Reach = Reach::Mod;

#[cfg(test)]
mod tests {
    use super::types::simple_type_name;
    use super::*;
    use crate::indexer::lang::Home;

    /// Every reference's target, as `name -> resolution`.
    fn targets(text: &str) -> Vec<(String, String)> {
        let facts = read(&Source { package: "p", module: "m", path: "src/m.rs", text })
            .expect("the fixture parses");
        // RESOLVED, not just read. `read` mints a candidate identity; the
        // ladder is what places it against a declaration. A helper that stopped
        // at `read` would report every candidate as `Unplaced` and could not
        // tell "the walk minted nothing" from "the walk minted the right thing
        // and nothing looked it up" — which is exactly the distinction these
        // tests are about.
        let first_party = std::collections::BTreeSet::from(["p".to_string()]);
        let scanned = BTreeSet::new();
        let facts = crate::indexer::resolve::resolve(
            facts,
            &GRAMMAR,
            &crate::indexer::resolve::World {
                first_party: &first_party,
                first_party_members: &BTreeSet::new(),
                declared_members: &BTreeSet::new(),
                returns: &std::collections::BTreeMap::new(),
                scanned: &scanned,
            },
        );
        facts
            .references
            .iter()
            .map(|r| {
                let shown = match &r.target {
                    crate::indexer::facts::Resolution::Resolved { fqn: f, .. } => {
                        f.as_str().to_string()
                    }
                    crate::indexer::facts::Resolution::Unresolved { reason, evidence } => {
                        format!("UNRESOLVED({reason:?}) {}", evidence.name)
                    }
                };
                let name = match &r.target {
                    crate::indexer::facts::Resolution::Resolved { fqn: f, .. } => {
                        f.as_str().rsplit('\u{00B7}').nth(1).unwrap_or("").to_string()
                    }
                    crate::indexer::facts::Resolution::Unresolved { evidence, .. } => {
                        evidence.name.clone()
                    }
                };
                (name, shown)
            })
            .collect()
    }

    /// A `let` binding states the receiver's type, and the walk must read it.
    ///
    /// The member's identity is `…·<Type>·<member>`, so the TYPE is a segment of
    /// the key — `brew_install_script` alone is not an identity, and guessing by
    /// name alone is what `Reason::AmbiguousCandidates` exists to refuse.
    ///
    /// Before this, the walk resolved a receiver's type in exactly ONE case:
    /// `self`/`Self` inside a type's own body. It never visited a `let`
    /// declaration, so `let cfg = SenseiConfig::from_env(); cfg.method()` was
    /// `ReceiverTypeUnknown` with the answer written one line above. Measured on
    /// this workspace: 33,047 unresolved receivers, of which 1,908 are bound by
    /// an initialiser that names its type and 1,851 by an annotation.
    #[test]
    fn a_let_binding_gives_the_receiver_its_type() {
        let resolved: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config {\n\
               pub fn from_env() -> Self { Config }\n\
               pub fn script(&self) -> u32 { 1 }\n\
             }\n\
             pub fn go() -> u32 {\n\
               let cfg = Config::from_env();\n\
               cfg.script()\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            resolved,
            vec!["rust·p·m·Config·script·item".to_string()],
            "the initialiser names the type one line above the call"
        );
    }

    /// The same, from an ANNOTATION rather than an initialiser.
    #[test]
    fn a_type_annotation_gives_the_receiver_its_type() {
        let resolved: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config { pub fn script(&self) -> u32 { 1 } }\n\
             pub fn go(make: impl Fn() -> Config) -> u32 {\n\
               let cfg: Config = make();\n\
               cfg.script()\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(resolved, vec!["rust·p·m·Config·script·item".to_string()]);
    }

    /// A QUALIFIED associated call names its type in the second-to-last
    /// segment, and the walk must read that one rather than the first.
    ///
    /// `Config::from_env()` worked only by accident: a two-segment path puts the
    /// type first, so reading the head found it. Add a module in front and the
    /// head is `crate` — which names no type, so the binding went untyped and
    /// every member read off it became `ReceiverTypeUnknown`. MEASURED on this
    /// workspace: 644 `let` bindings take this shape, including the
    /// `sensei_bootstrap::SenseiConfig::from_env()` behind `cfg.db_url`.
    ///
    /// The LAST segment is the associated function, so the type is the one
    /// before it. This is the arm that differs from the two below, which is why
    /// it is tested apart from them.
    #[test]
    fn a_qualified_associated_call_names_its_type_in_the_segment_before_the_call() {
        let resolved: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config {\n\
               pub fn from_env() -> Self { Config }\n\
               pub fn script(&self) -> u32 { 1 }\n\
             }\n\
             pub fn go() -> u32 {\n\
               let cfg = crate::m::Config::from_env();\n\
               cfg.script()\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            resolved,
            vec!["rust·p·m·Config·script·item".to_string()],
            "`crate::m::Config::from_env` names `Config`, not `crate`"
        );
    }

    /// A QUALIFIED struct literal names its type in the LAST segment — the
    /// opposite end from the associated call above, because no function name
    /// follows it.
    #[test]
    fn a_qualified_struct_literal_names_its_type_in_the_last_segment() {
        let resolved: Vec<String> = targets(
            "pub struct Config { pub width: u32 }\n\
             impl Config { pub fn script(&self) -> u32 { 1 } }\n\
             pub fn go() -> u32 {\n\
               let cfg = crate::m::Config { width: 1 };\n\
               cfg.script()\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            resolved,
            vec!["rust·p·m·Config·script·item".to_string()],
            "a qualified struct literal names `Config`, not `crate`"
        );
    }

    /// A QUALIFIED unit struct names its own type, also in the last segment.
    ///
    /// Its own test rather than a case of the one above: a unit struct reaches
    /// the walk as a bare path with no literal and no call around it, so it
    /// arrives on a different arm and a fix to the other two would leave it
    /// untyped.
    #[test]
    fn a_qualified_unit_struct_names_its_type_in_the_last_segment() {
        let resolved: Vec<String> = targets(
            "pub struct Provider;\n\
             impl Provider { pub fn script(&self) -> u32 { 1 } }\n\
             pub fn go() -> u32 {\n\
               let p = crate::m::Provider;\n\
               p.script()\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            resolved,
            vec!["rust·p·m·Provider·script·item".to_string()],
            "`crate::m::Provider` names `Provider`, not `crate`"
        );
    }

    /// A parameter destructured by a ONE-BINDING wrapper is typed, and the
    /// type is the one written in the signature beside it.
    ///
    /// `param_bindings` refused every pattern that was not a bare identifier,
    /// on the grounds that "a destructuring parameter binds several names of
    /// several types". That is true of `(a, b): (X, Y)` and false of this: one
    /// name, and the type argument is right there. MEASURED on this workspace:
    /// 470 parameters take this shape and ALL of them are the four axum
    /// extractors — `State` 240, `Path` 105, `Json` 78, `Query` 47 — worth 632
    /// otherwise-untyped field reads, the single largest recoverable group.
    #[test]
    fn a_one_binding_extractor_parameter_is_typed_by_its_type_argument() {
        let resolved: Vec<String> = targets(
            "pub struct AppState;\n\
             impl AppState { pub fn pg(&self) -> u32 { 1 } }\n\
             pub fn go(State(state): State<AppState>) -> u32 { state.pg() }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "pg")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            resolved,
            vec!["rust·p·m·AppState·pg·item".to_string()],
            "`State(state): State<AppState>` binds one name, and the signature states its type"
        );
    }

    /// A FIRST-PARTY newtype is typed from its own DECLARATION, not from the
    /// type argument — because we indexed the declaration and it is the only
    /// thing that actually says what field 0 holds.
    ///
    /// The two halves are different rules on purpose. `State<AppState>` is
    /// external: nothing tells us its field 0 is its type argument except that
    /// we listed it. `Wrap(Config)` is ours: the answer is read, not assumed.
    #[test]
    fn a_one_binding_first_party_newtype_is_typed_by_its_declared_field() {
        let resolved: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config { pub fn width(&self) -> u32 { 1 } }\n\
             pub struct Wrap(Config);\n\
             pub fn go(Wrap(c): Wrap) -> u32 { c.width() }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "width")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            resolved,
            vec!["rust·p·m·Config·width·item".to_string()],
            "the declaration says field 0 is a `Config`, so the binding is one"
        );
    }

    /// SEVERAL names is still refused, which is the case the original blanket
    /// rule was actually written for.
    ///
    /// `Pair`'s field 0 IS a `Config`, so a rule that looked up field 0 without
    /// first counting the bindings would type `a` and look correct here — and
    /// then type `b` as a `Config` too on a `Pair(Config, Widget)`. The count
    /// is the guard, not the lookup.
    #[test]
    fn a_parameter_binding_several_names_is_still_refused() {
        let shown: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config { pub fn width(&self) -> u32 { 1 } }\n\
             pub struct Pair(Config, Config);\n\
             pub fn go(Pair(a, _b): Pair) -> u32 { a.width() }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "width")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec!["UNRESOLVED(ReceiverTypeUnknown) width".to_string()],
            "two names, and attributing one field's type to both is the false claim"
        );
    }

    /// A wrapper that is NEITHER first-party NOR on the list states nothing.
    ///
    /// The list is what keeps this narrow. `struct W<T>(Vec<T>)` has a single
    /// type argument and a single field, and its field is NOT its argument — so
    /// unwrapping any single-argument generic would mint `u32` where the truth
    /// is `Vec<u32>`, and a wrong receiver type mints a wrong key (R4).
    #[test]
    fn a_one_binding_wrapper_we_know_nothing_about_is_refused() {
        let shown: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config { pub fn width(&self) -> u32 { 1 } }\n\
             pub fn go(Mystery(m): Mystery<Config>) -> u32 { m.width() }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "width")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec!["UNRESOLVED(ReceiverTypeUnknown) width".to_string()],
            "nothing here says what `Mystery`'s field 0 holds, so nothing is claimed"
        );
    }

    /// A binding the walk cannot type stays UNRESOLVED. It does not fall back to
    /// the enclosing type, and it does not guess from the member name — a wrong
    /// receiver type mints a wrong identity, and R4 ranks that below no answer.
    #[test]
    fn a_binding_whose_type_is_not_stated_stays_unresolved() {
        let shown: Vec<String> = targets(
            "pub fn go(x: u32) -> u32 {\n\
               let thing = helper(x);\n\
               thing.script()\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec!["UNRESOLVED(ReceiverTypeUnknown) script".to_string()],
            "`helper(x)` names no type, so the receiver has none — and inventing one is worse \
             than saying so"
        );
    }

    /// A binding is visible only AFTER its `let`, and only inside its block.
    ///
    /// Both halves matter. Recording it too early types a use of an OUTER
    /// binding with the inner one's type; leaking it past the block types a
    /// later, unrelated `cfg` the same way. Either is a wrong identity that
    /// nothing downstream can tell from a right one.
    #[test]
    fn a_binding_does_not_escape_its_block_or_precede_its_own_let() {
        let shown: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config { pub fn script(&self) -> u32 { 1 } }\n\
             pub fn go() -> u32 {\n\
               { let cfg = Config::from_env(); cfg.script(); }\n\
               cfg.script()\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec![
                "rust·p·m·Config·script·item".to_string(),
                "UNRESOLVED(ReceiverTypeUnknown) script".to_string(),
            ],
            "inside the block it is typed; outside it the name is not bound and must not be"
        );
    }

    /// A parameter's type is stated in the signature, and the body knows it.
    ///
    /// Measured on this workspace before the fix: 3,873 unresolved receivers are
    /// plain identifiers the enclosing signature types.
    #[test]
    fn a_parameter_gives_the_receiver_its_type() {
        let shown: Vec<String> = targets(
            "pub struct Config;\n\
             impl Config { pub fn script(&self) -> u32 { 1 } }\n\
             pub fn go(cfg: Config) -> u32 { cfg.script() }\n\
             pub fn other(cfg: u32) -> u32 { cfg.script() }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec![
                "rust·p·m·Config·script·item".to_string(),
                // `u32` is not a type this grammar can mint a member of, and
                // `simple_type_name` takes only capitalised names — so nothing
                // is recorded and the receiver's type stays genuinely UNKNOWN.
                // Not `Unplaced`, which would mean a candidate was minted and
                // not found: no candidate exists, and the reason says so.
                "UNRESOLVED(ReceiverTypeUnknown) script".to_string(),
            ],
            "the signature types the first receiver; the second names a primitive"
        );
    }

    /// `&T` and `&mut T` carry `T`'s methods; `Arc<T>` is refused.
    ///
    /// The split is not squeamishness. Rust auto-derefs a reference receiver, so
    /// `(&Config).script()` IS `Config::script` — a language fact. A smart
    /// pointer derefs too, but it also has inherent members of its own, so
    /// `arc.clone()` could be `Arc::clone` or `Config::clone` and choosing is a
    /// coin flip that mints a wrong identity half the time (R4).
    #[test]
    fn a_reference_receiver_carries_the_referents_methods() {
        assert_eq!(simple_type_name("&Config"), Some("Config".to_string()));
        assert_eq!(simple_type_name("&mut Config"), Some("Config".to_string()));
        assert_eq!(simple_type_name("&&Config"), Some("Config".to_string()));
        assert_eq!(simple_type_name("Config"), Some("Config".to_string()));

        // A PATH is typed by its last segment: the module path says where the
        // type is declared, not what it is.
        assert_eq!(simple_type_name("crate::a::Config"), Some("Config".to_string()));
        assert_eq!(simple_type_name("&std::path::PathBuf"), Some("PathBuf".to_string()));
        assert_eq!(simple_type_name("a::b::Wrapper<T>"), Some("Wrapper".to_string()));

        // `Vec<Config>` is NOT here any more. It used to be, on the reasoning
        // that a generic spelling is not a mintable identity — but the question
        // a receiver asks is which type OWNS the member, and that is `Vec`.
        // See `a_generic_receiver_is_typed_by_the_type_that_owns_the_member`.
        for refused in ["Arc<Config>", "Box<Config>", "u32", "&str"] {
            assert_eq!(simple_type_name(refused), None, "{refused} must not be recorded");
        }
    }

    #[test]
    fn repro_content_hash() {
        for (n, t) in targets(
            "pub fn content_hash(c: &str) -> String { c.into() }\n\
             pub struct PublishedRule { pub content_hash: String }\n\
             pub fn go() { let r = PublishedRule { content_hash: content_hash(\"x\") }; \
             let _ = r.content_hash; }\n",
        ) {
            println!("  {n:24} -> {t}");
        }
    }

    /// A GENERIC receiver is typed by its head, so a call on it is classified
    /// rather than called unknown.
    ///
    /// `Vec<Config>::push` is `Vec`'s method — the type argument does not change
    /// which type OWNS the member. Refusing the whole spelling left 82% of the
    /// attributable unresolved receivers (7,785 of 9,534, measured) reported as
    /// "type unknown" when the type was written down and simply was not ours.
    /// "We do not know" and "we know, and it is out of scope" are different
    /// facts, and only the second is true here.
    #[test]
    fn a_generic_receiver_is_typed_by_the_type_that_owns_the_member() {
        assert_eq!(simple_type_name("Vec<Config>"), Some("Vec".to_string()));
        assert_eq!(simple_type_name("Option<String>"), Some("Option".to_string()));
        assert_eq!(simple_type_name("HashMap<String, u32>"), Some("HashMap".to_string()));
        assert_eq!(simple_type_name("&Vec<Config>"), Some("Vec".to_string()));
        // A first-party generic works the same way — the rule is about the
        // grammar, not about who owns the crate.
        assert_eq!(simple_type_name("Wrapper<T>"), Some("Wrapper".to_string()));
    }

    /// A DEREF wrapper is still refused, and that is the one exception.
    ///
    /// `Arc<Config>` derefs to `Config`, so `arc.method()` may be `Arc::method`
    /// OR `Config::method` and the source does not say which. Taking the head
    /// would mint `Arc·method` for every call that is really on the inner type —
    /// a wrong identity, on the half of the cases the rule guesses wrong (R4).
    ///
    /// `Vec` and friends do not have this problem: the member is on the
    /// container, and nothing is being unwrapped.
    #[test]
    fn a_deref_wrapper_is_refused_because_the_member_may_be_on_the_inner_type() {
        for wrapper in ["Arc<Config>", "Rc<Config>", "Box<Config>", "Mutex<Config>", "Cow<Config>"]
        {
            assert_eq!(simple_type_name(wrapper), None, "{wrapper} is ambiguous under Deref");
        }
        // Still refused: no type this grammar can own a member of is named.
        // `crate::a::Config` moved OUT of this list — a path is typed by its
        // last segment, which is the type; the module path in front says where
        // it is declared, not what it is.
        for refused in ["u32", "&str", "[u8; 4]"] {
            assert_eq!(simple_type_name(refused), None, "{refused}");
        }
    }

    /// `self.m().member` — the inner call resolves; only `m`'s RETURN type is
    /// missing, and `m` is a method of this same type.
    ///
    /// Declared AFTER the call on purpose: a single-pass walk would not have
    /// seen `dir` yet, which is why the return types are collected in a pre-pass
    /// over the impl block. Move the pre-pass and this fails.
    #[test]
    fn a_chained_call_on_self_is_typed_by_what_the_method_returns() {
        let shown: Vec<String> = targets(
            "pub struct Holder;\n\
             pub struct Cfg;\n\
             impl Cfg { pub fn script(&self) -> u32 { 1 } }\n\
             impl Holder {\n\
               pub fn go(&self) -> u32 { self.dir().script() }\n\
               pub fn dir(&self) -> Cfg { Cfg }\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec!["rust·p·m·Cfg·script·item".to_string()],
            "`dir` returns Cfg, so `.script()` is Cfg's — even though `dir` is declared below"
        );
    }

    /// A chain DEEPER than one hop is not guessed at.
    ///
    /// `self.a().b().c()` needs the return type of `b`, which this impl block
    /// does not declare. Reaching for the enclosing type there would mint a
    /// member on the wrong one (R4), so it stays unresolved and says why.
    #[test]
    fn a_chain_deeper_than_one_hop_stays_unresolved() {
        let shown: Vec<String> = targets(
            "pub struct Cfg;\n\
             impl Cfg { pub fn one(&self) -> Cfg { Cfg } }\n\
             impl Cfg { pub fn go(&self) -> u32 { self.one().one().script() } }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(shown, vec!["UNRESOLVED(ReceiverTypeUnknown) script".to_string()]);
    }

    /// A `for` binding is a DECLARATION: the collection states the element type.
    ///
    /// It was reaching nothing purely because the walk had no rule for it —
    /// 1,735 unresolved receivers on this workspace, larger than every route
    /// built so far except the signature one, and invisible because the
    /// measurement had no name for it.
    #[test]
    fn a_for_binding_is_typed_by_the_collections_element_type() {
        let shown: Vec<String> = targets(
            "pub struct Cfg;\n\
             impl Cfg { pub fn script(&self) -> u32 { 1 } }\n\
             pub struct Holder { pub items: Vec<Cfg> }\n\
             impl Holder {\n\
               pub fn go(&self) { for c in &self.items { c.script(); } }\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(shown, vec!["rust·p·m·Cfg·script·item".to_string()]);
    }

    /// A `for` PATTERN names types, and the walk was not reading it at all.
    ///
    /// `for_expression` walked the collection and the body and skipped the
    /// pattern between them, so `for Placed { facts, .. } in &corpus` never
    /// emitted a use site for `Placed`. The type is written down in the source
    /// and the graph had no edge for it.
    ///
    /// FOUND BY A2, not by reading: the independent counter counts the
    /// `type_identifier` in a struct pattern and the walk emitted nothing, so
    /// the first file in this repository to use the shape took A2 from 0 files
    /// disagreeing to 1, at 644 references against 649.
    ///
    /// MUTATION: drop the `self.node(pattern, scope)` from `for_expression` and
    /// this returns no `Holder` at all.
    #[test]
    fn a_for_pattern_names_the_type_it_destructures() {
        let shown: Vec<String> = targets(
            "pub struct Cfg;\n\
             pub struct Holder { pub items: Vec<Cfg> }\n\
             pub fn go(all: &[Holder]) { for Holder { items } in all { let _ = items; } }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "Holder")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec!["rust·p·m·Holder·item".to_string(), "rust·p·m·Holder·item".to_string()],
            "the parameter's element type and the pattern's own name are two use sites"
        );
    }

    /// `Box<dyn Trait>` resolves to the TRAIT METHOD.
    ///
    /// The concrete impl is unknowable — that is what dynamic dispatch means —
    /// but WHICH METHOD is called is not in doubt, and the trait declares it.
    /// Refusing the edge loses that fact to protect against a question nobody
    /// asked. "Who implements it" is answered separately, from the `implements`
    /// relations the walk already records.
    #[test]
    fn a_dyn_receiver_resolves_to_the_trait_method() {
        let shown: Vec<String> = targets(
            "pub trait Checker { fn check(&self) -> u32; }\n\
             pub struct And { pub checkers: Vec<Box<dyn Checker>> }\n\
             impl And {\n\
               pub fn go(&self) { for c in &self.checkers { c.check(); } }\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "check")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(
            shown,
            vec!["rust·p·m·Checker·check·item".to_string()],
            "the trait declares `check`; which impl runs is a different question"
        );
    }

    /// And a `dyn` PARAMETER works the same way — the rule is about the type,
    /// not about what bound it.
    #[test]
    fn a_dyn_parameter_resolves_to_the_trait_method() {
        let shown: Vec<String> = targets(
            "pub trait Checker { fn check(&self) -> u32; }\n\
             pub fn go(c: &dyn Checker) -> u32 { c.check() }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "check")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(shown, vec!["rust·p·m·Checker·check·item".to_string()]);
    }

    /// A collection whose element type is NOT single and stated is refused.
    ///
    /// A map yields `(K, V)` pairs, so the binding is a tuple and attributing
    /// either type to it is false. A range yields a primitive this grammar owns
    /// no members of. Both stay unresolved rather than guessing (R4).
    #[test]
    fn a_binding_over_pairs_or_primitives_is_not_typed() {
        let shown: Vec<String> = targets(
            "use std::collections::HashMap;\n\
             pub struct Holder { pub m: HashMap<String, u32> }\n\
             impl Holder {\n\
               pub fn go(&self) { for e in &self.m { e.script(); } }\n\
             }\n",
        )
        .into_iter()
        .filter(|(name, _)| name == "script")
        .map(|(_, shown)| shown)
        .collect();

        assert_eq!(shown, vec!["UNRESOLVED(ReceiverTypeUnknown) script".to_string()]);
    }

    /// The file-as-module rule, on the cases that distinguish it from a naive
    /// "path minus extension".
    #[test]
    fn a_files_module_path_follows_rusts_file_as_module_rule() {
        let root = "/w/crates/x";
        for (file, want) in [
            ("/w/crates/x/src/lib.rs", ""),
            ("/w/crates/x/src/main.rs", ""),
            ("/w/crates/x/src/a.rs", "a"),
            ("/w/crates/x/src/a/b.rs", "a::b"),
            // A `mod.rs` names its DIRECTORY, not itself — the case a naive
            // rule gets wrong, and the one that makes a rename free below.
            ("/w/crates/x/src/a/mod.rs", "a"),
            ("/w/crates/x/src/a/b/mod.rs", "a::b"),
            // Outside `src/`: relative to the crate root instead. `build` and
            // NOT `""` — only `mod`/`lib`/`main` name their directory, and a
            // build script is not one of them. Cargo does compile `build.rs` as
            // its own crate, so an argument exists for `""`; this PROMOTED an
            // existing rule and changing behaviour while moving it would be a
            // change smuggled into a refactor.
            ("/w/crates/x/build.rs", "build"),
        ] {
            assert_eq!(module_path(file, root), want, "{file}");
        }
    }

    /// 09 S5. A rename re-mints identity exactly when it moves the module path.
    ///
    /// Both directions are asserted. A rule that only ever returned `true`
    /// would pass a test that checked the re-minting cases alone, and it is the
    /// FREE rename — the one where re-parsing is the waste R14 exists to avoid
    /// — that such a rule gets wrong.
    #[test]
    fn a_rename_remints_identity_exactly_when_the_module_path_moves() {
        let root = "/w/crates/x";

        assert!(
            RustAdapter.rename_remints_identity(
                "/w/crates/x/src/a.rs",
                "/w/crates/x/src/b.rs",
                root
            ),
            "module a -> b renames every declaration in the file"
        );
        assert!(
            RustAdapter.rename_remints_identity(
                "/w/crates/x/src/a.rs",
                "/w/crates/x/src/d/a.rs",
                root
            ),
            "a -> d::a likewise, even though the file name did not change"
        );
        // Rust module names are CASE-SENSITIVE, so this is a real re-mint —
        // contradicting the spec's own example, which offers a case-only change
        // as a free rename. The rule decides; the example was wrong.
        assert!(
            RustAdapter.rename_remints_identity(
                "/w/crates/x/src/a.rs",
                "/w/crates/x/src/A.rs",
                root
            ),
            "a -> A is a different module in Rust"
        );

        assert!(
            !RustAdapter.rename_remints_identity(
                "/w/crates/x/src/a/mod.rs",
                "/w/crates/x/src/a.rs",
                root
            ),
            "both spell module `a` — the bytes moved, the identities did not"
        );
        assert!(
            !RustAdapter.rename_remints_identity(
                "/w/crates/x/src/lib.rs",
                "/w/crates/x/src/main.rs",
                root
            ),
            "both are the crate root, whose module path is empty"
        );
    }

    /// The module path is a SEGMENT OF THE FQN, which is why the rename
    /// question is answered by comparing module paths at all.
    ///
    /// Asserted through `file_fqn` rather than by reading the string, so a
    /// change that stopped threading the module into the identity fails here
    /// instead of leaving the rename rule correct about a value nothing uses.
    #[test]
    fn the_module_path_is_what_file_fqn_is_built_from() {
        let root = "/w/crates/x";
        let a = file_fqn("x", &module_path("/w/crates/x/src/a.rs", root), "/w/crates/x/src/a.rs")
            .expect("a module file has an identity");
        let b = file_fqn("x", &module_path("/w/crates/x/src/b.rs", root), "/w/crates/x/src/b.rs")
            .expect("a module file has an identity");
        assert_ne!(a.as_str(), b.as_str(), "two modules, two identities");

        let via_mod = module_path("/w/crates/x/src/a/mod.rs", root);
        let via_file = module_path("/w/crates/x/src/a.rs", root);
        assert_eq!(
            file_fqn("x", &via_mod, "/w/crates/x/src/a/mod.rs").unwrap().as_str(),
            file_fqn("x", &via_file, "/w/crates/x/src/a.rs").unwrap().as_str(),
            "and the free rename really does land on the same identity"
        );
    }

    use crate::indexer::facts::{
        Binding, DeclaredType, Evidence, Fqn, ImportOrigin, Observation, Reason, RefKind,
        RelationKind, Resolution, Symbol, SymbolKind, Visibility,
    };

    /// One of every declaration the plan's step 3 names, in one file, so the
    /// expected set can be written out by hand and compared whole.
    const ONE_OF_EACH: &str = r#"
use std::collections::HashMap;

pub mod inner;

pub const MAX: u32 = 10;
pub static NAME: &str = "x";
pub type Alias = HashMap<String, u32>;

macro_rules! shout { () => {} }

/// A widget.
pub struct Widget {
    pub width: u32,
    height: u32,
}

pub struct Pair(pub u32, String);

pub enum Shape {
    Circle,
    Rect { w: u32 },
}

pub trait Draw {
    type Canvas;
    const SIDES: u32;
    fn draw(&self) -> String;
}

impl Widget {
    pub fn new(width: u32) -> Widget { Widget { width, height: 0 } }
    fn width(&self) -> u32 { self.width }
}

impl Draw for Widget {
    type Canvas = u32;
    const SIDES: u32 = 4;
    fn draw(&self) -> String { String::new() }
}

pub fn free(w: &Widget) -> u32 { w.width }
"#;

    fn facts(module: &str, text: &str) -> FileFacts {
        read(&Source { package: "p", module, path: "src/fixture.rs", text })
            .expect("the fixture parses")
    }

    fn fqns(facts: &FileFacts) -> Vec<&str> {
        facts.symbols.iter().map(|s| s.fqn.as_str()).collect()
    }

    /// The whole expected symbol set, written out. A count alone would pass
    /// while naming the wrong things.
    #[test]
    fn every_declaration_in_the_fixture_becomes_exactly_one_symbol() {
        let facts = facts("m", ONE_OF_EACH);
        let mut got = fqns(&facts);
        got.sort_unstable();

        let mut expected = vec![
            "rust·p·m·MAX·item",
            "rust·p·m·NAME·item",
            "rust·p·m·Alias·item",
            "rust·p·m·shout·macro",
            "rust·p·m·Widget·item",
            "rust·p·m·Widget·width·field",
            "rust·p·m·Widget·height·field",
            "rust·p·m·Pair·item",
            "rust·p·m·Pair·0·field",
            "rust·p·m·Pair·1·field",
            "rust·p·m·Shape·item",
            "rust·p·m·Shape·Circle·item",
            "rust·p·m·Shape·Rect·item",
            "rust·p·m·Shape::Rect·w·field",
            "rust·p·m·Draw·item",
            "rust·p·m·Draw·Canvas·item",
            "rust·p·m·Draw·SIDES·item",
            "rust·p·m·Draw·draw·item",
            "rust·p·m·Widget·new·item",
            "rust·p·m·Widget·width·item",
            // The trait impl's copies, keyed on the TYPE and the member with
            // NO trait segment (stage 11, S8) — and still three symbols apart
            // from the trait's OWN `Draw·Canvas` / `Draw·SIDES` / `Draw·draw`
            // above, which is the property worth keeping: 3 + 3, not 3.
            "rust·p·m·Widget·Canvas·item",
            "rust·p·m·Widget·SIDES·item",
            "rust·p·m·Widget·draw·item",
            "rust·p·m·free·item",
            // The FILE's own module. `pub mod inner;` no longer declares
            // `inner` — that module's body is another file, and that file
            // declares it. What this file declares of its own is itself.
            "rust·p·m·mod",
        ];
        expected.sort_unstable();

        assert_eq!(got, expected);
    }

    /// **A METHOD IS KEYED ON ITS TYPE AND ITS NAME. THE TRAIT IS AN EDGE**
    /// (stage 11, S8).
    ///
    /// A merge key must be what BOTH sides can produce, and the trait is a
    /// segment the caller can never spell — `b.draw()` says one thing, *Box2 is
    /// expected to have a method `draw`*, and which trait supplies it is what
    /// dispatch decides. So the declaration minting `Box2·Draw·draw` and the
    /// use site minting `Box2·draw` were two strings for one method, and the
    /// whole of the collapsed-spelling table — repo-wide, built at a barrier — was
    /// a translator between them.
    ///
    /// The two facts belong to two FILES and neither needs the other. This test
    /// reads them off two independent walks, neither told anything about the
    /// other, and requires the strings to be equal.
    ///
    /// The mutation that must break it: restore the trait segment — the
    /// `Container::TraitImpl` arm of `Walk::declare`, minting `Form::TraitMember`.
    #[test]
    fn a_trait_method_mints_one_key_from_both_sides() {
        // The DECLARATION, in the module `Box2` lives in.
        let declared = facts(
            "shape",
            "pub trait Draw { fn draw(&self) -> u32; }\n\
             pub struct Box2 { pub n: u32 }\n\
             impl Draw for Box2 { fn draw(&self) -> u32 { 0 } }\n",
        );
        // The one named `draw` that hangs off BOX2. The trait declares a `draw`
        // of its own and that is a different symbol, so the fixture would be
        // ambiguous without saying which.
        let minted_by_the_declaration: Vec<&str> = fqns(&declared)
            .into_iter()
            .filter(|f| f.contains("Box2") && f.ends_with("draw·item"))
            .collect();

        // The USE SITE, in another module, with no table and no sibling file —
        // only `use crate::shape::Box2`, which is where it learns Box2's home.
        let calling = facts(
            "start",
            "use crate::shape::Box2;\n\
             pub fn go(b: &Box2) -> u32 { b.draw() }\n",
        );
        let minted_by_the_caller: Vec<String> = calling
            .references
            .iter()
            .filter(|r| r.kind == RefKind::Calls)
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { evidence, .. } => Some(evidence),
                Resolution::Resolved { .. } => None,
            })
            .flat_map(Evidence::identities)
            .map(Fqn::to_string)
            .collect();

        assert_eq!(
            minted_by_the_declaration,
            vec!["rust·p·shape·Box2·draw·item"],
            "the declaration keys the method on its TYPE and its NAME; the trait is not in \
             the key. All of it: {:?}",
            fqns(&declared)
        );
        assert_eq!(
            minted_by_the_caller,
            vec!["rust·p·shape·Box2·draw·item".to_string()],
            "and the caller, reading Box2's home off its own import, mints the same string"
        );
    }

    /// **TWO TRAITS SUPPLYING ONE NAME ON ONE TYPE: ONE METHOD NODE, TWO
    /// `TraitImpl` EDGES** (stage 11, §7).
    ///
    /// Nothing is lost by taking the trait out of the key, because the trait was
    /// never lost — it is an edge, emitted by the file that writes
    /// `impl Wide for Box2`. `Box as Wide` and `Box as Tall` are both
    /// represented; what they do not do is split the method key, because a
    /// caller cannot spell the difference.
    ///
    /// The merge IS visible, and deliberately so: it shows up as an A7 identity
    /// collision rather than as a silently refused edge. See the ratchet in
    /// `acceptance::no_two_declarations_mint_one_identity`.
    ///
    /// The mutation that must break it: restore the trait segment.
    #[test]
    fn two_traits_on_one_type_are_one_method_node_and_two_edges() {
        let facts = facts(
            "shape",
            "pub trait Wide { fn draw(&self) -> u32; }\n\
             pub trait Tall { fn draw(&self) -> u32; }\n\
             pub struct Box2 { pub n: u32 }\n\
             impl Wide for Box2 { fn draw(&self) -> u32 { 0 } }\n\
             impl Tall for Box2 { fn draw(&self) -> u32 { 1 } }\n",
        );

        let on_box2: BTreeSet<&str> =
            fqns(&facts).into_iter().filter(|f| f.contains("Box2·")).collect();
        assert_eq!(
            on_box2,
            BTreeSet::from([
                "rust·p·shape·Box2·item",
                "rust·p·shape·Box2·draw·item",
                "rust·p·shape·Box2·n·field",
            ]),
            "two impls, ONE method node — a caller writing `b.draw()` can spell one string \
             and this is it"
        );

        // And the two facts the key gave up are right there as edges, each
        // emitted by the block that writes it.
        let traits: Vec<String> = facts
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::TraitImpl)
            .map(|r| match &r.parent {
                Resolution::Resolved { fqn, .. } => fqn.to_string(),
                Resolution::Unresolved { evidence, .. } => evidence.name.clone(),
            })
            .collect();
        assert_eq!(
            traits,
            vec!["Wide".to_string(), "Tall".to_string()],
            "both traits survive as edges into Box2 — nothing is lost, it moved"
        );
    }

    /// **AN INHERENT METHOD OWNS THE PLAIN KEY; A TRAIT'S COPY OF THAT NAME
    /// KEEPS THE TRAIT** — the one case S8 must NOT flatten.
    ///
    /// S8's argument is that a merge key must be what both sides can produce,
    /// and a caller cannot spell which trait supplies a name. That is true when
    /// the trait is the ONLY supplier. It is false the moment the type also
    /// declares the name inherently, because Rust then has a deterministic
    /// rule — the inherent method wins — and a caller that wants the other one
    /// spells it out.
    ///
    /// MEASURED, from this repository's own source
    /// (`api/model_provisioning.rs`), which does exactly that and says why:
    ///
    /// ```ignore
    /// async fn status_all(&self) -> Vec<(String, ProvisionPhase)> {
    ///     // Fully-qualified to call the inherent (disk-aware catalog) method,
    ///     // not recurse into this trait method.
    ///     ModelProvisioning::status_all(self).await
    /// }
    /// ```
    ///
    /// Flattened, both declarations mint one key and that call — written to
    /// AVOID recursion — becomes an edge from the method to itself. A self-loop
    /// is a WRONG edge, which R4 ranks below the missing one S8 was removing,
    /// so the flattening has to stop exactly here and nowhere wider.
    ///
    /// The rule is decided from THIS FILE, which is what keeps it inside the
    /// stage's budget: the walk pre-scans its own inherent `impl` blocks. A
    /// type whose inherent and trait impls live in different files is not
    /// covered and collides instead — which A7 counts, rather than silently
    /// merging.
    ///
    /// The mutation that must break it: drop the inherent-member check in
    /// `Walk::declare` so the trait arm always mints the plain key.
    #[test]
    fn an_inherent_method_keeps_its_key_and_the_traits_copy_keeps_the_trait() {
        let facts = facts(
            "m",
            "pub struct P;\n\
             impl P { pub fn status_all(&self) -> u32 { 1 } }\n\
             impl Ready for P {\n\
             \x20   fn status_all(&self) -> u32 { P::status_all(self) }\n\
             }\n",
        );

        let mut minted: Vec<&str> =
            fqns(&facts).into_iter().filter(|f| f.contains("status_all")).collect();
        minted.sort_unstable();
        assert_eq!(
            minted,
            vec!["rust·p·m·P·Ready·status_all·item", "rust·p·m·P·status_all·item"],
            "two declarations Rust keeps apart must stay two identities: the inherent one owns \
             the key `P::status_all` and `p.status_all()` both mint"
        );

        // AND THE CONSEQUENCE, which is the reason the rule exists: the
        // deliberately non-recursive call reaches the INHERENT method, not the
        // trait body it is written inside.
        let call = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the trait body calls the inherent method");
        let target = match &call.target {
            Resolution::Resolved { fqn, .. } => fqn.to_string(),
            Resolution::Unresolved { evidence, .. } => evidence
                .identities()
                .next()
                .map(ToString::to_string)
                .unwrap_or_else(|| evidence.name.clone()),
        };
        assert_eq!(target, "rust·p·m·P·status_all·item", "the call names the inherent method");
        assert_ne!(
            call.from.as_str(),
            target,
            "and it is NOT a self-loop — the call was written precisely to avoid recursing"
        );
    }

    /// The collision the fqn grammar's trailing REACH exists to prevent,
    /// checked end to end through the walk rather than through the builder.
    /// `nodes_unique_identity` is `(folder, path, kind, name, parent, line)`, so
    /// two declarations sharing a name on one type must differ in `kind` AND in
    /// identity — one of them being overwritten is a wrong edge (R4).
    #[test]
    fn a_field_and_a_method_of_the_same_name_are_two_symbols_with_two_identities() {
        let facts = facts("m", ONE_OF_EACH);
        let width: Vec<_> = facts.symbols.iter().filter(|s| s.name == "width").collect();

        assert_eq!(width.len(), 2, "the field and the method are both declarations");
        assert_ne!(width[0].fqn, width[1].fqn, "one identity would merge them onto one node");
        assert_ne!(width[0].kind, width[1].kind, "kind is what `nodes_unique_identity` separates");
        let mut kinds = width.iter().map(|s| s.kind).collect::<Vec<_>>();
        kinds.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(kinds, vec![SymbolKind::Field, SymbolKind::Method]);
    }

    /// A tuple-struct field has no name, so its identity is its POSITION, spelled
    /// the same way a use site spells it (`pair.0`). Written down here because
    /// the choice is arbitrary until it is recorded.
    #[test]
    fn a_tuple_struct_field_is_identified_by_its_position() {
        let facts = facts("m", ONE_OF_EACH);
        let positional: Vec<_> =
            facts.symbols.iter().filter(|s| s.fqn.as_str().contains("·Pair·")).collect();
        let positional: Vec<_> =
            positional.into_iter().filter(|s| s.kind == SymbolKind::Field).collect();

        assert_eq!(positional.len(), 2);
        for (index, symbol) in positional.iter().enumerate() {
            assert_eq!(symbol.name, index.to_string(), "the name is the position");
            assert_eq!(symbol.kind, SymbolKind::Field);
        }
        assert_eq!(positional[0].declared_type, DeclaredType::Stated("u32".to_string()));
        assert_eq!(positional[1].declared_type, DeclaredType::Stated("String".to_string()));
    }

    /// D2: a parameter is a typed prop on its function, never a node of its own.
    /// R3: the type the language states reaches the fact, verbatim.
    #[test]
    fn a_parameter_is_a_typed_prop_and_not_a_symbol() {
        let facts = facts("m", ONE_OF_EACH);
        assert!(
            !fqns(&facts)
                .iter()
                .any(|f| f.ends_with("·w·item") || f.ends_with("·width·item") && f.contains("free")),
            "no parameter may appear as a symbol"
        );

        let free = facts.symbols.iter().find(|s| s.name == "free").expect("the free fn");
        assert_eq!(free.kind, SymbolKind::Function);
        assert_eq!(free.declared_type, DeclaredType::Stated("u32".to_string()), "the return type");
        assert_eq!(free.params.len(), 1);
        assert_eq!(free.params[0].name, "w");
        assert_eq!(free.params[0].position, 0);
        assert_eq!(free.params[0].declared_type, DeclaredType::Stated("&Widget".to_string()));
    }

    /// R3 for the fact that lost data before: a field's declared type. And the
    /// other half of `DeclaredType` being total — a struct states no type of its
    /// own, and that is a reading, not a failure.
    #[test]
    fn a_declared_type_reaches_the_fact_and_an_absent_one_is_stated_as_absent() {
        let facts = facts("m", ONE_OF_EACH);
        let by = |fqn: &str| {
            facts.symbols.iter().find(|s| s.fqn.as_str() == fqn).unwrap_or_else(|| {
                panic!("{fqn} is missing from {:?}", fqns(&facts));
            })
        };

        assert_eq!(
            by("rust·p·m·Widget·width·field").declared_type,
            DeclaredType::Stated("u32".to_string())
        );
        assert_eq!(by("rust·p·m·MAX·item").declared_type, DeclaredType::Stated("u32".to_string()));
        assert_eq!(
            by("rust·p·m·Widget·item").declared_type,
            DeclaredType::Unstated,
            "a struct states no type of its own; that is a fact, not a miss"
        );
    }

    /// An inline `mod` extends the module path, so a declaration inside one is
    /// named the same way whether it lives in `a/b.rs` or in `mod b { }` inside
    /// `a.rs`. Two spellings of one symbol would never merge (spec §2).
    #[test]
    fn an_inline_module_extends_the_module_path_of_what_it_contains() {
        let facts = facts("a", "pub mod b { pub fn f() {} pub mod c { pub fn g() {} } }");
        let mut got = fqns(&facts);
        got.sort_unstable();
        assert_eq!(
            got,
            vec![
                "rust·p·a::b::c·g·item",
                "rust·p·a::b·c·mod",
                "rust·p·a::b·f·item",
                "rust·p·a·b·mod",
                // The file itself, module `a`. The inline `mod b { }` still
                // declares `b`, because b's body IS here.
                "rust·p·a·mod",
            ]
        );
    }

    /// At the crate root the module segment is empty, which the grammar drops.
    ///
    /// The file's own module is the counter-example beside it: a crate root is
    /// the one file no `mod` declaration names, so its identity is minted under
    /// the reserved `crate` segment from its own file stem — which is exactly
    /// why `CRATE_ROOT` is a reserved word no declaration can mint.
    #[test]
    fn a_declaration_at_the_crate_root_carries_no_module_segment() {
        let facts = facts("", "pub fn main() {}");
        let mut got = fqns(&facts);
        got.sort_unstable();
        assert_eq!(got, vec!["rust·p·crate·fixture·mod", "rust·p·main·item"]);
    }

    /// `impl Trait for (A, B)` names no type, so its members have no identity in
    /// this grammar. A missing symbol is the correct outcome; naming them as if
    /// they were free items of the module would mint an identity no use site
    /// could ever mint, which is a wrong edge and worse than a missing one (R4).
    /// The body is still walked, so nothing inside it is lost.
    #[test]
    fn a_member_of_an_unnameable_impl_is_missing_rather_than_misnamed() {
        let facts = facts(
            "m",
            "pub trait Draw { fn draw(&self); }\nimpl Draw for (u32, u32) { fn draw(&self) { helper(); } }",
        );
        let got = fqns(&facts);
        assert!(
            !got.contains(&"rust\u{b7}p\u{b7}m\u{b7}draw\u{b7}item"),
            "the trait-impl method was named as a free item of the module: {got:?}"
        );
        assert_eq!(
            got.iter().filter(|f| f.ends_with("draw\u{b7}item")).count(),
            1,
            "only the trait's own declaration can be named; got {got:?}"
        );
        assert_eq!(
            facts.references.iter().filter(|r| r.kind == RefKind::Calls).count(),
            1,
            "the body is still walked, so `helper()` is not lost"
        );
    }

    /// Visibility and the doc comment are facts R8 and "what is this module's
    /// surface" read, and both are stated by the source rather than inferred.
    /// A restricted scope is kept verbatim, because narrowing `pub(in a::b)` to a
    /// flag would discard which scope was meant.
    #[test]
    fn a_declarations_visibility_and_docstring_are_read_as_the_source_states_them() {
        let facts = facts(
            "m",
            "/// One.\n/// Two.\n#[derive(Debug)]\npub struct A;\npub(crate) struct B;\npub(super) struct C;\nstruct D;\n",
        );
        let by = |name: &str| {
            facts.symbols.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{name}"))
        };

        assert_eq!(by("A").visibility, Visibility::Public);
        assert_eq!(by("B").visibility, Visibility::Crate);
        assert_eq!(by("C").visibility, Visibility::Restricted("super".to_string()));
        assert_eq!(by("D").visibility, Visibility::Private);
        assert_eq!(
            by("A").docstring.as_deref(),
            Some("/// One.\n/// Two."),
            "an attribute between the comment and the item does not end the run"
        );
        assert_eq!(by("B").docstring, None, "no comment is not an empty comment");
    }

    // ── independent counters ─────────────────────────────────────────────────
    //
    // These are written from the OTHER side: they walk the same tree and count
    // nodes by tree-sitter kind, knowing nothing about `Walk`, its scopes or its
    // fqns. A producer that checks its own output proves only that it is
    // self-consistent; this is what notices a dropped fact.

    fn parse(text: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("the rust grammar loads");
        parser.parse(text, None).expect("tree-sitter returns a tree")
    }

    /// The node kinds that ARE a declaration, listed here and nowhere else.
    const DECLARATION_KINDS: &[&str] = &[
        "function_item",
        "function_signature_item",
        "struct_item",
        "union_item",
        "enum_item",
        "enum_variant",
        "trait_item",
        "type_item",
        "associated_type",
        "const_item",
        "static_item",
        "mod_item",
        "macro_definition",
        "field_declaration",
    ];

    use std::collections::BTreeSet;

    use crate::indexer::corpus_rust_sources as repo_rust_sources;

    /// True for a node the walk deliberately does not read: an attribute's
    /// contents are an unparsed token tree.
    fn inside_an_attribute(node: tree_sitter::Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(n) = current {
            if matches!(n.kind(), "attribute_item" | "inner_attribute_item") {
                return true;
            }
            current = n.parent();
        }
        false
    }

    /// True for a declaration whose enclosing `impl` is on a type with no name
    /// — a tuple, a slice, a unit. There is no type segment to build a member
    /// identity from, so no such declaration is nameable. Stated here in
    /// tree-sitter terms rather than by asking the walk.
    fn inside_an_unnameable_impl(node: tree_sitter::Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(n) = current {
            if n.kind() == "impl_item" {
                return !n.child_by_field_name("type").is_some_and(|t| {
                    matches!(
                        t.kind(),
                        "type_identifier" | "scoped_type_identifier" | "generic_type"
                    )
                });
            }
            current = n.parent();
        }
        false
    }

    /// Declarations, counted by kind, with no knowledge of `Walk`.
    fn count_declarations(root: tree_sitter::Node<'_>) -> usize {
        let mut count = 0;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !inside_an_attribute(node) && !inside_an_unnameable_impl(node) {
                if DECLARATION_KINDS.contains(&node.kind())
                    && node.child_by_field_name("name").is_some()
                    // A `mod` declares a module only when the module's BODY is
                    // here. `mod x;` names a module living in another file, and
                    // that file declares it — stated in tree-sitter terms, from
                    // the grammar, so this side stays independent of the walk.
                    && !(node.kind() == "mod_item" && node.child_by_field_name("body").is_none())
                {
                    count += 1;
                }
                if node.kind() == "ordered_field_declaration_list" {
                    let mut cursor = node.walk();
                    count += node.children_by_field_name("type", &mut cursor).count();
                }
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        count
    }

    /// The node kinds that ARE a use site, listed here and nowhere else.
    ///
    /// This is the plan's step-4 set — call, member access, path use,
    /// construction — plus macro invocation, which is a use of the macro. A bare
    /// `identifier` in expression position is NOT here: without scope analysis
    /// the walk cannot tell a local variable read from a const read, and the
    /// rung that could is step 5's. That is a scoping decision, not an
    /// oversight, and both sides of this count apply it.
    const USE_SITE_KINDS: &[&str] = &[
        "call_expression",
        "macro_invocation",
        "struct_expression",
        "field_expression",
        "scoped_identifier",
        "type_identifier",
        "scoped_type_identifier",
        "generic_type",
    ];

    /// Parent kinds that make a node a SEGMENT of a longer path rather than a
    /// use site of its own. `crate::db::PgStore::connect` names one thing, not
    /// four.
    const PATH_PARENTS: &[&str] = &[
        "scoped_identifier",
        "scoped_type_identifier",
        "generic_type",
        "qualified_type",
        "bracketed_type",
        "use_declaration",
        "use_as_clause",
        "use_list",
        "scoped_use_list",
        "use_wildcard",
    ];

    /// True when a node is named by its parent rather than naming something
    /// itself: the callee of a call, the macro of an invocation, the type of a
    /// construction, the `name` of any declaration.
    fn is_named_by_its_parent(node: tree_sitter::Node<'_>) -> bool {
        let Some(parent) = node.parent() else {
            return false;
        };
        let is_field =
            |field: &str| parent.child_by_field_name(field).is_some_and(|c| c.id() == node.id());
        match parent.kind() {
            "call_expression" | "generic_function" => is_field("function"),
            "macro_invocation" => is_field("macro"),
            _ => is_field("name"),
        }
    }

    /// True when a node sits inside something the walk does not read: an
    /// attribute's token tree, or a `use` path (which is an Import, not a
    /// reference).
    fn inside_unread_text(node: tree_sitter::Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(n) = current {
            if matches!(
                n.kind(),
                "attribute_item"
                    | "inner_attribute_item"
                    | "use_declaration"
                    | "extern_crate_declaration"
            ) {
                return true;
            }
            current = n.parent();
        }
        false
    }

    /// Use sites, counted by kind, with no knowledge of `Walk` and none at all
    /// of resolution.
    fn count_use_sites(root: tree_sitter::Node<'_>) -> usize {
        let mut count = 0;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let parent_kind = node.parent().map(|p| p.kind()).unwrap_or_default();
            // A GLOB IMPORT is a use site of the module it globs, and the
            // grammar has a node for exactly that. Counting `use_wildcard`
            // rather than reading the text keeps this side derived from the
            // parse, and gets `use a::{b::*, c::*}` right for free — two
            // wildcards, two modules entered, which is what the walk emits.
            //
            // A NAMED import is not counted, because the walk deliberately
            // emits none: `use a::b;` and `use a::b::C;` are the same shape and
            // only one of them names a module.
            if node.kind() == "use_wildcard" {
                count += 1;
            }
            if USE_SITE_KINDS.contains(&node.kind())
                && !is_named_by_its_parent(node)
                && !PATH_PARENTS.contains(&parent_kind)
                && !inside_unread_text(node)
            {
                count += 1;
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        count
    }

    /// Step 3's load-bearing check, over this repo's own rust rather than a
    /// fixture: if the walk emits fewer symbols than there are declaration nodes
    /// it is dropping declarations, and if it emits more it is inventing them.
    ///
    /// EXACTLY ONE symbol per file has no declaration node, and it is the file's
    /// OWN module. A file is a module in Rust, but nothing in the file says so —
    /// there is no AST node to count, because the declaration is the file's
    /// existence. `count_declarations` walks tree-sitter nodes, so it
    /// structurally cannot see this one; the `+ 1` is that fact, not slack in
    /// the check.
    ///
    /// A pseudo-kind in `DECLARATION_KINDS` would have been the wrong repair:
    /// the counter's whole value is that it is derived from the GRAMMAR and not
    /// from the walk, and teaching it about a symbol the grammar does not
    /// contain would make the two sides agree by construction.
    /// Every `cfg`-gated declaration, counted off the TREE — the second term of
    /// the conservation identity.
    ///
    /// A gated declaration yields an ARM, and every arm of one name shares ONE
    /// callable. So the extra nodes are the DISTINCT gated names per scope, not
    /// the number of gated declarations — two arms of `provision_status` add
    /// one node between them, not two. Getting that wrong was the first cut of
    /// this term, and the corpus said so in seven files at once.
    ///
    /// INDEPENDENT BY CONSTRUCTION, which is the whole value of the property.
    /// It reads node kinds and sibling order straight off the grammar and never
    /// asks `Walk::condition_of` anything; a shared reader would make the two
    /// sides agree because they are one derivation, and a conservation property
    /// that cannot disagree measures nothing.
    fn count_gated(root: tree_sitter::Node<'_>, text: &str) -> usize {
        let mut callables: std::collections::BTreeSet<(usize, String)> =
            std::collections::BTreeSet::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let mut cursor = node.walk();
            let children: Vec<tree_sitter::Node<'_>> = node.named_children(&mut cursor).collect();
            for (at, child) in children.iter().enumerate() {
                stack.push(*child);
                if child.kind() != "attribute_item" {
                    continue;
                }
                let raw = &text[child.byte_range()];
                let Some(inner) = raw.strip_prefix("#[cfg(").and_then(|r| r.strip_suffix(")]"))
                else {
                    continue;
                };
                // `cfg(test)` is not a variant — the inline test boundary
                // already owns it, and 405 of them would say nothing a reader
                // wants.
                if inner == "test" {
                    continue;
                }
                // The attribute gates whatever comes NEXT, and only a
                // declaration the other side counts produces a node to pair
                // with.
                let gated = children.get(at + 1).filter(|n| {
                    DECLARATION_KINDS.contains(&n.kind())
                        && n.child_by_field_name("name").is_some()
                        && !(n.kind() == "mod_item" && n.child_by_field_name("body").is_none())
                });
                // Keyed on the ENCLOSING node and the name: that pair is what
                // decides whether two arms are arms of one callable, and it is
                // read off the tree rather than from the walk.
                // A MEMBER OF A TYPE IS NOT SPLIT, and the walk says so in as
                // many words: an arm of `Widget::pg` would need a segment the
                // grammar has no shape for, so `Walk::split_into_variant` leaves it
                // whole rather than keying it on a string no reader could
                // decompose (R4). `logger/src/writer.rs` is the corpus case —
                // seven gated enum variants, fields and methods, none of them
                // free items. This side excludes them on the same grammatical
                // ground rather than by consulting the walk.
                // `declaration_list` is BOTH a `mod` body and an `impl` body, so
                // the kind alone cannot say it — the PARENT does. A fn in a mod
                // is a free item and is split; a fn in an impl is a member and
                // is not.
                let free = match node.kind() {
                    "field_declaration_list"
                    | "enum_variant_list"
                    | "ordered_field_declaration_list" => false,
                    "declaration_list" => node.parent().is_some_and(|p| p.kind() == "mod_item"),
                    _ => true,
                };
                if let Some(declaration) = gated.filter(|n| free && !inside_an_unnameable_impl(**n))
                    && let Some(named) = declaration.child_by_field_name("name")
                {
                    callables.insert((node.id(), text[named.byte_range()].to_string()));
                }
            }
        }
        callables.len()
    }

    #[test]
    fn the_symbol_count_equals_an_independent_count_of_declaration_nodes() {
        let mut disagreements = Vec::new();
        for (path, text) in repo_rust_sources() {
            let facts = read(&Source { package: "p", module: "m", path: &path, text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            // A CONSERVATION IDENTITY, not an equality, and the extra term is
            // the point. A `cfg`-gated declaration yields TWO nodes: the
            // callable a use site mints, and the arm that implements it. The
            // flat scan counts the declaration once, so the two sides differ by
            // exactly the number of gated declarations — counted here the same
            // independent way, off the tree rather than off the walk.
            //
            // No tolerance. A tolerance is where a double-count hides, which is
            // what this property exists to prevent.
            let root = parse(&text);
            let expected =
                count_declarations(root.root_node()) + 1 + count_gated(root.root_node(), &text);
            if facts.symbols.len() != expected {
                disagreements.push(format!(
                    "{path}: walk produced {}, independent count says {}",
                    facts.symbols.len(),
                    expected
                ));
            }
        }
        assert!(
            disagreements.is_empty(),
            "{} of the corpus files disagree:\n{}",
            disagreements.len(),
            disagreements.join("\n")
        );
    }

    // ── step 4: every reference ──────────────────────────────────────────────

    /// A declaration inside a function body is named under that function.
    ///
    /// The AST always carried the parent — the walk is inside `Walk::function`
    /// descending into its own children, and `Scope::from` was already being
    /// set from it, which is why a REFERENCE in a body attributed correctly.
    /// The declaration did not, because `declare` reads `container` and nothing
    /// told the container it had gone inside a body.
    ///
    /// MEASURED: three `static RE` in three `fn`s of one file minted ONE
    /// identity; A7 reported 708 such collisions.
    ///
    /// MUTATION: drop `inner.fn_scope.push(..)` from `Walk::function` — the two
    /// `RE`s collapse onto one fqn again.
    #[test]
    fn a_declaration_inside_a_function_body_is_named_under_that_function() {
        let facts = facts(
            "m",
            "pub fn a() -> u32 { static RE: u32 = 1; RE }\n\
             pub fn b() -> u32 { static RE: u32 = 2; RE }\n\
             pub static RE: u32 = 3;\n",
        );
        let minted: Vec<&str> =
            facts.symbols.iter().filter(|s| s.name == "RE").map(|s| s.fqn.as_str()).collect();
        assert_eq!(minted.len(), 3, "three declarations: {minted:?}");
        let distinct: std::collections::BTreeSet<&&str> = minted.iter().collect();
        assert_eq!(distinct.len(), 3, "three declarations, three identities: {minted:?}");
        assert!(
            minted.contains(&"rust·p·m::fn::a·RE·item"),
            "the local is named under its function: {minted:?}"
        );
        assert!(
            minted.contains(&"rust·p·m·RE·item"),
            "and the module-level one is untouched: {minted:?}"
        );

        // AND WHAT THIS DOES NOT DO. `RE` in the body is a BARE IDENTIFIER,
        // and `Walk::use_site` deliberately emits no reference for one — the
        // walk cannot tell a local variable read from a const read without
        // scope analysis, and emitting every one would bury the graph. So these
        // three declarations are now three distinct nodes with NO inbound edge,
        // where before they were one node with the wrong answer.
        //
        // That is the trade, stated rather than glossed: R4 ranks a missing
        // edge above a wrong one, and "where is `RE` defined" answering with
        // whichever file-position won is the wrong one. Closing the other half
        // needs identifier reads, which is a separate decision.
        assert!(
            !facts.references.iter().any(|r| matches!(
                &r.target,
                Resolution::Unresolved { evidence, .. } if evidence.name == "RE"
            )),
            "a bare identifier read is not a use site today; if that changes, this test is \
             where the other half of the merge gets asserted"
        );
    }

    /// A fully-qualified path to another crate resolves with NO import (R5).
    ///
    /// `serde_json::json!(..)` is a complete use; requiring a `use` to place it
    /// is requiring something Rust does not. MEASURED: 268 `serde_json::json`,
    /// 57 `tracing::warn`, 39 `uuid::Uuid::parse_str` sat unresolved.
    ///
    /// MUTATION: set `paths_name_packages: false` — all three go back to
    /// `NoImportInScope`. The three NEGATIVE cases below are the guard: without
    /// them the rung mints a library node for a local path, a crate-relative
    /// one, and a sibling crate this scan owns.
    #[test]
    fn a_fully_qualified_path_to_another_crate_needs_no_import() {
        use crate::indexer::resolve::{World, resolve};
        let text = "pub fn go() {\n    let _ = serde_json::json!({});\n    tracing::warn!(\"x\");\n\
                    let _ = crate::helper();\n    let _ = senseid::thing();\n}\n\
                    pub fn helper() -> u32 { 0 }\n";
        let facts = read(&Source { package: "sensei-cli", module: "m", path: "src/m.rs", text })
            .expect("it parses");
        let first_party: BTreeSet<String> =
            ["sensei-cli".to_string(), "senseid".to_string()].into_iter().collect();
        let placed = resolve(
            facts,
            &GRAMMAR,
            &World {
                first_party: &first_party,
                first_party_members: &BTreeSet::new(),
                declared_members: &BTreeSet::new(),
                returns: &std::collections::BTreeMap::new(),
                scanned: &BTreeSet::new(),
            },
        );
        let resolved: Vec<String> = placed
            .references
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => Some(fqn.to_string()),
                _ => None,
            })
            .collect();
        assert!(
            resolved.iter().any(|f| f == "lib·serde_json·json"),
            "an external crate path is a complete use: {resolved:?}"
        );
        assert!(
            resolved.iter().any(|f| f == "lib·tracing·warn"),
            "and so is a macro path: {resolved:?}"
        );
        // The guards. A sibling crate THIS SCAN OWNS is not a library, and a
        // crate-relative path is not one either.
        assert!(
            !resolved.iter().any(|f| f.starts_with("lib·senseid")),
            "a first-party package is ours however it is spelled: {resolved:?}"
        );
        assert!(
            !resolved.iter().any(|f| f.starts_with("lib·crate")),
            "`crate::` is a root word, not a package: {resolved:?}"
        );
    }

    /// `Self` names the enclosing type, not a type called `Self`.
    ///
    /// MEASURED: 33 references named `Self` and could reach nothing, because
    /// the walk minted an identity for a type no declaration carries.
    ///
    /// MUTATION: drop the `Self` arm from `Walk::concrete` — `Self::new()` goes
    /// back to naming `Self` and matches nothing.
    #[test]
    fn self_names_the_enclosing_type_and_not_a_type_called_self() {
        let facts = facts(
            "m",
            "pub struct Widget { w: u32 }\n\
             impl Widget {\n\
             \x20   pub fn new() -> Self { Self { w: 0 } }\n\
             \x20   pub fn make() -> Self { Self::new() }\n\
             }\n",
        );
        let named: Vec<String> = facts
            .references
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => Some(fqn.to_string()),
                Resolution::Unresolved { evidence, .. } => {
                    evidence.identities().next().map(Fqn::to_string)
                }
            })
            .collect();
        assert!(
            !named.iter().any(|f| f.contains("\u{00B7}Self\u{00B7}")),
            "nothing may be filed under a type called `Self`: {named:?}"
        );
        assert!(
            named.iter().any(|f| f.ends_with("Widget\u{00B7}new\u{00B7}item")),
            "`Self::new()` reaches Widget::new: {named:?}"
        );
    }

    /// **A GLOB ROOTED IN THIS PACKAGE IS THIS FILE NAMING A MODULE** (S6).
    ///
    /// `use super::*` inside `db::pg_store::graph` is the file writing down
    /// `db::pg_store`. Resolving that root needs no export list, no table and
    /// no other file — only the module the walk is already standing in. It is
    /// the one shape the spec's §1 measured at 17.6% of every member reference
    /// the barrier resolved, and dropping the table without it cost 3,389
    /// references in one commit.
    ///
    /// A CANDIDATE, NOT A NAMED. The glob says where the name COULD come from,
    /// not that it does — `PgStore` might be declared in the prelude, in
    /// another glob, or nowhere. That is an inference about this file rather
    /// than a statement by it, so it still needs a declaration to agree (R4,
    /// and §9: `Named` is not a licence). The distinction is the whole of S7
    /// and this is the case that tests it from the weak side.
    ///
    /// `crate::` and `self::` roots are the same fact spelled differently and
    /// are handled with it; a BARE glob (`use serde::*`) names a package we do
    /// not open and answers nothing (R5).
    ///
    /// MUTATION: return `None` from `glob_rooted_here` — the member falls back
    /// to the impl block's own module and the first assertion goes red naming
    /// the module it landed in instead.
    #[test]
    fn a_glob_rooted_in_this_package_names_the_module_its_names_come_from() {
        // `graph.rs` declares nothing and imports nothing by name. Its ONLY
        // statement about `PgStore` is the glob root.
        let facts = facts(
            "db::pg_store::graph",
            "use super::*;\n\
             pub fn wire(pg: &PgStore) -> u32 { pg.upsert() }\n",
        );

        let considered: Vec<String> = facts
            .references
            .iter()
            .filter(|r| r.kind == RefKind::Calls)
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { evidence, .. } => Some(evidence),
                Resolution::Resolved { .. } => None,
            })
            .flat_map(Evidence::identities)
            .map(Fqn::to_string)
            .collect();
        assert_eq!(
            considered,
            vec!["rust·p·db::pg_store·PgStore·upsert·item".to_string()],
            "`super` from `db::pg_store::graph` is `db::pg_store`, which is where the member \
             is named — the file said so and no table was asked"
        );

        // AND IT IS THE WEAK GRADE. A glob says where a name COULD come from.
        let graded: Vec<&str> = facts
            .references
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { evidence, .. } => Some(evidence),
                Resolution::Resolved { .. } => None,
            })
            .flat_map(|e| e.saw.iter())
            .filter_map(|o| match o {
                Observation::Named(_) => Some("Named"),
                Observation::Candidate(_) => Some("Candidate"),
                _ => None,
            })
            .collect();
        assert!(
            !graded.contains(&"Named"),
            "a glob is an inference about this file, not a statement by it, so nothing it \
             yields may become an edge unaided: {graded:?}"
        );
    }

    /// **A DECLARATION INSIDE A FUNCTION BODY IS A LOCAL ITEM, NEVER A MEMBER
    /// OF THE TYPE WHOSE `impl` BLOCK THE FUNCTION SITS IN.**
    ///
    /// `module_here` has named a local under its enclosing function since the
    /// function-body rule landed — but only for a FREE function. Inside a
    /// method the container is still `Container::Type`, and `declare`'s
    /// member arm reads the container's module and never asks whether it is
    /// standing in a body. So `type Row = (..)` inside `PgStore::sweep` minted
    /// `PgStore·Row·item`: a MEMBER of `PgStore`, which it is not. `PgStore`
    /// declares no `Row`, nothing can reach one, and four such aliases in four
    /// method bodies are one node.
    ///
    /// MEASURED over this repository: four `type Row` aliases in
    /// `db/pg_store/{metrics,reasons,sessions}.rs`, all four inside method
    /// bodies, all four on one identity. It sat on the known-collision list
    /// as "the function-body rule reaches free functions; a method body is the
    /// remaining shape".
    ///
    /// The free-function half is asserted beside it, because a fix that named
    /// locals correctly in methods by breaking free functions would otherwise
    /// pass.
    ///
    /// MUTATION: drop the `fn_scope` check from `Walk::declare` — the two
    /// aliases collapse onto `Holder·Row·item` and `Row·item`, and the first
    /// assertion names what it got.
    #[test]
    fn a_local_declared_in_a_method_body_is_not_a_member_of_the_enclosing_type() {
        let facts = facts(
            "db",
            "pub struct Holder;\n\
             impl Holder {\n\
             \x20   pub fn sweep(&self) -> u32 {\n\
             \x20       type Row = (u32, u32);\n\
             \x20       0\n\
             \x20   }\n\
             }\n\
             pub fn free() -> u32 {\n\
             \x20   type Row = (u32, u32);\n\
             \x20   0\n\
             }\n",
        );

        let mut rows: Vec<&str> =
            fqns(&facts).into_iter().filter(|f| f.ends_with("Row·item")).collect();
        rows.sort_unstable();
        assert_eq!(
            rows,
            vec!["rust·p·db::fn::free·Row·item", "rust·p·db::fn::sweep·Row·item"],
            "each local is named under the FUNCTION that declares it — two bodies, two \
             identities, and neither is a member of `Holder`"
        );

        // And the method itself is still a member, which is the half a careless
        // fix would take with it.
        assert!(
            fqns(&facts).contains(&"rust·p·db·Holder·sweep·item"),
            "the METHOD is a member of its type; only what its BODY declares is not: {:?}",
            fqns(&facts)
        );
    }

    /// **A `cfg`-GATED DECLARATION IS A VARIANT, AND BOTH ARMS ARE KEPT.**
    ///
    /// `#[cfg(feature = "x")] fn f` and `#[cfg(not(feature = "x"))] fn f` are
    /// two BODIES of one function. Both are in the codebase — which is what
    /// this indexer describes — and exactly one is in any given binary. A
    /// graph that kept one arm would be describing a build; a graph that
    /// merged them keeps a node whose span is whichever arm the writer reached
    /// last.
    ///
    /// **THE COST OF MERGING IS A WRONG EDGE, not untidiness.** The two bodies
    /// have DIFFERENT CALLEES. Collapsed onto one node, the call graph asserts
    /// that the non-embedded build calls `merge` — false in half the builds,
    /// and R4 ranks a wrong edge below a missing one. That is why the variants
    /// have to be real NODES: a property list on one node has nothing to hang
    /// outbound edges off.
    ///
    /// So one declaration becomes a shared CALLABLE plus one variant per arm:
    /// callers mint the callable and nothing else, so the merge contract is
    /// untouched, and each body's edges hang off its own variant.
    ///
    /// MEASURED over this repository: 52 non-test `cfg`-gated declarations,
    /// of which one pair shares a name — `api/handlers/model_provisioning.rs`,
    /// which is where this test's fixture comes from.
    ///
    /// MUTATION: return the plain identity from `Walk::variant_of` — the two
    /// arms collapse onto one node and the `Variant` relations vanish.
    #[test]
    fn a_cfg_gated_declaration_keeps_both_arms_under_one_callable() {
        let facts = facts(
            "h",
            "#[cfg(feature = \"emb\")]\n\
             pub fn status() -> u32 { merge() }\n\
             #[cfg(not(feature = \"emb\"))]\n\
             pub fn status() -> u32 { 0 }\n",
        );

        let mut minted: Vec<&str> =
            fqns(&facts).into_iter().filter(|f| f.contains("status")).collect();
        minted.sort_unstable();
        assert_eq!(
            minted,
            vec![
                "rust·p·h·status·cfg:feature=emb·item",
                "rust·p·h·status·cfg:not(feature=emb)·item",
                "rust·p·h·status·item",
            ],
            "ONE callable, which is all a caller can spell, and one variant per arm carrying \
             the condition the source wrote"
        );

        // THE PROPERTY THAT MATTERS. `merge()` is called by the embedded arm
        // and by nothing else, so its edge hangs off THAT variant — never off
        // the callable, which would claim every build makes the call.
        let calls: Vec<String> = facts
            .references
            .iter()
            .filter(|r| r.kind == RefKind::Calls)
            .map(|r| r.from.as_str().to_string())
            .collect();
        assert_eq!(
            calls,
            vec!["rust·p·h·status·cfg:feature=emb·item".to_string()],
            "the call belongs to the arm that makes it"
        );

        // And each arm says which callable it is an arm OF.
        let variants: Vec<String> = facts
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Variant)
            .map(|r| match &r.parent {
                Resolution::Resolved { fqn, .. } => format!("{} -> {fqn}", r.child.as_str()),
                Resolution::Unresolved { evidence, .. } => evidence.name.clone(),
            })
            .collect();
        assert_eq!(
            variants,
            vec![
                "rust·p·h·status·cfg:feature=emb·item -> rust·p·h·status·item".to_string(),
                "rust·p·h·status·cfg:not(feature=emb)·item -> rust·p·h·status·item".to_string(),
            ],
            "each arm points at the callable it implements"
        );
    }

    // ── anchoring a member to its type's module ──────────────────────────

    /// **A TYPE THIS FILE NEVER NAMES IS NOT THE EXTERNAL BOUNDARY** (R5, §2).
    ///
    /// Externality comes from the IMPORT and from nothing else. `Home::NotOurs`
    /// used to be the repo-wide table saying "no first-party declaration answers
    /// to this name", and the walk read that absence as a VERDICT — so every
    /// receiver whose type the scan had simply not placed was filed as
    /// `Reason::ExternalBoundary`, which `Reason::casts_doubt` EXCLUDES. The
    /// doubt column then FALLS as the walk learns less, which is exactly
    /// backwards: a type nobody has placed is an open question, not a closed
    /// one. It matters now because the table is being deleted, and with it the
    /// only producer of `NotOurs` — without this split every unplaced receiver
    /// would land outside the doubt column on the day the table goes.
    ///
    /// A PAIR, because either half alone is satisfied by collapsing the two
    /// cases onto one answer. The first half says "the file says nothing" must
    /// be a miss that casts doubt, and must carry the TYPE as material for a
    /// later pass. The second says a type the file DID place — by importing it
    /// from a library — must still read as the boundary it is.
    ///
    /// MUTATION: restore `other => other` in `home_of`'s table arm, so the
    /// table's empty answer is `NotOurs` again, and the first half goes red
    /// with `("ExternalBoundary", false)`.
    #[test]
    fn a_type_this_file_never_names_is_not_the_boundary_but_an_imported_library_type_is() {
        // The reason on the miss named `wide`, and whether its evidence names
        // the TYPE the miss is about. Read off the WALK, with no ladder: this
        // is a claim about what one file on its own says (R7).
        let miss = |text: &str| -> (String, bool) {
            let facts = facts("m", text);
            let (reason, evidence) = facts
                .references
                .iter()
                .find_map(|r| match &r.target {
                    Resolution::Unresolved { reason, evidence } if evidence.name == "wide" => {
                        Some((reason, evidence))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no miss named `wide` in {:?}", facts.references));
            let named_the_type = evidence.saw.iter().any(|o| {
                matches!(o, crate::indexer::facts::Observation::UnplacedType(t) if t == "Widget")
            });
            (format!("{reason:?}"), named_the_type)
        };

        assert_eq!(
            miss("pub fn go(w: Widget) -> u32 { w.wide() }\n"),
            ("NoImportInScope".to_string(), true),
            "nothing in this file says where `Widget` lives — no declaration, no import — so \
             the type is UNPLACED, not placed outside, and the evidence carries it so a later \
             pass has the one fact this miss is about"
        );

        assert_eq!(
            miss("use serde_json::Widget;\npub fn go(w: Widget) -> u32 { w.wide() }\n"),
            ("ExternalBoundary".to_string(), false),
            "the file DID say where `Widget` lives — a package we never open — and that is the \
             boundary, which is a verdict and not doubt about anything of ours"
        );
    }

    /// The fix, and the defect it replaces, in one test.
    ///
    /// `impl PgStore` sits in `db::pg_store::personas`; `PgStore` is declared in
    /// `db::pg_store`. Rust reaches the method as
    /// `db::pg_store::PgStore::forge` — the impl block's location is not part of
    /// the path — so the member is named in the TYPE's module.
    ///
    /// MEASURED before this: 634 of 5,186 members sat under a module their type
    /// does not live in, `PgStore` alone across 24 files, and 2,538 unresolved
    /// **AN `impl` BLOCK THE FILE CANNOT PLACE NAMES ITS MEMBERS WHERE THE
    /// BLOCK SITS**, and that is a MISS rather than a guess.
    ///
    /// What is left of a test whose other half was the barrier's effect. That
    /// half read the same fixture twice — once with an empty type table, once
    /// with `PgStore` in it — and asserted the members moved. The table is gone
    /// (S5), so the comparison has nothing to compare and the test was deleted
    /// rather than adjusted.
    ///
    /// THIS half survives because it is the behaviour that REMAINS, and after
    /// S5 it is the only behaviour: `impl PgStore` in `db::pg_store::personas`,
    /// in a file that neither declares `PgStore` nor imports it by a
    /// package-rooted path, has nothing to say about where the type lives. The
    /// block's own module stands. The declaration and a caller elsewhere then
    /// mint different strings and never meet — which is a known, measured and
    /// RATCHETED defect, not a surprise: see
    /// `resolve::tests::every_ownership_edge_points_at_a_type_declared_somewhere_in_its_own_package`,
    /// 655 of 5,474 ownership edges. Counted in aggregate there; pinned as a
    /// single case here, because a ratchet over a corpus proves nothing about
    /// one file.
    ///
    /// MUTATION that must break it: make `impl_block`'s `None` arm invent a
    /// module — anything but `scope.module.clone()`.
    #[test]
    fn an_impl_block_the_file_cannot_place_names_its_members_where_the_block_sits() {
        let facts = facts(
            "db::pg_store::personas",
            "impl PgStore {\n    pub fn forge(&self) -> u32 { 0 }\n}\n\
             pub fn caller(pg: &PgStore) -> u32 { pg.forge() }\n",
        );
        assert!(
            fqns(&facts).contains(&"rust·p·db::pg_store::personas·PgStore·forge·item"),
            "this file says nothing about where `PgStore` lives, so the member stays where its \
             impl block is — a miss the ownership ratchet counts, never a guess at a module \
             the file never named (R4): {:?}",
            fqns(&facts)
        );
    }

    /// A name two modules of one package declare has NO home, and nothing
    /// moves. Picking one of two would mint a wrong identity, which R4 ranks
    /// below no identity — and 354 members of this corpus are in that case.
    #[test]
    fn a_type_name_declared_in_two_modules_has_no_home_and_anchors_nothing() {
        let declare = |module: &str| Symbol {
            fqn: fqn::define(&Form::Item {
                lang: Language::Rust,
                package: "senseid",
                module,
                name: "Config",
                reach: Reach::Item,
            })
            .expect("well formed"),
            kind: SymbolKind::Struct,
            name: "Config".to_string(),
            span: crate::indexer::facts::Span {
                start_line: 1,
                start_col: 0,
                end_line: 1,
                end_col: 1,
            },
            visibility: Visibility::Public,
            docstring: None,
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        };
        let one = declare("a");
        let two = declare("b");
        let homes = TypeHomes::of(vec![("senseid", &one), ("senseid", &two)]);
        assert_eq!(
            homes.lookup("senseid", "Config"),
            Home::Ambiguous,
            "two homes is not one home, and not no home either"
        );
        assert!(homes.is_empty(), "an ambiguous name is absent, not resolved to the first");

        // The same name in a DIFFERENT package is a different type and keeps
        // its home.
        let elsewhere = TypeHomes::of(vec![("senseid", &one), ("cli", &two)]);
        assert_eq!(elsewhere.lookup("senseid", "Config"), Home::Tabled { module: "a" });
        assert_eq!(elsewhere.lookup("cli", "Config"), Home::Tabled { module: "b" });
    }

    /// A2, and the load-bearing check of the whole rewrite. The legacy defect was a
    /// catch-all arm that returned early, so a use site it did not understand
    /// left no trace at all and the loss was invisible. This is what makes the
    /// loss visible: an independent count of use-site nodes over this repo's own
    /// rust, against the number of references the walk produced.
    #[test]
    fn the_reference_count_equals_an_independent_count_of_use_sites() {
        let mut disagreements = Vec::new();
        let mut total = 0usize;
        for (path, text) in repo_rust_sources() {
            let facts = read(&Source { package: "p", module: "m", path: &path, text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            let expected = count_use_sites(parse(&text).root_node());
            total += expected;
            if facts.references.len() != expected {
                disagreements.push(format!(
                    "{path}: walk produced {}, independent count says {}",
                    facts.references.len(),
                    expected
                ));
            }
        }
        assert!(total > 10_000, "the corpus produced only {total} use sites; that is not it");
        assert!(
            disagreements.is_empty(),
            "{} of the corpus files disagree, so references are being dropped:\n{}",
            disagreements.len(),
            disagreements.join("\n")
        );
    }

    /// A3's precondition. Every miss names its cause and carries the node kind
    /// that produced it, so a form nobody thought about is a NAMED bucket in the
    /// histogram rather than an absence.
    #[test]
    fn every_unresolved_reference_carries_a_reason_and_the_node_kind_it_came_from() {
        for (path, text) in repo_rust_sources().into_iter().take(40) {
            let facts = read(&Source { package: "p", module: "m", path: &path, text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            for reference in &facts.references {
                let Resolution::Unresolved { evidence, .. } = &reference.target else {
                    continue;
                };
                assert!(
                    !evidence.name.is_empty(),
                    "{path}: a miss with no name at {:?}",
                    reference.at
                );
                assert!(
                    !evidence.node_kind.is_empty(),
                    "{path}: a miss the histogram cannot name at {:?}",
                    reference.at
                );
            }
        }
    }

    /// The catch-all arm this rewrite exists to remove. A callee shape the walk
    /// has no rule for still yields a reference — one that says so, and says
    /// which node kind it was, rather than yielding nothing.
    #[test]
    fn a_callee_shape_with_no_rule_yields_a_named_miss_and_never_nothing() {
        let facts =
            facts("m", "fn f(t: (fn(), fn()), b: Box<dyn Fn()>) { (b)(); t.0(); [b][0](); }");
        let unhandled: Vec<&str> = facts
            .references
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { reason: Reason::UnhandledForm, evidence } => {
                    Some(evidence.node_kind.as_str())
                }
                _ => None,
            })
            .collect();

        assert!(
            unhandled.contains(&"parenthesized_expression"),
            "`(b)()` must name what it did not understand; got {unhandled:?}"
        );
        assert!(
            unhandled.contains(&"index_expression"),
            "`[b][0]()` must name what it did not understand; got {unhandled:?}"
        );
        assert_eq!(
            facts.references.iter().filter(|r| r.kind == RefKind::Calls).count(),
            3,
            "three calls, three references, however strange the callees"
        );
    }

    /// Deliberately broken source. tree-sitter still returns a tree, full of
    /// ERROR nodes; the walk must read what is there without panicking and
    /// without quietly emitting less than is there.
    #[test]
    fn malformed_source_neither_panics_nor_drops() {
        for text in [
            "fn f() { x.(); }",
            "fn f() { ().0(); }",
            "fn f() { let x = ",
            "impl {",
            "struct S { a: }",
            "fn f() { a.b.c().d[0].e(); }",
            "",
        ] {
            let facts = read(&Source { package: "p", module: "m", path: "src/m.rs", text })
                .unwrap_or_else(|e| panic!("`{text}`: {e:?}"));
            let expected = count_use_sites(parse(text).root_node());
            assert_eq!(
                facts.references.len(),
                expected,
                "`{text}`: the walk and the independent count disagree"
            );
        }
    }

    /// THE MERGE CONTRACT, exercised rather than asserted about itself. The
    /// declaration side and the use side are written by different code reading
    /// different nodes; this checks that for a symbol declared and used in one
    /// file, the identity the use site MINTED is byte-identical to the one the
    /// declaration minted. If they differ the two never merge, and no amount of
    /// resolution afterwards can fix it.
    #[test]
    fn a_use_site_mints_the_same_identity_the_declaration_did() {
        let facts = facts(
            "m",
            r#"
pub struct Widget { pub width: u32 }
impl Widget {
    pub fn new() -> Widget { Widget { width: 0 } }
    fn wide(&self) -> u32 { self.width }
}
pub fn free() -> u32 { helper() }
fn helper() -> u32 { 0 }
"#,
        );
        let declared: Vec<&str> = facts.symbols.iter().map(|s| s.fqn.as_str()).collect();
        let considered: Vec<&str> = facts
            .references
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { evidence, .. } => Some(evidence),
                Resolution::Resolved { .. } => None,
            })
            .flat_map(Evidence::identities)
            .map(Fqn::as_str)
            .collect();

        for expected in [
            // `self.width` inside an impl knows its receiver's type.
            "rust·p·m·Widget·width·field",
            // `Widget { .. }` is a construction of the declared struct.
            "rust·p·m·Widget·item",
            // `helper()` is a free call in the same module.
            "rust·p·m·helper·item",
        ] {
            assert!(
                considered.contains(&expected),
                "no use site minted {expected}; the use side considered {considered:?}"
            );
            assert!(
                declared.contains(&expected),
                "{expected} was minted by a use site but never by the declaration side, \
                 so the two would never merge"
            );
        }
    }

    /// A macro invocation is a distinct tree-sitter kind from a call, and this
    /// is the recorded decision about it: it IS a reference, of its own kind,
    /// pointing at the macro. What the macro EXPANDS to is not parsed, so uses
    /// inside its token tree are not facts this walk has and none are claimed.
    #[test]
    fn a_macro_invocation_is_a_reference_of_its_own_kind_and_its_body_is_not_read() {
        let facts = facts("m", r#"fn f() { println!("{}", self.hidden().call()); }"#);
        assert_eq!(facts.references.len(), 1, "one invocation, one reference");
        assert_eq!(facts.references[0].kind, RefKind::MacroInvokes);
        let Resolution::Unresolved { evidence, .. } = &facts.references[0].target else {
            panic!("the walk resolves nothing; that is step 5's job");
        };
        assert_eq!(evidence.name, "println");
        assert_eq!(evidence.node_kind, "macro_invocation");
    }

    /// A member access that is read and one that is written are different facts,
    /// and pattern detection reads the difference (R8).
    #[test]
    fn a_member_access_records_whether_it_is_read_or_written() {
        let facts = facts("m", "fn f(w: &mut W) { w.width = 1; let _ = w.height; }");
        let kinds: Vec<RefKind> = facts
            .references
            .iter()
            .filter(|r| matches!(r.kind, RefKind::Reads | RefKind::Writes))
            .map(|r| r.kind)
            .collect();
        assert!(kinds.contains(&RefKind::Writes), "`w.width = 1` writes; got {kinds:?}");
        assert!(kinds.contains(&RefKind::Reads), "`w.height` reads; got {kinds:?}");
    }

    /// Externality comes from the import, never from a symbol being absent from
    /// what has been scanned (spec §2, R6). The walk records the import verbatim
    /// and says which side of the boundary the specifier puts it on; it does not
    /// look at any other file to decide.
    #[test]
    fn an_import_is_recorded_with_what_it_binds_and_a_glob_claims_nothing() {
        let facts = facts(
            "m",
            "use std::collections::HashMap;\nuse serde_json::Value as Json;\nuse crate::db::{PgStore, Row};\nuse super::helper::*;\n",
        );
        let by = |path: &str| {
            facts.imports.iter().find(|i| i.path == path).unwrap_or_else(|| {
                panic!("{path} is missing from {:?}", facts.imports);
            })
        };

        assert_eq!(by("std::collections::HashMap").binds, Binding::Name("HashMap".to_string()));
        assert_eq!(
            by("std::collections::HashMap").origin,
            ImportOrigin::External { package: "std".to_string() }
        );
        assert_eq!(
            by("serde_json::Value as Json").binds,
            Binding::Name("Json".to_string()),
            "an alias binds the alias, not the original name"
        );
        assert_eq!(by("crate::db::PgStore").origin, ImportOrigin::Local);
        assert_eq!(by("crate::db::Row").binds, Binding::Name("Row".to_string()));
        assert_eq!(
            by("super::helper::*").binds,
            Binding::Glob,
            "a glob binds an unknown set, and must never read as proof that it bound a name"
        );
    }

    /// A3's shape, one step early: every reference the corpus produces is
    /// accounted for by exactly one reason, so the histogram sums to the whole
    /// and nothing sits outside it.
    ///
    /// It also records where step 4 leaves things. The walk resolves NOTHING —
    /// placing a name is the shared ladder's job (R7), so every reference here
    /// is `Unresolved`, and `Unplaced` is the bucket that says exactly that.
    /// Step 5 must empty that bucket; if it does not, the ladder is not doing
    /// its job and this test is where that shows.
    #[test]
    fn every_reference_is_accounted_for_by_exactly_one_reason() {
        use std::collections::BTreeMap;

        let mut histogram: BTreeMap<String, usize> = BTreeMap::new();
        let mut total = 0usize;
        for (path, text) in repo_rust_sources() {
            let facts = read(&Source { package: "p", module: "m", path: &path, text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            for reference in &facts.references {
                total += 1;
                match &reference.target {
                    Resolution::Resolved { fqn, .. } => panic!(
                        "{path}: the walk resolved {fqn} — resolution is step 5's, not a \
                         language module's (R7)"
                    ),
                    Resolution::Unresolved { reason, evidence } => {
                        assert!(!evidence.name.is_empty(), "{path}: a miss with no name");
                        assert!(
                            !evidence.node_kind.is_empty(),
                            "{path}: a miss the histogram cannot name"
                        );
                        *histogram.entry(format!("{reason:?}")).or_default() += 1;
                    }
                }
            }
        }

        assert_eq!(
            histogram.values().sum::<usize>(),
            total,
            "the reasons must account for 100% of {total} references, not {histogram:?}"
        );
        assert!(total > 10_000, "the corpus produced only {total} references; that is not it");
    }

    // ── relations (spec §3.3, plan step 6) ───────────────────────────────────

    /// Every relation as `kind child -> parent`, so an assertion names the fact
    /// it wants instead of indexing into a vector.
    /// **A MODULE CONTAINS THE DECLARATIONS WRITTEN IN IT**, and that is what
    /// finally gives `nodes.parent_id` a value for a top-level item.
    ///
    /// Until now `parent_id` was fed only by `Owns`, which is type -> member, so
    /// the containment tree was two levels deep and everything at file scope
    /// hung off nothing. The file-module is the missing top.
    ///
    /// Three claims in one fixture, because they are one rule seen from three
    /// sides: a free item is contained by its FILE; an inline `mod` is itself
    /// contained by the file; and an item inside that `mod` is contained by the
    /// `mod` and NOT by the file.
    ///
    /// A member is NOT here. `Widget::new` is owned by `Widget`, and a member
    /// that also claimed a module parent would give `parent_id` two answers.
    #[test]
    fn a_module_contains_the_declarations_written_directly_in_it() {
        let facts = facts(
            "m",
            "pub fn free() {}\n\
             pub struct Widget;\n\
             impl Widget { pub fn new() -> Widget { Widget } }\n\
             pub mod inner { pub fn deep() {} }\n",
        );
        let mut contains: Vec<String> =
            relations(&facts).into_iter().filter(|r| r.starts_with("Contains ")).collect();
        contains.sort_unstable();

        assert_eq!(
            contains,
            vec![
                "Contains rust·p·m::inner·deep·item -> rust·p·m·inner·mod".to_string(),
                "Contains rust·p·m·Widget·item -> rust·p·m·mod".to_string(),
                "Contains rust·p·m·free·item -> rust·p·m·mod".to_string(),
                "Contains rust·p·m·inner·mod -> rust·p·m·mod".to_string(),
            ],
            "the file holds what is written at file scope, the inline `mod` holds its own, \
             and `Widget::new` is owned by `Widget` rather than contained by either"
        );
    }

    /// **A GLOB IMPORT IS A REFERENCE TO A MODULE; A NAMED ONE IS NOT.**
    ///
    /// `use a::b::*` names the module `a::b` — that is what is being globbed —
    /// so it is the one Rust shape that can point at a [`SymbolKind::Module`],
    /// and the file-module work is what finally gave it a target.
    ///
    /// `use a::b::C` is REFUSED, and the refusal is the point. It arrives as the
    /// same `Binding::Name` as `use a::b;`, which imports a MODULE — the last
    /// segment is a type in one and a module in the other, and nothing in this
    /// file says which. Emitting either at `Reach::Mod` is right half the time.
    #[test]
    fn a_glob_import_references_the_module_it_globs_and_a_named_import_does_not() {
        use crate::indexer::facts::RefKind;

        let facts = facts(
            "m",
            "use crate::other::*;\n\
             use crate::thing::Widget;\n\
             pub fn go(w: Widget) -> u32 { 1 }\n",
        );
        let imported: Vec<String> = facts
            .references
            .iter()
            .filter(|r| r.kind == RefKind::Imports)
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                Resolution::Unresolved { reason, evidence } => {
                    format!("{reason:?}({})", evidence.name)
                }
            })
            .collect();

        assert_eq!(
            imported.len(),
            1,
            "one import names a module here — the glob — and `use crate::thing::Widget` does \
             not: {imported:?}"
        );
        assert!(
            imported[0].contains("other"),
            "and it names the module being globbed: {imported:?}"
        );
    }

    fn relations(facts: &FileFacts) -> Vec<String> {
        facts
            .relations
            .iter()
            .map(|r| {
                let parent = match &r.parent {
                    Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                    Resolution::Unresolved { reason, evidence } => {
                        format!("{reason:?}({})", evidence.name)
                    }
                };
                format!("{:?} {} -> {}", r.kind, r.child.as_str(), parent)
            })
            .collect()
    }

    /// The relations that claim one type IS another. Kept apart from
    /// [`RelationKind::Owns`], which claims only that a member lives on a type.
    fn inheritance(facts: &FileFacts) -> Vec<String> {
        facts
            .relations
            .iter()
            // Neither ownership NOR containment is inheritance. `!= Owns`
            // alone would read every `Contains` as a supertype.
            .filter(|r| !matches!(r.kind, RelationKind::Owns | RelationKind::Contains))
            .map(|r| format!("{:?} {}", r.kind, r.child.as_str()))
            .collect()
    }

    /// The trap the plan names. An inherent `impl Foo { }` says nothing about
    /// what `Foo` IS — it only groups members. Emitting an inheritance edge for
    /// one is a false edge, and pattern detection reads false edges as real: an
    /// Adapter is "implements X and holds an X", so a bogus `implements` on
    /// every impl block would name half the repository an Adapter.
    #[test]
    fn an_inherent_impl_is_not_inheritance_and_only_a_trait_impl_is() {
        let facts = facts(
            "m",
            "pub struct Widget;\npub trait Draw {}\n\
             impl Widget { fn area(&self) -> u32 { 0 } }\n\
             impl<T: Clone> Holder<T> { fn get(&self) {} }\n\
             impl !Send for Widget {}\n\
             impl Draw for Widget {}\n",
        );

        assert_eq!(
            inheritance(&facts),
            vec!["TraitImpl rust·p·m·Widget·item"],
            "only `impl Draw for Widget` claims Widget IS something; the inherent impls claim \
             nothing, and `impl !Send` claims the opposite. Got {:?}",
            relations(&facts)
        );
        assert!(
            relations(&facts)
                .contains(&"TraitImpl rust·p·m·Widget·item -> Unplaced(Draw)".to_string()),
            "the trait side is a name the shared ladder places, carried as a stated miss \
             rather than a bare string; got {:?}",
            relations(&facts)
        );
    }

    /// **THE `TraitImpl` EDGE AND THE METHOD NODE MUST NAME THE SAME TYPE**
    /// (stage 11, §5).
    ///
    /// `impl Draw for Box2` inside a file that merely IMPORTS `Box2` states two
    /// facts about one type, and they were minted under two different modules.
    /// The block's members were already anchored on the type's own home — that
    /// is what `home` is for, and it is the whole of the split-`impl` fix — but
    /// the owner identity the edge points at was minted from the BLOCK's module.
    ///
    /// So the file emitted `TraitImpl -> rust·p·start·Box2·item`, an identity no
    /// declaration anywhere mints, while putting `draw` on
    /// `rust·p·shape·Box2·item`, which is the real one. A dangling edge beside a
    /// correct node, from one `impl` header.
    ///
    /// This is not a hypothetical shape. Rust puts `impl` blocks anywhere, and
    /// `impl <someone else's trait> for <our type>` in a third file is ordinary.
    ///
    /// The mutation that must break it: `module: &home` back to
    /// `module: &scope.module` in `Walk::impl_block`.
    #[test]
    fn the_trait_impl_edge_names_the_type_at_its_own_home() {
        let facts = facts(
            "start",
            "use crate::shape::Box2;\n\
             pub trait Draw { fn draw(&self) -> u32; }\n\
             impl Draw for Box2 { fn draw(&self) -> u32 { 0 } }\n",
        );

        assert_eq!(
            inheritance(&facts),
            vec!["TraitImpl rust·p·shape·Box2·item"],
            "the edge names Box2 where Box2 LIVES — which this file states, in its own import \
             — and not where the impl block happens to sit. All of it: {:?}",
            relations(&facts)
        );
        assert!(
            fqns(&facts).contains(&"rust·p·shape·Box2·draw·item"),
            "and the method the same block declares names the same type, so the edge and the \
             node agree; got {:?}",
            fqns(&facts)
        );
    }

    /// A supertrait is the one thing Rust spells the way `extends` is spelled:
    /// `trait Sub: Super` states that every `Sub` is a `Super`. A lifetime in
    /// the same bound list names no type, so it yields no relation rather than
    /// a relation to an invented one (R4).
    #[test]
    fn a_supertrait_bound_is_the_only_extends_rust_states() {
        let facts = facts("m", "pub trait Sub<'a>: Super + Send + 'a {}");
        assert_eq!(
            relations(&facts),
            vec![
                // The trait is written at file scope, so the file holds it.
                "Contains rust·p·m·Sub·item -> rust·p·m·mod",
                "Extends rust·p·m·Sub·item -> Unplaced(Super)",
                "Extends rust·p·m·Sub·item -> Unplaced(Send)",
            ],
            "`'a` bounds the trait's lifetime, not its supertraits"
        );
    }

    /// R8's Facade row needs member ownership as an EDGE, not as a substring of
    /// an identity: "which members does this type own" has to be answerable from
    /// nodes and edges with no return to source. A field, a method, an
    /// associated const, an enum variant and a trait-impl method are all owned.
    #[test]
    fn a_type_owns_every_member_declared_on_it() {
        let facts = facts("m", ONE_OF_EACH);
        let owned = relations(&facts);

        for expected in [
            "Owns rust·p·m·Widget·width·field -> rust·p·m·Widget·item",
            "Owns rust·p·m·Widget·height·field -> rust·p·m·Widget·item",
            "Owns rust·p·m·Pair·0·field -> rust·p·m·Pair·item",
            "Owns rust·p·m·Shape·Circle·item -> rust·p·m·Shape·item",
            "Owns rust·p·m·Shape::Rect·w·field -> rust·p·m·Shape·Rect·item",
            "Owns rust·p·m·Draw·Canvas·item -> rust·p·m·Draw·item",
            "Owns rust·p·m·Draw·SIDES·item -> rust·p·m·Draw·item",
            "Owns rust·p·m·Widget·new·item -> rust·p·m·Widget·item",
            "Owns rust·p·m·Widget·draw·item -> rust·p·m·Widget·item",
        ] {
            assert!(owned.contains(&expected.to_string()), "no relation {expected}; got {owned:?}");
        }

        // A free item is owned by nothing. Naming the file or the module as its
        // owner would be an edge the source never states.
        assert!(
            !owned.iter().any(|r| r.starts_with("Owns ") && r.contains("·free·item ->")),
            "a free function is owned by no TYPE; it is contained by its module, which is a \
             different relation and a different column-feeder: {owned:?}"
        );
    }

    /// An `impl` on a type with no name has no identity to hang a relation on,
    /// and the members inside it have none either. Minting one would be exactly
    /// the fabrication R4 forbids, so the whole block yields no relation while
    /// its body is still walked for use sites.
    #[test]
    fn an_impl_on_an_unnameable_type_yields_no_relation_at_all() {
        let facts = facts(
            "m",
            "pub trait Draw { fn draw(&self); }\n\
             impl Draw for (u32, u32) { fn draw(&self) {} }\n",
        );
        assert_eq!(
            relations(&facts),
            vec![
                "Contains rust·p·m·Draw·item -> rust·p·m·mod",
                "Owns rust·p·m·Draw·draw·item -> rust·p·m·Draw·item",
            ],
            "the tuple impl names no type, so it hangs no relation; the trait's own \
             declaration still owns its member"
        );
    }

    /// The plan's step 6 verify, over the corpus rather than a fixture. A
    /// relation with a child that is not a well-formed identity, or a parent
    /// that is a bare name with nothing said about it, is the "wrong edge"
    /// failure in structural form.
    #[test]
    fn every_relation_in_this_repos_rust_names_both_of_its_ends() {
        let mut total = 0usize;
        for (path, text) in repo_rust_sources() {
            let facts = read(&Source { package: "p", module: "m", path: &path, text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            for relation in &facts.relations {
                total += 1;
                fqn::parse(relation.child.as_str())
                    .unwrap_or_else(|e| panic!("{path}: relation child is not an fqn: {e:?}"));
                match &relation.parent {
                    Resolution::Resolved { fqn, .. } => {
                        fqn::parse(fqn.as_str()).unwrap_or_else(|e| {
                            panic!("{path}: relation parent is not an fqn: {e:?}")
                        });
                    }
                    Resolution::Unresolved { evidence, .. } => assert!(
                        !evidence.name.is_empty(),
                        "{path}: a relation parent with no name is a bare edge with no reason"
                    ),
                }
            }
        }
        assert!(total > 1_000, "the corpus produced only {total} relations; that is not it");
    }

    /// **A member's module segment is the module its TYPE lives in — the same
    /// on both sides, or the two never merge (spec §2).**
    ///
    /// A `#[cfg(test)] mod tests` calling its own file's type is the case that
    /// exposes it, and it is everywhere: a test calling its subject IS this
    /// shape. MEASURED before the fix, from the corpus:
    ///
    ///     declared at config.rs:220   …config·SenseiConfig·brew_install_script·item
    ///     called  at config.rs:459    …config::tests·SenseiConfig·brew_install_script·item
    ///
    /// One segment apart, same file, same type. The use site took ITS OWN
    /// module because `Walk::declared_here` recorded only type NAMES, so the
    /// home fell back to the scope. A name is not a home.
    #[test]
    fn a_member_is_named_by_its_types_module_not_the_callers() {
        let facts = crate::indexer::walked_rust(
            "config",
            "src/config.rs",
            "pub struct Config;\n\
             impl Config {\n\
               pub fn script(&self) -> u32 { 0 }\n\
             }\n\
             mod tests {\n\
               use super::Config;\n\
               fn t() { let c = Config; c.script(); }\n\
             }\n",
        );

        let declared = facts
            .symbols
            .iter()
            .find(|s| s.name == "script")
            .expect("the fixture declares script")
            .fqn
            .clone();

        let called = facts
            .references
            .iter()
            .filter(|r| r.kind == RefKind::Calls)
            .find_map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => Some(fqn.as_str().to_string()),
                Resolution::Unresolved { evidence, .. } => {
                    evidence.identities().next().map(|f| f.as_str().to_string())
                }
            })
            .expect("the fixture calls script");

        assert_eq!(
            called,
            declared.as_str(),
            "the call site inside `mod tests` named the member under ITS module; the \
             declaration named it under the type's. Two spellings of one symbol never merge."
        );
    }
}
