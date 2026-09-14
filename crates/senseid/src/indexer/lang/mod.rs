//! One module per language, each owning exactly one thing: how to read that
//! language's grammar (R7).
//!
//! Everything a language module is NOT allowed to own lives above it: the fqn
//! grammar in `fqn.rs`, the fact vocabulary in `facts.rs`, the resolution ladder
//! and the reason codes in `resolve.rs`. A rule that would have to be written
//! twice for two languages does not belong here.
//!
//! # Why a trait, and not a `match` on [`Language`]
//!
//! Because the `match` was already there and was already wrong. `persist.rs`
//! carried `Language::Rust => lang::rust::file_fqn(..)`, which is one arm per
//! language per QUESTION — and there are five questions a caller asks a
//! language, so a second language meant five more arms in five more files, each
//! of them a place to forget one. The registry below is the single list, and
//! [`adapter_for`] is total over [`Language`], so a language cannot be added
//! without answering every question it has to answer.
//!
//! This is the pattern `crate::languages` established for the legacy indexer
//! and it is kept deliberately: one trait, one registry, capabilities declared
//! rather than inferred, and a probe test that fails the build when a
//! declaration and the implementation disagree.
//!
//! # Two dispatch keys, and they are different questions
//!
//! - **By EXTENSION** — "who reads this file?" Three adapters read the
//!   TypeScript language and they claim different extensions, because a `.js`
//!   file has no annotations to read and a `.svelte` file has markup wrapped
//!   around its script.
//! - **By [`Language`]** — "how is a symbol of this language NAMED?" Identity is
//!   a property of the language, not of the file's extension: `.js` and `.ts`
//!   must mint the same identity for the same symbol or an import across them
//!   never merges. So [`adapter_for`] answers with a CANONICAL adapter, and
//!   `every_adapter_of_one_language_agrees_on_identity` is what keeps the other
//!   adapters of that language from drifting from it.
//
// These modules have no caller on purpose — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

pub mod common;
pub mod javascript;
pub mod rust;
pub mod svelte;

use std::collections::BTreeMap;

use crate::indexer::facts::{FileFacts, Fqn, Language, Symbol, SymbolKind};
use crate::indexer::fqn::FqnError;
use crate::indexer::resolve::Grammar;

/// One file, plus the two things the file cannot know about itself: which
/// package owns it and where it sits in that package's module tree. Both are
/// supplied by the processor from the manifest and the path, because a source
/// file states neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source<'a> {
    pub package: &'a str,
    /// Package-relative module path, joined with the language's module
    /// separator, empty at the package root.
    pub module: &'a str,
    /// Where the file is, as the graph records it.
    ///
    /// Needed because the module path does not identify a file at the PACKAGE
    /// ROOT, where it is empty and one package may have several. Carried on the
    /// facts rather than supplied again at write time so the identity a
    /// file-scope use site is filed under and the identity the writer hangs
    /// imports off cannot be derived from two different strings.
    pub path: &'a str,
    pub text: &'a str,
}

/// Why a file produced no facts at all.
///
/// Only whole-file failures live here. A single declaration or use site the
/// walk cannot read is never an error — it is a fact carrying what was seen
/// (R2, R4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The parser could not be prepared — a build problem, not a property of
    /// the source.
    GrammarUnavailable(String),
    /// The parser returned no tree. Distinct from a tree full of ERROR nodes,
    /// which is a normal thing to walk.
    NotParsed,
    /// The file's own identity could not be minted, so nothing inside it could
    /// be named either.
    NoFileIdentity(FqnError),
}

/// Where each type NAME is declared, for the packages being scanned.
///
/// **The one fact a walk cannot read out of the file it was handed, and needs.**
/// A member's identity carries the module, then the type, then the member,
/// and that MODULE is the
/// module the TYPE lives in — `PgStore::upsert_symbol` is reached through
/// `db::pg_store::PgStore` however many files carry an `impl PgStore`. Rust puts
/// those impl blocks anywhere; this repo has 24 of them for `PgStore` alone.
///
/// Without it the walk fills `<module>` with the module the impl block sits in,
/// and so does the reference side, so a call resolves only when the caller
/// happens to share a module with the impl. MEASURED: 634 of 5,186 members sat
/// under a module their type does not live in, and 2,538 unresolved member
/// references named a type whose home was elsewhere.
///
/// The walk RESOLVES nothing here — it is TOLD. Building the table is a
/// barrier's job (every file walked before any is anchored), which is what
/// keeps the answer independent of scan order (R6).
///
/// An AMBIGUOUS name — two modules of one package declaring it — is absent, not
/// guessed at. 354 members are in that case, and picking one of two homes would
/// mint a wrong identity, which R4 ranks below no identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeHomes {
    homes: BTreeMap<(String, String), String>,
}

impl TypeHomes {
    /// No table at all: every type is treated as living where its impl block
    /// does. What a caller that has not run the barrier gets, and what the
    /// walk did before this existed.
    pub fn unknown() -> Self {
        Self { homes: BTreeMap::new() }
    }

    /// Build from every declaration the scan has seen, as
    /// `(package, symbol)` pairs.
    ///
    /// A name declared in two modules of one package is DROPPED rather than
    /// resolved to the first: two homes is not one home, and the walk must be
    /// able to tell "I know where this lives" from "I know two places".
    pub fn of<'a>(declarations: impl IntoIterator<Item = (&'a str, &'a Symbol)>) -> Self {
        let mut seen: BTreeMap<(String, String), Option<String>> = BTreeMap::new();
        for (package, symbol) in declarations {
            if !names_a_type(symbol.kind) {
                continue;
            }
            let Some(module) = module_of_item(symbol.fqn.as_str(), &symbol.name) else {
                continue;
            };
            let key = (package.to_string(), symbol.name.clone());
            match seen.get(&key) {
                // A second, DIFFERENT home makes the name ambiguous for good.
                Some(Some(first)) if *first != module => {
                    seen.insert(key, None);
                }
                Some(_) => {}
                None => {
                    seen.insert(key, Some(module));
                }
            }
        }
        Self { homes: seen.into_iter().filter_map(|(k, v)| Some((k, v?))).collect() }
    }

    /// The module a type is declared in, or `None` when the scan does not know
    /// or knows two.
    pub fn home_of(&self, package: &str, ty: &str) -> Option<&str> {
        self.homes.get(&(package.to_string(), ty.to_string())).map(String::as_str)
    }

    /// How many names have ONE known home. For a report; a table that silently
    /// came out empty would make every anchoring decision a no-op.
    pub fn len(&self) -> usize {
        self.homes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.homes.is_empty()
    }
}

/// Whether a [`SymbolKind`] names something a member can hang off.
fn names_a_type(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Struct
            | SymbolKind::Enum
            | SymbolKind::Trait
            | SymbolKind::Class
            | SymbolKind::Interface
    )
}

/// The module segment of an ITEM identity, whose parts in order are the
/// language, the package, an optional module, the name, and the reach.
///
/// Only the item form is decomposed, and only with the name in hand. The MEMBER
/// forms differ by one segment while an empty module is dropped, so a
/// six-segment string is a member-with-module or a trait-member-without-one and
/// the string cannot say which — see [`crate::indexer::fqn::parse`]. Guessing
/// there read a trait as a type and would have collapsed every implementation
/// of a trait onto one identity.
fn module_of_item(fqn: &str, name: &str) -> Option<String> {
    let mut segments: Vec<&str> = fqn.split(crate::indexer::fqn::SEPARATOR).collect();
    // reach
    segments.pop()?;
    if segments.pop()? != name {
        return None;
    }
    // lang, package
    if segments.len() < 2 {
        return None;
    }
    Some(segments[2..].join(&crate::indexer::fqn::SEPARATOR.to_string()))
}

/// How one language is read (R7). Everything else about it — how a miss is
/// classified, how an identity is encoded, how a path is climbed — is shared.
pub trait LanguageAdapter: Send + Sync {
    /// Which language this adapter's symbols are FILED under. Several adapters
    /// may answer the same — see the module docs.
    fn language(&self) -> Language;

    /// A stable name for this adapter, for a report a person reads. Distinct
    /// from [`LanguageAdapter::language`] precisely because three adapters
    /// share one language and a matrix that could not tell them apart would be
    /// useless.
    fn name(&self) -> &'static str;

    /// The file extensions this adapter claims, WITH the leading dot.
    ///
    /// No default, deliberately: it is the single source of truth that
    /// [`adapter_for_ext`] dispatches on, so a new adapter cannot be added
    /// without declaring what it handles.
    fn extensions(&self) -> &'static [&'static str];

    /// What the shared resolution ladder needs to know about this language, and
    /// nothing more.
    fn grammar(&self) -> &'static Grammar;

    /// Parse one file once (R1) and return everything that parse saw (spec §3).
    ///
    /// PURE, and it must stay pure: it takes source TEXT rather than a path to
    /// open, so its whole suite runs on string literals and no database is in
    /// scope.
    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError>;

    /// The identity of the file itself, which is the identity of the module it
    /// declares.
    fn file_fqn(&self, package: &str, module: &str, path: &str) -> Result<Fqn, FqnError>;

    /// A file's package-relative MODULE PATH, from its path alone.
    ///
    /// `package_root` is the directory the package's manifest sits in. The rule
    /// is the language's — Rust drops a trailing `mod`/`lib`/`main`, JavaScript
    /// drops a trailing `index` — and it is a SEGMENT OF EVERY FQN the file
    /// declares, so two implementations of it would mint two identities for one
    /// declaration.
    fn module_path(&self, file: &str, package_root: &str) -> String;

    /// Reduce the source text of a type to the one segment that names it.
    ///
    /// Per-language because the decorations are: Rust strips `&`, `dyn` and a
    /// turbofish; TypeScript strips `| null`, `[]` and a generic argument list.
    /// The DEFINITION side and the REFERENCE side of one language must both
    /// call this one, or they mint different strings for one type and never
    /// merge (spec §2).
    fn type_segment(&self, raw: &str) -> Result<String, FqnError>;

    /// Does renaming `from` to `to` re-mint the identities the file declares?
    /// (09 S5.)
    ///
    /// Given a body here because the ANSWER is shared even though the rule is
    /// not: the
    /// module path is a segment of every identity a file declares in every
    /// language here, so a rename that moves it re-mints them and one that does
    /// not is a `files` row update and nothing more.
    fn rename_remints_identity(&self, from: &str, to: &str, package_root: &str) -> bool {
        self.module_path(from, package_root) != self.module_path(to, package_root)
    }

    /// The language this one DELEGATES parsing to, if any.
    ///
    /// Frameworks compose over a host rather than inheriting from it: Svelte
    /// extracts its `<script>` block and hands it to the TypeScript reader.
    /// Declaring it makes the relationship queryable, and explains why a
    /// `.svelte` file's symbols are filed under the TypeScript language rather
    /// than a Svelte one.
    fn host(&self) -> Option<&'static str> {
        None
    }
}

/// EVERY adapter — the one registry.
///
/// [`adapter_for_ext`] dispatches off this list plus each adapter's
/// [`LanguageAdapter::extensions`], so adding a language means adding one entry
/// here and nothing else.
pub fn all_adapters() -> &'static [&'static dyn LanguageAdapter] {
    &[
        &rust::RustAdapter,
        &javascript::TypeScriptAdapter,
        &javascript::JavaScriptAdapter,
        &svelte::SvelteAdapter,
    ]
}

/// Who reads a file with this extension (WITH the leading dot).
///
/// `None` means no adapter claims it, which is a fact the caller acts on — it
/// is never a licence to pick the nearest one.
pub fn adapter_for_ext(ext: &str) -> Option<&'static dyn LanguageAdapter> {
    all_adapters().iter().copied().find(|a| a.extensions().contains(&ext))
}

/// The canonical adapter for a language — the one whose IDENTITY rules
/// (`file_fqn`, `module_path`, `type_segment`, `grammar`) are that language's.
///
/// TOTAL over [`Language`], and that totality is the point: a language variant
/// that no adapter answers for would be a symbol nothing can name, and the
/// `match` this replaced let exactly that compile as a `todo!()`.
pub fn adapter_for(language: Language) -> &'static dyn LanguageAdapter {
    // The FIRST adapter registered for the language. Registration order is
    // therefore meaningful and is asserted in the tests: for TypeScript it puts
    // the annotation-reading adapter ahead of the two that have none, which is
    // the one whose rules the others are checked against.
    all_adapters()
        .iter()
        .copied()
        .find(|a| a.language() == language)
        .expect("every language has an adapter, which the registry tests prove")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The totality [`adapter_for`] relies on, proven rather than asserted by
    /// its `expect`. A `Language` variant with no adapter is a symbol the graph
    /// could store and never name.
    #[test]
    fn every_language_has_a_canonical_adapter_and_it_claims_that_language() {
        for language in Language::all() {
            let adapter = adapter_for(*language);
            assert_eq!(
                adapter.language(),
                *language,
                "adapter_for({language:?}) answered with {}, which reads {:?}",
                adapter.name(),
                adapter.language()
            );
        }
    }

    /// One extension, one reader. Two adapters claiming `.ts` would make which
    /// one runs depend on registration order, and the two do not have to agree.
    #[test]
    fn no_two_adapters_claim_one_extension() {
        let mut claimed: Vec<(&str, &str)> = Vec::new();
        for adapter in all_adapters() {
            for ext in adapter.extensions() {
                if let Some((_, other)) = claimed.iter().find(|(e, _)| e == ext) {
                    panic!("{ext} is claimed by both {other} and {}", adapter.name());
                }
                claimed.push((ext, adapter.name()));
            }
        }
        assert!(!claimed.is_empty(), "the registry claims no extensions at all");
    }

    /// Every extension a leading dot, because [`adapter_for_ext`] compares
    /// against what the caller split off a filename and a mismatch there is a
    /// silent "no adapter" rather than an error.
    #[test]
    fn every_declared_extension_carries_its_dot_and_dispatches_back() {
        for adapter in all_adapters() {
            for ext in adapter.extensions() {
                assert!(ext.starts_with('.'), "{} declares {ext} without its dot", adapter.name());
                let found = adapter_for_ext(ext).expect("a declared extension dispatches");
                assert_eq!(found.name(), adapter.name(), "{ext} dispatched to the wrong adapter");
            }
        }
        assert!(adapter_for_ext(".cobol").is_none(), "an unclaimed extension has no adapter");
        assert!(adapter_for_ext("ts").is_none(), "dispatch is on the dotted form");
    }

    /// Identity is a property of the LANGUAGE, so the adapters sharing one must
    /// answer the identity questions identically. This is what makes it safe for
    /// [`adapter_for`] to answer with any one of them.
    ///
    /// The mutation that must break it: give `JavaScriptAdapter` its own
    /// `module_path` that keeps a trailing `index`. A `.js` file importing a
    /// `.ts` file would then never merge with it.
    #[test]
    fn every_adapter_of_one_language_agrees_on_identity() {
        let cases = [
            ("pkg", "a/b", "src/a/b.ts"),
            ("pkg", "", "src/index.ts"),
            ("pkg", "lib/store", "src/lib/store/index.ts"),
        ];
        let mut compared = 0;
        for adapter in all_adapters() {
            let canonical = adapter_for(adapter.language());
            if canonical.name() == adapter.name() {
                continue;
            }
            compared += 1;
            for (package, module, path) in cases {
                assert_eq!(
                    adapter.file_fqn(package, module, path).map(|f| f.to_string()),
                    canonical.file_fqn(package, module, path).map(|f| f.to_string()),
                    "{} and {} mint different file identities for {path}",
                    adapter.name(),
                    canonical.name()
                );
                assert_eq!(
                    adapter.module_path(path, "."),
                    canonical.module_path(path, "."),
                    "{} and {} disagree on the module path of {path}",
                    adapter.name(),
                    canonical.name()
                );
                assert_eq!(
                    adapter.type_segment("Widget"),
                    canonical.type_segment("Widget"),
                    "{} and {} reduce one type two ways",
                    adapter.name(),
                    canonical.name()
                );
            }
            assert_eq!(
                adapter.grammar().language,
                canonical.grammar().language,
                "{} and {} carry grammars for different languages",
                adapter.name(),
                canonical.name()
            );
        }
        assert!(compared > 0, "no language has a second adapter, so this passed vacuously");
    }

    /// A registered adapter that cannot read its own language is worse than an
    /// absent one: dispatch finds it, `read` returns nothing useful, and the
    /// file looks like it has no declarations. Probed by CALLING `read`, because
    /// a declaration can drift from the implementation and a probe cannot.
    #[test]
    fn every_registered_adapter_reads_a_file_of_its_own_language() {
        for adapter in all_adapters() {
            let text = fixture_for(adapter.name());
            let path = format!("src/fixture{}", adapter.extensions()[0]);
            let facts = adapter
                .read(
                    &Source { package: "pkg", module: "fixture", path: &path, text },
                    &TypeHomes::unknown(),
                )
                .unwrap_or_else(|e| {
                    panic!("{} could not read its own fixture: {e:?}", adapter.name())
                });
            assert_eq!(facts.language, adapter.language());
            assert!(
                !facts.symbols.is_empty(),
                "{} read its own fixture and found no declarations",
                adapter.name()
            );
            assert!(
                !facts.references.is_empty(),
                "{} read its own fixture and found no use sites",
                adapter.name()
            );
        }
    }

    /// A file of each language that declares something and uses something.
    /// Deliberately per-adapter rather than one string: the whole reason three
    /// adapters exist is that the three grammars are not the same text.
    fn fixture_for(name: &str) -> &'static str {
        match name {
            "rust" => {
                "pub struct Widget { pub width: u32 }\n\
                 impl Widget { pub fn wide(&self) -> u32 { self.width } }\n\
                 pub fn free(w: &Widget) -> u32 { w.wide() }\n"
            }
            "typescript" => {
                "export class Widget { width = 0; wide(): number { return this.width; } }\n\
                 export function free(): number { const w = new Widget(); return w.wide(); }\n"
            }
            "javascript" => {
                "export class Widget { wide() { return 1; } }\n\
                 export function free() { const w = new Widget(); return w.wide(); }\n"
            }
            "svelte" => {
                "<script>\n\
                 class Widget { wide() { return 1; } }\n\
                 const w = new Widget();\n\
                 </script>\n\
                 <p>{w.wide()}</p>\n"
            }
            other => panic!("no fixture for the adapter named {other}"),
        }
    }

    /// The trait exists so a caller asks the registry rather than writing its
    /// own arm. This is the guard that says so, and it reads the sources because
    /// the defect it prevents is a `match` somebody adds later.
    #[test]
    fn nothing_outside_this_module_dispatches_on_a_language_by_hand() {
        let needle = format!("{}::Rust =>", "Language");
        let mut read = 0;
        for (path, body) in crate::indexer::guard_sources() {
            if path.starts_with("lang") {
                continue;
            }
            read += 1;
            let body = crate::indexer::outside_tests(&body);
            assert!(
                !body.contains(needle.as_str()),
                "{path} dispatches on a language by hand; ask `lang::adapter_for` instead, \
                 which is total over `Language` and cannot silently miss one"
            );
        }
        assert!(read > 0, "the guard read no files, so it would have passed vacuously");
    }
}
