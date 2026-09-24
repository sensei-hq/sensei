pub mod common;
#[cfg(test)]
mod corpus_tests;
pub mod fqn;
pub mod import_target;
pub mod jvm;
pub mod kotlin;
pub mod rust_lang;
pub mod sql;
pub mod swift;

use crate::ir::IRParsedFile;
use crate::types::ParsedFile;

/// Trait for language-specific adapters.
pub trait LanguageAdapter: Send + Sync {
    #[allow(dead_code)]
    fn language(&self) -> &'static str;
    /// UI-facing label. Defaults to Title-Casing `language()`; override where
    /// the natural label diverges (acronyms like SQL, single letters like C).
    /// Currently exercised only from tests + planned Track 3 Libraries screen —
    /// silence dead-code until a call site lands.
    #[allow(dead_code)]
    fn display_name(&self) -> &str {
        // Best-effort default: Title-Case the language slug. Overrides in
        // each impl are the right home for acronym / short-word exceptions.
        title_case_static(self.language())
    }
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile;
    fn parse_to_ir(&self, source: &str, file_path: &str) -> IRParsedFile;

    /// Produce the FQN symbol-table output for this file (plan Phase 3+): every
    /// definition and reference resolved to a canonical FQN so `process_file` can
    /// emit resolved node→node edges. The default is `None` — the file stays on the
    /// bare-name path. Every shipped adapter overrides this: derive the file's
    /// `(package, module)` context from its own manifest/layout rules, then run its
    /// per-language producer.
    ///
    /// Params, in order: `abs_path` is the on-disk path, used ONLY for walking up to
    /// a manifest; `rel_path` is folder-relative — the same value stored in
    /// `nodes.file_path` — and is the correct anchor for a language whose scope comes
    /// from layout rather than a package declaration (C, Swift); `content` is source.
    ///
    /// A path-scoped language MUST prefer `rel_path`: `abs_path` would bake this
    /// machine's home directory into every FQN, and a bare file stem collides across
    /// directories (see the header/impl case in `c_fqn`).
    fn fqn_output(
        &self,
        _abs_path: &str,
        _rel_path: &str,
        _content: &str,
    ) -> Option<fqn::FqnFileOutput> {
        None
    }

    /// The file extensions this adapter claims, WITH the leading dot.
    ///
    /// No default, deliberately: it is the single source of truth that
    /// [`adapter_for_ext`] dispatches on, so a new adapter cannot be added
    /// without declaring what it handles. Before this the extension list lived
    /// in a `match` beside the adapter list — two lists that had to agree, and
    /// nothing made them.
    fn extensions(&self) -> &[&'static str];

    /// Whether this adapter produces FQN symbol tables.
    ///
    /// A DECLARATION, because the real thing (`fqn_output`) is not a pure
    /// function of the source — the TS and Rust producers walk the filesystem
    /// for a manifest, so probing it needs a real directory and cannot run on a
    /// request path. No default, so every adapter must state its position.
    ///
    /// A declaration can lie, so it is not trusted: the test
    /// `declared_fqn_support_matches_a_real_probe` builds a per-language fixture,
    /// CALLS `fqn_output`, and fails the build if any adapter's claim disagrees
    /// with what it actually does.
    fn supports_fqn(&self) -> bool;

    /// Whether this adapter PARSES, or is registered only so its extensions map
    /// to a language name.
    ///
    /// This registry answers two questions, and a cutover moves only one of
    /// them: `crate::indexer::lang` produces the graph for a language in
    /// `PRODUCTION_LANGUAGES`, while `classifiers::is_source_file`,
    /// `language_for_ext` and `scan_logic::is_project_source_ext` still ask HERE
    /// what language an extension is. An adapter that has been cut over keeps
    /// the second job and loses the first.
    ///
    /// The invariants that are properties of a PARSER — "every adapter supports
    /// fqn", "every claimed capability survives a probe" — are scoped to
    /// adapters where this is true. Not an exemption granted to a language, but
    /// a statement that there is no parser here for the invariant to be about.
    fn parses(&self) -> bool {
        true
    }

    /// Whether this adapter can resolve a bare name using the LANGUAGE's own
    /// scope rules, rather than by matching the name against the folder.
    ///
    /// Name equality is not a substitute and the data says so: the measured
    /// head of same-name matches is `json` 1,600 / `path` 483 / `join` 443 —
    /// external accessor methods that happen to share a name with one local
    /// symbol — while genuinely resolvable in-file references are MISSED
    /// because some unrelated file defines the same name. A resolver built on
    /// name equality is the one `process.rs` refuses as "confidently WRONG".
    ///
    /// Declared rather than derived, like `supports_fqn`, and kept honest the
    /// same way: a probe test builds a fixture and checks the claim, because a
    /// declaration can drift from the implementation.
    ///
    /// Default `false`. An adapter with no scope machinery must not claim it —
    /// kotlin is exactly that case today.
    fn resolves_in_scope(&self) -> bool {
        false
    }

    /// Whether this adapter emits INHERITANCE facts — `extends` / `implements` /
    /// trait impls — into `FqnFileOutput::relations`.
    ///
    /// Declared separately, and reported, because `relations` being empty for a
    /// file is ambiguous: a language with no producer looks exactly like a
    /// language whose file simply declares no supertype. That ambiguity hid a
    /// real gap for a long time — typescript, javascript and svelte emitted ZERO
    /// while java, rust, python and kotlin emitted thousands, and the only way to
    /// notice was to query the graph. A capability cell makes it self-reporting.
    ///
    /// Default `false`: an adapter with no relation producer must not claim one.
    fn emits_inheritance(&self) -> bool {
        false
    }

    /// The language this one DELEGATES parsing to, if any.
    ///
    /// Frameworks compose over a host rather than inheriting from it: `svelte`
    /// and `vue` extract `<script>` blocks and hand them to the TypeScript
    /// adapter. Declaring it makes the relationship queryable — and explains why
    /// a `.svelte` file's symbols carry `typescript·` fqns, which import
    /// resolution has to fan out across.
    fn host_language(&self) -> Option<&'static str> {
        None
    }
}

/// Title-Case a lowercase language slug for the default `display_name`.
///
/// Returns a `&'static str` for the common single-word cases so the default
/// works without allocation. Non-matching inputs fall back to the slug —
/// concrete adapters that need a different casing must override
/// `display_name`.
#[allow(dead_code)]
fn title_case_static(slug: &str) -> &str {
    match slug {
        "rust" => "Rust",
        "typescript" => "TypeScript",
        "javascript" => "JavaScript",
        "python" => "Python",
        "java" => "Java",
        "swift" => "Swift",
        "kotlin" => "Kotlin",
        "csharp" => "C#",
        "svelte" => "Svelte",
        "vue" => "Vue",
        // ACRONYMS and single letters, where the default Title-Case is wrong
        // ("Php") or does nothing ("c").
        "php" => "PHP",
        "c" => "C",
        "go" => "Go",
        "ruby" => "Ruby",
        "shell" => "Shell",
        "markdown" => "Markdown",
        other => other,
    }
}

/// Get the adapter for a file extension, or None if unsupported.
/// EVERY language adapter — the one registry.
///
/// `adapter_for_ext` dispatches off this list plus each adapter's
/// [`LanguageAdapter::extensions`], so adding a language means adding one entry
/// here and nothing else. It also makes the set enumerable, which is what
/// [`capability_matrix`] needs: before this there was no way to ask "what
/// languages does this daemon support, and what can each of them do?" — the
/// answer lived in a `match` arm.
/// An adapter that answers "what language is this?" and nothing else.
///
/// This registry answers TWO questions — "who parses this?" and "what language
/// is this?" — and a cutover only moves the first. `classifiers::is_source_file`,
/// `language_for_ext` and `scan_logic::is_project_source_ext` all read the
/// second, so deleting a cut-over adapter outright stops its files being
/// recognised as source at all. That is measured, not hypothetical: it happened
/// to `.rs` (23 tests) and again to `.ts` here.
///
/// So the TypeScript family keeps a registration and loses its PARSER, which is
/// the split the note on `RustAdapter` asks for. `crate::indexer::lang` is the
/// producer for every extension below.
///
/// Parsing is `unreachable!`, not an empty result. `process_file` routes a
/// production language to v2 before v1's parse path is reached, so nothing can
/// call these — and if that routing ever regresses, a panic naming the file is a
/// bug report, while `ParsedFile::default()` would be a file that silently
/// indexed to nothing.
struct DetectionOnly {
    language: &'static str,
    extensions: &'static [&'static str],
}

impl LanguageAdapter for DetectionOnly {
    fn language(&self) -> &'static str {
        self.language
    }

    fn extensions(&self) -> &[&'static str] {
        self.extensions
    }

    fn supports_fqn(&self) -> bool {
        false
    }

    fn parses(&self) -> bool {
        false
    }

    fn parse(&self, _source: &str, file_path: &str) -> ParsedFile {
        unreachable!(
            "{}: `{file_path}` reached v1's parser, but {} is produced by crate::indexer::lang \
             — process_file must route a PRODUCTION_LANGUAGES file to v2 first",
            self.language, self.language
        )
    }

    fn parse_to_ir(&self, _source: &str, file_path: &str) -> IRParsedFile {
        unreachable!(
            "{}: `{file_path}` reached v1's IR parser, but {} is produced by \
             crate::indexer::lang",
            self.language, self.language
        )
    }
}

pub fn all_adapters() -> Vec<Box<dyn LanguageAdapter>> {
    vec![
        // **THE TYPESCRIPT FAMILY IS v2's, AND THESE ARE DETECTION ONLY.**
        // Their parsers were DELETED — `languages/typescript.rs`,
        // `languages/svelte.rs` and `languages/vue.rs` are gone — and what is
        // left is the language name each extension maps to, which this registry
        // is also the source of truth for.
        Box::new(DetectionOnly {
            language: "typescript",
            extensions: &[".ts", ".tsx", ".cts", ".mts"],
        }),
        Box::new(DetectionOnly {
            language: "javascript",
            extensions: &[".js", ".jsx", ".mjs", ".cjs"],
        }),
        Box::new(DetectionOnly { language: "svelte", extensions: &[".svelte"] }),
        // Java and Python cut over with the same change that deleted their
        // parsers here. `languages/jvm.rs` STAYS: `kotlin.rs` shares its
        // `resolve_supertype` / `resolve_type_call`, and Kotlin is still v1's.
        Box::new(DetectionOnly { language: "java", extensions: &[".java"] }),
        Box::new(DetectionOnly { language: "python", extensions: &[".py", ".pyi"] }),
        // C# has NO parser here and never had one — this entry is the first
        // thing in this registry to claim `.cs` at all. Before it, all 19,404
        // `.cs` files in the watched roots carried no language and no nodes:
        // `classifiers` counted them as source off a hardcoded extension list
        // while nothing could say what language they were.
        Box::new(DetectionOnly { language: "csharp", extensions: &[".cs"] }),
        Box::new(DetectionOnly { language: "vue", extensions: &[".vue"] }),
        // PHP is the SAME case as C# above: no parser here, and never one.
        // Before this entry nothing claimed `.php` at all, so all 2,251 `.php`
        // files in the watched roots carried no language while `classifiers`
        // counted them as source off a hardcoded extension list.
        Box::new(DetectionOnly { language: "php", extensions: &[".php"] }),
        // `.c` AND `.h` ONLY — v1's `c_lang.rs` also claimed `.cpp`, `.hpp` and
        // `.cc` and read them with a line-based scanner. C++ is not C:
        // `tree-sitter-c` recovers a class or a template into `ERROR` nodes, so
        // v2's adapter refuses them and this entry must not claim them either.
        //
        // A `DetectionOnly` entry for an extension NO v2 adapter claims is a
        // PANIC, not a skip: `production_adapter_for_ext` answers `None`, the
        // file falls through to v1, and `DetectionOnly::parse` is
        // `unreachable!()`. With no entry at all, `code::process` returns `None`
        // at its `adapter_for_ext(..)?` and the router files the file under
        // "unknown file type" — which is where `.go` and `.rb` already sit.
        Box::new(DetectionOnly { language: "c", extensions: &[".c", ".h"] }),
        // **RUST IS v2's, AND THIS REGISTRATION IS NOW DETECTION ONLY.**
        // `process_file` routes every `.rs` file to `indexer::lang::rust`
        // before v1's parse path is reached, so nothing here parses rust any
        // more. The entry stays because this registry answers TWO questions —
        // "who parses this?" and "what language is this?" — and the second is
        // read by `classifiers::is_source_file`, `language_for_ext` and
        // `scan_logic::is_project_source_ext`. Removing it made `.rs` stop
        // being recognised as source at all (23 tests).
        //
        // Splitting detection from parsing is what lets this adapter's parse
        // half actually be deleted; until then it is unreachable, not absent.
        Box::new(rust_lang::RustAdapter),
        Box::new(sql::SqlAdapter),
        Box::new(swift::SwiftAdapter),
        Box::new(kotlin::KotlinAdapter),
    ]
}

pub fn adapter_for_ext(ext: &str) -> Option<Box<dyn LanguageAdapter>> {
    all_adapters().into_iter().find(|a| a.extensions().contains(&ext))
}

/// What one language can do — derived from the trait impls, never hand-written.
///
/// A hand-maintained support table is wrong the first time someone adds an
/// adapter. This is computed, so it cannot drift; the pinning test then makes a
/// missing declaration a build failure rather than a silent gap.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapabilityReport {
    pub language: String,
    /// Whether this language still has a PARSER here. False means the language
    /// cut over to `crate::indexer::lang` and this entry survives only so its
    /// extensions map to a name — see [`LanguageAdapter::parses`].
    pub parses: bool,
    pub extensions: Vec<String>,
    /// The language this one delegates parsing to (`svelte` → `typescript`).
    pub host: Option<String>,
    /// Whether this adapter produces FQN symbol tables. Probed by CALLING
    /// `fqn_output` with a representative sample rather than trusting a declared
    /// flag — a declaration can drift from the implementation, a probe cannot.
    pub fqn: bool,
    /// Whether this adapter resolves bare names by the language's own scope
    /// rules. Reported because it is the capability that separates a real
    /// resolution defect from a name coincidence, and it is NOT universal.
    pub scope: bool,
    /// Whether this adapter emits `extends`/`implements`/trait-impl facts.
    /// Reported for the same reason as `scope`: an empty `relations` list cannot
    /// distinguish "no producer" from "nothing to report".
    pub inheritance: bool,
}

/// The capability matrix for every registered language — a pure projection of
/// the trait impls, cheap enough to serve on a request.
///
/// Uses each adapter's DECLARED `supports_fqn`. The declaration is kept honest
/// by `declared_fqn_support_matches_a_real_probe`, which calls `fqn_output`
/// against a per-language fixture and fails the build on any disagreement — so
/// this stays cheap without becoming a lie.
pub fn capability_matrix() -> Vec<CapabilityReport> {
    all_adapters()
        .into_iter()
        .map(|a| CapabilityReport {
            language: a.language().to_string(),
            parses: a.parses(),
            extensions: a.extensions().iter().map(|e| e.to_string()).collect(),
            host: a.host_language().map(str::to_string),
            fqn: a.supports_fqn(),
            // A framework INHERITS its host's scope capability. svelte and vue
            // hand their `<script>` to the TypeScript adapter, so TypeScript's
            // scope rules are the ones that apply — a framework reporting
            // `false` while delegating to a host reporting `true` would make
            // the composition claim and the capability claim contradict.
            //
            // Derived here rather than restated in each framework adapter: two
            // places declaring the same fact is two places to drift.
            scope: a.resolves_in_scope()
                || a.host_language().is_some_and(|h| {
                    all_adapters().iter().any(|x| x.language() == h && x.resolves_in_scope())
                }),
            // A framework inherits its host's relation capability for the same
            // reason it inherits scope: svelte and vue hand their `<script>` to
            // the TypeScript producer, so whatever it emits is what they emit.
            inheritance: a.emits_inheritance()
                || a.host_language().is_some_and(|h| {
                    all_adapters().iter().any(|x| x.language() == h && x.emits_inheritance())
                }),
        })
        .collect()
}

/// Get the adapter for a filename, handling compound extensions.
///
/// The `.svelte.ts` / `.svelte.js` special cases are GONE with the TypeScript
/// cutover: this indexer no longer produces that language, so there is no
/// adapter here to return for one.
///
/// They needed no replacement. They existed because THIS lookup takes a whole
/// filename, where `.svelte.ts` is ambiguous; `crate::indexer::lang` dispatches
/// on `Path::extension()`, which for `foo.svelte.ts` is `ts` — already the
/// right answer, with no compound rule to keep in step.
pub fn adapter_for_filename(filename: &str) -> Option<Box<dyn LanguageAdapter>> {
    let lower = filename.to_lowercase();

    // Fall back to regular extension
    // Derive the extension from the LOWERCASED name. Adapters declare their
    // extensions lowercase and `adapter_for_ext` matches exactly, so taking the
    // extension from the original name skipped every uppercase-extension file —
    // `ADVMATH.CPP` in `Labs/Bezier3D` produced a file node and no symbols at
    // all, and nothing said so. An unmatched extension is indistinguishable
    // from a file that genuinely has no symbols.
    let ext = std::path::Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    adapter_for_ext(&ext)
}

/// Map an adapter's `language()` slug to a `&'static str`. Every adapter returns
/// one of a closed set of slugs; this keeps callers off the short-lived boxed
/// adapter borrow without a clone. A future adapter whose slug isn't listed maps
/// to `"other"` (add a case when a new adapter lands).
/// Canonical language slug for a bare file extension (no leading dot), or `None`
/// if unrecognized. Consults the `LanguageAdapter` registry first (so
/// adapter-backed languages never duplicate their slug), then a small table for
/// text/config formats that have no adapter yet. Single source of truth for the
/// code-graph structure summary (`api::handlers::codebase`) and the node-write
/// path (`nodes.language`).
pub fn language_for_ext_slug(ext: &str) -> Option<&'static str> {
    let dotted = format!(".{ext}");
    // THE ADAPTER'S OWN ANSWER, not a second table keyed on it. There was one,
    // and it had drifted: `.cs` and `.php` reached an adapter that named itself
    // and then came back `other`, so `nodes.language` was wrong for every C#
    // node in the graph. A list that has to agree with the registry is a list
    // that eventually does not.
    if let Some(adapter) = adapter_for_ext(&dotted) {
        return Some(adapter.language());
    }
    match ext {
        // C++ — SOURCE, but parsed by nothing here. It is labelled so
        // `nodes.language` is not null on a file the scanner counts as source;
        // it used to be labelled `c`, which was wrong twice over.
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Some("cpp"),
        "go" => Some("go"),
        "rb" => Some("ruby"),
        "sh" | "bash" => Some("shell"),
        "md" | "markdown" | "mdx" => Some("markdown"),
        // NOTE: `txt` is deliberately absent — see `text_language_from_content`.
        // The extension cannot decide (llms corpus vs a licence), so this abstains.
        "toml" => Some("toml"),
        "yaml" | "yml" => Some("yaml"),
        "json" => Some("json"),
        "css" => Some("css"),
        "html" => Some("html"),
        _ => None,
    }
}

/// Whether a `.txt` file's CONTENT is markdown, or just text.
///
/// `router.rs` parses every `.txt` with the markdown doc processor, so both end up
/// as doc/section nodes — but the LANGUAGE stamp should say which it actually is.
/// The extension cannot: `docs/llms/index.txt` is markdown (rokkit's corpus —
/// headings, tables, fenced code) while `docs/License.txt` is prose. MEASURED:
/// 2,565 of the 2,896 null-language `.txt` nodes are that corpus.
///
/// Structure, not prose heuristics. Three markers, any one of which is decisive:
///
/// * an ATX heading — `#` … `######` followed by a SPACE. The space is required:
///   `#include <stdio.h>` and `#!/bin/sh` both start a line with `#`, and counting
///   those would call most C and shell files markdown.
/// * a fenced code block (```` ``` ````).
/// * a table delimiter row (`|---|`), which is what the llms component docs are
///   built from.
///
/// Deliberately NOT looking for `*emphasis*` or `[links](…)`: both appear in plain
/// prose and in code, and a false positive here is a lie a language-scoped query
/// then repeats. Erring toward `text` costs nothing — the node is still indexed and
/// still searchable.
pub fn text_language_from_content(content: &str) -> &'static str {
    for line in content.lines() {
        let t = line.trim_start();
        // ATX heading: 1..=6 '#' then a space.
        let hashes = t.bytes().take_while(|b| *b == b'#').count();
        if (1..=6).contains(&hashes) && t.as_bytes().get(hashes) == Some(&b' ') {
            return "markdown";
        }
        if t.starts_with("```") {
            return "markdown";
        }
        // Table delimiter row: only pipes, dashes, colons and spaces, with at
        // least one pipe and one dash.
        if t.contains('|')
            && t.contains('-')
            && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        {
            return "markdown";
        }
    }
    "text"
}

/// Canonical language slug for a file PATH, or `None`. Compound-extension aware
/// (`foo.svelte.ts` → typescript) via `adapter_for_filename`, then falls back to
/// the bare-extension table. Used by the node-write path to populate
/// `nodes.language`, which scopes the bare-name fallback to same-language
/// candidates during the per-language FQN rollout.
pub fn language_for_path(file_path: &str) -> Option<&'static str> {
    if let Some(adapter) = adapter_for_filename(file_path) {
        return Some(adapter.language());
    }
    let ext = std::path::Path::new(file_path).extension().and_then(|e| e.to_str())?;
    language_for_ext_slug(&ext.to_ascii_lowercase())
}

/// True when a file PATH is a test file, by language-aware convention — the single
/// source of truth for `nodes.is_test`, so the UI can filter tests out when
/// focusing on production code. PATH/segment-based (not content): a whole path
/// segment that is a test dir (`tests`/`__tests__`/`spec`/`e2e`/… — matched as a
/// segment, never a substring, so `latest`/`contest` don't false-match), or a
/// filename convention (`*.test.ts`/`*.spec.ts`, `*_test.rs|go|py`, `test_*.py`,
/// `conftest.py`, and — for Java/Kotlin — `*Test`/`*Tests`/`*IT` class names).
/// `language` is the slug from [`language_for_path`], used to gate the class-name
/// suffixes where `Test` is a common production identifier elsewhere. Inline unit
/// tests (a Rust `#[cfg(test)]` module inside a production file) are NOT flagged
/// here — that needs per-symbol granularity and would wrongly hide the file's
/// production code.
pub fn is_test_path(rel_path: &str, language: Option<&str>) -> bool {
    let norm = rel_path.replace('\\', "/");
    let lower = norm.to_ascii_lowercase();

    // Directory-segment conventions (language-agnostic). Whole-segment match so
    // `latest/`, `contest/`, `attestation/` don't false-match on "test".
    const TEST_DIRS: &[&str] =
        &["test", "tests", "__tests__", "__test__", "spec", "specs", "e2e", "testing"];
    if lower.split('/').any(|seg| TEST_DIRS.contains(&seg)) {
        return true;
    }

    // Filename conventions.
    let file = norm.rsplit('/').next().unwrap_or(&norm);
    let file_lower = file.to_ascii_lowercase();
    let stem = std::path::Path::new(file).file_stem().and_then(|s| s.to_str()).unwrap_or(file);
    let stem_lower = stem.to_ascii_lowercase();

    // Cross-language: foo.test.* / foo.spec.* (JS/TS/Svelte/Vue), *_test.* and
    // *_tests.* (Go/Py/Rust), test_*.py-style prefix, pytest's conftest.
    //
    // The bare `test`/`tests` stem is EXACT equality, not a prefix or suffix
    // rule: it names Rust's idiomatic sibling test module (`pg_store/tests.rs`),
    // where no path segment is a `tests/` DIRECTORY so the stem is the only
    // signal. Widening it to a substring would swallow `latest.rs`,
    // `contest.rs` and `testable.rs`, which the false-match test pins.
    //
    // The AFFIX rules carry both numbers for the same reason the bare stem
    // does. `pg_store/playbook_tests.rs` is the same idiom as `foo_test.rs`
    // and only `_test` was listed, so five real files of this repository —
    // four under `pg_store/`, one under `languages/` — indexed as production.
    // The underscore is what keeps this safe: `contests.rs` and `protests.rs`
    // do not carry one, and the false-match test pins them.
    if file_lower.contains(".test.")
        || file_lower.contains(".spec.")
        || stem_lower.ends_with("_test")
        || stem_lower.ends_with("_tests")
        || stem_lower.starts_with("test_")
        || stem_lower == "test"
        || stem_lower == "tests"
        || file_lower == "conftest.py"
    {
        return true;
    }

    // Class-name suffixes, case-sensitive and gated by language so `Unit` and
    // `Audit` (both end in a lowercase "it") and a production `TestData` in
    // another language don't false-match.
    //
    // JUnit's `*Test`/`*Tests` and PHPUnit's `*Test` are the SAME convention —
    // the suite is discovered by the class name — so the three languages share
    // the rule. Failsafe's `*IT`/`*ITCase` is JVM-only: PHP tooling has no
    // spelling for it, and admitting it there would read a production
    // `RateLimitIT` as a test.
    let jvm = matches!(language, Some("java") | Some("kotlin"));
    if (jvm || matches!(language, Some("php")))
        && (stem.ends_with("Test") || stem.ends_with("Tests"))
    {
        return true;
    }
    if jvm && (stem.ends_with("IT") || stem.ends_with("ITCase")) {
        return true;
    }

    false
}

/// Cyclomatic complexity estimate from source text.
pub fn compute_complexity(body: &str) -> u32 {
    let patterns = [
        "if ", "else if ", "elif ", "else ", "for ", "while ", "catch ", "case ", "&&", "||", "? ",
        "try ", "match ", "except ",
    ];
    let mut n: u32 = 1;
    for pat in &patterns {
        n += body.matches(pat).count() as u32;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An extension that UNAMBIGUOUSLY means markdown is stamped from the table.
    ///
    /// `router.rs` sends `md`, `mdx` and `txt` to the same `doc::process` markdown
    /// parser, but this table only knew `md` — so `.mdx` nodes were parsed as
    /// markdown and stamped with no language. MEASURED 2026-09-01: 3 such nodes.
    #[test]
    fn unambiguous_markdown_extensions_are_stamped_from_the_table() {
        for ext in ["md", "markdown", "mdx"] {
            assert_eq!(
                language_for_ext_slug(ext),
                Some("markdown"),
                "`.{ext}` always means markdown",
            );
        }
    }

    /// `.txt` deliberately does NOT resolve here, because the extension cannot
    /// decide. `docs/llms/index.txt` is markdown; `docs/License.txt` is not. Only
    /// the CONTENT knows, and this table only sees a path — so it abstains rather
    /// than guessing, and [`text_language_from_content`] makes the call where the
    /// content is in hand.
    #[test]
    fn txt_abstains_because_the_extension_cannot_decide() {
        assert_eq!(language_for_ext_slug("txt"), None);
    }

    /// The llms corpus is markdown despite the extension — headings, tables, fenced
    /// code. MEASURED: 2,565 null-language section nodes come from
    /// `docs/llms/**/*.txt`, which is exactly this content.
    #[test]
    fn markdown_shaped_text_is_markdown() {
        let llms = "# Rokkit Switch Component\n\n> iOS-style boolean toggle.\n\n## Props\n\n| Prop | Type |\n|---|---|\n| `value` | bool |\n";
        assert_eq!(text_language_from_content(llms), "markdown");
        assert_eq!(
            text_language_from_content("Intro\n\n## A heading\n\nbody\n"),
            "markdown",
            "a setext-free ATX heading anywhere is enough",
        );
        assert_eq!(
            text_language_from_content("some prose\n\n```rust\nfn main() {}\n```\n"),
            "markdown",
            "a fenced code block is markdown structure",
        );
    }

    /// A licence or a changelog fragment with no markdown structure is text, and
    /// calling it markdown would be a small lie that a language-scoped query then
    /// repeats.
    #[test]
    fn structureless_prose_is_text() {
        let licence = "Copyright (c) 2026 Someone\n\nPermission is hereby granted, free of charge,\nto any person obtaining a copy of this software.\n";
        assert_eq!(text_language_from_content(licence), "text");
        assert_eq!(text_language_from_content(""), "text", "empty is text, not markdown");
    }

    /// A `#` that is not a heading must not count. `#include` and a shell comment
    /// both start a line with `#`, and treating them as headings would call most
    /// config and C files markdown.
    #[test]
    fn a_hash_that_is_not_a_heading_does_not_count() {
        assert_eq!(text_language_from_content("#include <stdio.h>\nint main(){}\n"), "text");
        assert_eq!(text_language_from_content("#!/bin/sh\necho hi\n"), "text");
        assert_eq!(
            text_language_from_content("#hashtag not a heading\nmore text\n"),
            "text",
            "ATX requires a space after the hashes",
        );
    }

    #[test]
    fn adapter_for_known_extensions() {
        for ext in &[
            ".py", ".rs", ".java", ".sql", ".ddl", ".ts", ".tsx", ".cts", ".js", ".jsx", ".swift",
            ".kt", ".kts", ".svelte", ".vue", ".c", ".h", ".php", ".cs",
        ] {
            assert!(adapter_for_ext(ext).is_some(), "Missing adapter for {}", ext);
        }
    }

    /// **C++ IS NOT C, and no adapter claims it.**
    ///
    /// This registry used to answer `c` for `.cpp`, `.hpp` and `.cc`, and v1's
    /// C adapter read them with a line-based scanner. `crate::indexer::lang::c`
    /// is a `tree-sitter-c` walk, and that grammar parses C: handed a class or
    /// a template it recovers into `ERROR` nodes, and facts read out of a
    /// recovered parse are invented ones.
    ///
    /// THE ABSENCE HAS TO BE TOTAL, not just a v2 absence. A `DetectionOnly`
    /// entry claiming `.cpp` would PANIC rather than skip:
    /// `production_adapter_for_ext` answers `None`, the file falls through to
    /// v1's path, and `DetectionOnly::parse` is `unreachable!()`. With no entry
    /// at all, `code::process` returns `None` at its `adapter_for_ext(..)?` and
    /// the router files the file under "unknown file type" — a file node and no
    /// symbols, which is where `.go` and `.rb` already sit.
    ///
    /// 9 files in the watched roots, against 165 `.c`/`.h`.
    ///
    /// MUTATION: add `.cpp` to either registry's C entry. Adding it to v2 makes
    /// tree-sitter-c invent structure from a recovered parse; adding it to v1's
    /// `DetectionOnly` panics the indexer on every C++ file.
    #[test]
    fn no_adapter_claims_a_cpp_extension() {
        for ext in [".cpp", ".hpp", ".cc", ".cxx", ".hh"] {
            assert!(adapter_for_ext(ext).is_none(), "{ext} is C++, and nothing here parses C++");
            assert!(
                crate::indexer::lang::adapter_for_ext(ext).is_none(),
                "{ext} is C++, and tree-sitter-c does not parse it"
            );
        }
    }

    #[test]
    fn adapter_for_unknown_extension() {
        assert!(adapter_for_ext(".xyz").is_none());
    }

    /// Probe FQN support for real: build a tempdir with the manifest and source
    /// that language's producer actually needs, then CALL `fqn_output`.
    ///
    /// A declared `supports_fqn` flag would be simpler and could lie. This
    /// cannot — but it does mean the fixture has to be honest about each
    /// producer's inputs (TS wants `package.json`, Rust wants `Cargo.toml`,
    /// Java reads the in-source `package` declaration and ignores the path).
    fn probe_fqn_for_real(a: &dyn LanguageAdapter) -> bool {
        let Some((manifest, src_name, src)) = (match a.language() {
            "typescript" => Some((
                Some(("package.json", "{\"name\":\"probe\"}")),
                "src/a.ts",
                "export function m() { return 1; }\n",
            )),
            "javascript" => Some((
                Some(("package.json", "{\"name\":\"probe\"}")),
                "src/a.js",
                "export function m() { return 1; }\n",
            )),
            "svelte" => Some((
                Some(("package.json", "{\"name\":\"probe\"}")),
                "src/A.svelte",
                "<script>export function m() { return 1; }</script>\n",
            )),
            "vue" => Some((
                Some(("package.json", "{\"name\":\"probe\"}")),
                "src/A.vue",
                "<script>export function m() { return 1; }</script>\n",
            )),
            "rust" => Some((
                Some(("Cargo.toml", "[package]\nname = \"probe\"\n")),
                "src/a.rs",
                "pub struct A;\nimpl A { pub fn m(&self) {} }\n",
            )),
            "java" => Some((None, "A.java", "package p;\npublic class A { void m() {} }\n")),
            "python" => Some((
                Some(("pyproject.toml", "[project]\nname = \"probe\"\n")),
                "probe/a.py",
                "class A:\n    def m(self):\n        pass\n",
            )),
            "sql" => Some((None, "t.sql", "create table t (id int);\n")),
            "swift" => Some((None, "A.swift", "class A { func m() {} }\n")),
            "kotlin" => Some((None, "A.kt", "package p\nclass A { fun m() {} }\n")),
            "c" => Some((None, "a.c", "int m(void) { return 1; }\n")),
            _ => None,
        }) else {
            return false;
        };

        let tmp = tempfile::tempdir().expect("tempdir");
        if let Some((name, body)) = manifest {
            std::fs::write(tmp.path().join(name), body).expect("write manifest");
        }
        let abs = tmp.path().join(src_name);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&abs, src).expect("write source");
        // rel_path is what production passes (folder-relative), so the probe
        // passes the same shape rather than the tempdir's absolute path.
        a.fqn_output(&abs.to_string_lossy(), src_name, src).is_some()
    }

    /// THE DECLARATION MUST NOT LIE. Every adapter states `supports_fqn`; this
    /// builds a real per-language fixture, CALLS `fqn_output`, and compares. A
    /// cheap declaration plus a proof beats both a slow probe on the request
    /// path and an unverified flag.
    ///
    /// Breaking mutation: flip any adapter's `supports_fqn` — this fails naming
    /// that adapter and the direction of the lie.
    #[test]
    fn declared_fqn_support_matches_a_real_probe() {
        for a in all_adapters() {
            let declared = a.supports_fqn();
            let actual = probe_fqn_for_real(a.as_ref());
            assert_eq!(
                declared,
                actual,
                "{} declares supports_fqn={declared} but calling fqn_output returned \
                 {}. The declaration is what callers see, so it must match reality.",
                a.language(),
                if actual { "Some" } else { "None" },
            );
        }
    }

    /// The registry is the ONE list. Every adapter declares its extensions, and
    /// `adapter_for_ext` dispatches off that declaration — so a new language
    /// cannot be half-added (present in the list, invisible to lookup) the way a
    /// separate `match` arm allowed.
    /// An UPPERCASE extension must find its adapter.
    ///
    /// Found in production, not by reasoning: `Labs/Bezier3D` holds `ADVMATH.CPP`
    /// and `ADVMATH.H`, and those files produced only `file` and `module` nodes —
    /// no symbols at all, ever. `adapter_for_filename` lowercases the filename
    /// for the compound-`.svelte.ts` check and then derives the extension from
    /// the ORIGINAL, so `.CPP` matched nothing and the file was silently skipped
    /// for symbol extraction.
    ///
    /// Silently is the problem: an unmatched extension is indistinguishable from
    /// a file with no symbols in it.
    ///
    /// Breaking mutation: derive `ext` from `filename` instead of `lower` in
    /// `adapter_for_filename`.
    #[test]
    fn an_uppercase_extension_still_finds_its_adapter() {
        for (name, want) in [
            ("ADVMATH.H", "c"),
            ("Widget.KT", "kotlin"),
            ("Main.JAVA", "java"),
            ("script.PY", "python"),
        ] {
            let a = adapter_for_filename(name)
                .unwrap_or_else(|| panic!("no adapter for {name} — uppercase extension skipped"));
            assert_eq!(a.language(), want, "{name} resolved to the wrong adapter");
        }
        // Mixed case too, since real trees hold `Foo.Kt`.
        assert!(adapter_for_filename("Foo.Kt").is_some(), "mixed-case extension");
        // The lowercase path must keep working.
        assert_eq!(
            adapter_for_filename("main.rs").map(|a| a.language().to_string()).as_deref(),
            Some("rust")
        );
    }

    #[test]
    fn every_registered_adapter_is_reachable_by_its_own_declared_extensions() {
        let adapters = all_adapters();
        assert!(adapters.len() >= 11, "registry shrank unexpectedly: {}", adapters.len());

        for a in &adapters {
            assert!(
                !a.extensions().is_empty(),
                "{} declares no extensions, so nothing can ever dispatch to it",
                a.language()
            );
            for ext in a.extensions() {
                assert!(
                    ext.starts_with('.'),
                    "{}: extension {ext} needs a leading dot",
                    a.language()
                );
                let found = adapter_for_ext(ext).map(|f| f.language().to_string());
                assert_eq!(
                    found.as_deref(),
                    Some(a.language()),
                    "{ext} declared by {} but adapter_for_ext resolved it to {found:?}",
                    a.language()
                );
            }
        }

        // No extension may be claimed twice — the first match would silently win.
        let mut seen = std::collections::HashMap::new();
        for a in &adapters {
            for ext in a.extensions() {
                if let Some(prev) = seen.insert(*ext, a.language().to_string()) {
                    panic!("{ext} claimed by both {prev} and {}", a.language());
                }
            }
        }
    }

    // `framework_adapters_declare_their_host_language` and
    // `a_framework_inherits_its_hosts_scope_capability` STOOD HERE. Svelte and
    // Vue were this registry's only two frameworks and both cut over with
    // TypeScript, so there is no host left here to declare or inherit from.
    //
    // The guarantee MOVED rather than went away:
    // `indexer::lang::tests::a_framework_declares_the_host_it_delegates_to`
    // holds it where the adapters now live.

    /// Scope resolution is a CAPABILITY, declared per language and reported.
    ///
    /// Resolving a bare name correctly needs the language's own scope rules —
    /// Rust's `use` map plus its submodule set, Java's imports plus
    /// same-package, Python's module bindings, TS's import bindings. A
    /// name-equality match cannot substitute: the measured head of same-name
    /// matches is `json` 1,600 / `path` 483 / `join` 443, which are external
    /// accessors, while genuinely resolvable in-file cases are MISSED because
    /// a same-named symbol exists elsewhere in the folder.
    ///
    /// So this is declared per adapter and surfaced in [`capability_matrix`],
    /// exactly as `supports_fqn` is — and for the same reason: before slice 1
    /// there was no way to ask which languages could do what, and the answer
    /// lived in a `match` arm nobody could query.
    ///
    /// Every adapter EXCEPT kotlin already has the machinery internally
    /// (`use_map`/`FileScope` in rust, an imports map in java/python/typescript);
    /// kotlin has none, which is why this must be reported rather than assumed.
    ///
    /// Breaking mutation: return `true` from KotlinAdapter — the probe test
    /// fails because no scope map exists to back it.
    #[test]
    fn scope_resolution_is_declared_and_reported_per_language() {
        let m = capability_matrix();
        assert!(!m.is_empty(), "the matrix must not be empty");

        // Every report carries the cell — a capability that is not reported
        // cannot be reasoned about.
        let with_scope: Vec<&str> =
            m.iter().filter(|r| r.scope).map(|r| r.language.as_str()).collect();
        let without: Vec<&str> =
            m.iter().filter(|r| !r.scope).map(|r| r.language.as_str()).collect();

        // typescript and javascript are NOT here any more: they cut over to
        // `crate::indexer::lang`, whose scope machinery is its own and is
        // measured by `indexer::acceptance` over the real corpus rather than
        // declared in this matrix. What is left is what this registry parses.
        // RUST is the only one left here with a scope map — java, python and the
        // TypeScript family all cut over, and kotlin has never had one. A list
        // of one rather than a loop, because a loop over a single element is a
        // list that used to be longer.
        assert!(
            with_scope.contains(&"rust"),
            "rust has a scope map and must declare it: with={with_scope:?}"
        );
        assert!(
            without.contains(&"kotlin"),
            "kotlin has NO scope map and must not claim one: without={without:?}"
        );
    }

    /// The INHERITANCE cell must be reported, honest, and inherited by a
    /// framework from its host.
    ///
    /// It exists because an empty `relations` list is ambiguous — a language with
    /// no producer looks exactly like a file that declares no supertype. That
    /// ambiguity hid a real gap: typescript, javascript and svelte emitted ZERO
    /// extends/implements while java (1,166+1,002), rust (753), python (389) and
    /// kotlin (43+20) emitted thousands, and the only way to notice was to query
    /// the graph.
    ///
    /// Probed, not just declared: each claimant parses a fixture WITH a supertype
    /// and must produce a non-empty `relations`. A declaration can drift from the
    /// implementation; a probe cannot.
    #[test]
    fn declared_inheritance_support_matches_a_real_probe() {
        let m = capability_matrix();
        let claims: Vec<&str> =
            m.iter().filter(|r| r.inheritance).map(|r| r.language.as_str()).collect();

        // typescript, javascript, svelte and vue are NOT here any more: they cut
        // over to `crate::indexer::lang`, and their adapters were deleted in the
        // same change. The framework-inheritance clause went with them — svelte
        // and vue were the only two frameworks this registry held, and with no
        // host left to delegate to there is nothing for it to assert.
        // java and python cut over with the same change that deleted their
        // parsers here. What is left is what this registry still parses.
        for lang in ["rust", "kotlin"] {
            assert!(
                claims.contains(&lang),
                "{lang} emits relations and must declare it: {claims:?}"
            );
        }

        // THE PROBE. Every claimant must actually produce a relation.
        //
        // java/kotlin/python are SOURCE-ONLY — the package comes from the file's
        // own header — so a synthetic path works.
        //
        // There used to be a second half here for typescript/javascript, which
        // walk up for a `package.json` and so return None on a synthetic path;
        // it probed the producer directly with an explicit context. Both the
        // producer and the adapters are gone with the cutover, and the same
        // property is now held by `crate::indexer::acceptance` over the real
        // corpus rather than over one fixture.
        let source_only: &[(&str, &str, &str)] =
            &[("kotlin", "T.kt", "package p\nclass C : B() {}\n")];
        for (lang, file, src) in source_only {
            let Some(a) = adapter_for_filename(file) else {
                panic!("no adapter for {file}");
            };
            let out = a
                .fqn_output(&format!("/tmp/probe/{file}"), file, src)
                .unwrap_or_else(|| panic!("{lang}: fqn_output returned None for {file}"));
            assert!(
                !out.relations.is_empty(),
                "{lang} claims inheritance but produced no relation for `{src}`"
            );
        }
    }

    /// FQN support is REQUIRED of every adapter, with NO exceptions.
    ///
    /// An adapter without it produces symbols an fqn lookup can never find while
    /// name-based lookups still match them — the same symbol visible to one
    /// mechanism and invisible to the other, which is how a reference ends up
    /// unmatched or matched to the wrong definition.
    ///
    /// This test used to carry a shrinking allowlist of known gaps (swift,
    /// kotlin, c). It is empty now, so the allowlist is gone rather than left
    /// behind at zero: an exception mechanism that exists is an exception
    /// mechanism that gets used. Adding a language means giving it an fqn
    /// producer in the same change, and this assert is what says so.
    ///
    /// Breaking mutation: return `false` from any adapter's `supports_fqn`, or
    /// drop any `fqn_output` override — the language is named in the failure.
    #[test]
    fn every_adapter_supports_fqn_with_no_exceptions() {
        // Scoped to adapters that PARSE. A cut-over language keeps a
        // detection-only registration with no producer behind it, so "it has no
        // fqn producer" is a description of that entry rather than a gap in it.
        let gaps: Vec<String> = capability_matrix()
            .into_iter()
            .filter(|r| r.parses && !r.fqn)
            .map(|r| r.language)
            .collect();

        assert!(
            gaps.is_empty(),
            "these adapters have no working FQN producer: {gaps:?} — every language must have one"
        );
    }

    #[test]
    fn is_test_path_detects_test_files_by_convention() {
        // Directory-segment conventions (any language).
        for p in [
            "tests/integration.rs",
            "crates/x/tests/foo.rs",
            "src/test/java/com/A.java",
            "app/src/__tests__/util.ts",
            "e2e/login.spec.ts",
            "spec/models/user_spec.rb",
        ] {
            assert!(is_test_path(p, None), "should be a test path: {p}");
        }
        // Filename conventions.
        assert!(is_test_path("src/util.test.ts", Some("typescript")));
        assert!(is_test_path("src/util.spec.ts", Some("typescript")));
        assert!(is_test_path("pkg/foo_test.go", Some("go")));
        assert!(is_test_path("mymod/parser_test.rs", Some("rust")));
        assert!(is_test_path("pkg/test_parser.py", Some("python")));
        assert!(is_test_path("pkg/conftest.py", Some("python")));
        // A BARE `tests.rs` / `test.rs` stem — Rust's idiomatic sibling test
        // module. Nothing in the path is a `tests/` DIRECTORY segment, so the
        // stem is the only signal; the affix rules above match `foo_test.rs`
        // and `test_foo.py` but never the bare word. Live consequence of the
        // gap: all 373 test fns in pg_store/tests.rs indexed as is_test=false,
        // so "exclude tests" filtered nothing and returned them as production.
        assert!(is_test_path("crates/senseid/src/db/pg_store/tests.rs", Some("rust")));
        assert!(is_test_path("src/parser/test.rs", Some("rust")));
        // The PLURAL affix, which is the same Rust idiom as `foo_test.rs` and
        // was the one spelling this rule did not have. Four real files carry it
        // — `pg_store/playbook_tests.rs`, `knowledge_tests.rs`, `run_tests.rs`,
        // `pack_resolution_tests.rs` — plus `languages/corpus_tests.rs`, and
        // every declaration in them read as PRODUCTION to `nodes.is_test` and
        // to the indexer's coverage barrier alike.
        assert!(is_test_path("crates/senseid/src/db/pg_store/playbook_tests.rs", Some("rust")));
        assert!(is_test_path("crates/senseid/src/languages/corpus_tests.rs", Some("rust")));
        // Language-agnostic: the stem carries the convention on its own.
        assert!(is_test_path("app/src/lib/tests.ts", Some("typescript")));
        // Java/Kotlin class-name suffixes (gated by language).
        assert!(is_test_path("src/main/java/com/FooTest.java", Some("java")));
        assert!(is_test_path("src/main/java/com/FooTests.java", Some("java")));
        assert!(is_test_path("src/main/java/com/FooIT.java", Some("java")));
        // PHP's is the SAME class-name convention: PHPUnit discovers a suite by
        // `*Test.php`, and a project that keeps its tests beside the code it
        // exercises has no `tests/` segment for the directory rule to find.
        //
        // The indexer's coverage barrier routes PHP entirely through this
        // function — `barrier::inline_tests_begin` returns `None` for it,
        // because PHP has no in-file marker — so a gap here is not cosmetic:
        // every declaration in such a file reads as PRODUCTION to both
        // `nodes.is_test` and the barrier.
        assert!(is_test_path("src/Domain/UserTest.php", Some("php")));
        assert!(is_test_path("src/Domain/UserTests.php", Some("php")));
        // NOT `*IT` — that is Failsafe's integration-test convention on the
        // JVM, and PHP has nothing that spells it. Admitting it here would make
        // a production `RateLimitIT` read as a test in a language whose
        // tooling never uses the suffix.
        assert!(!is_test_path("src/Domain/RateLimitIT.php", Some("php")));
    }

    #[test]
    fn is_test_path_does_not_false_match_production() {
        // Substrings that merely CONTAIN "test" are not test dirs.
        for p in [
            "src/latest/config.rs",
            "contest/rules.py",
            "src/attestation/verify.ts",
            "src/lib/pg_store.rs",
            "app/src/routes/+page.svelte",
            "src/main.rs",
            // Bare-stem matching is EXACT equality, so a stem that merely ends
            // or starts with the word is still production. These pin that the
            // `tests.rs` rule did not widen into a `contains`.
            "src/latest.rs",
            "src/contest.rs",
            "src/protest.ts",
            "src/testable.rs",
            // The plural affix needs its UNDERSCORE. These end in the letters
            // of `_tests` without one, and are what stops that rule becoming a
            // substring match.
            "src/contests.rs",
            "src/protests.ts",
        ] {
            assert!(!is_test_path(p, language_for_path(p)), "should NOT be a test path: {p}");
        }
        // `*Test`/`*IT` suffix is Java/Kotlin-gated: a Rust `TestHarness` production
        // file and a stem ending in lowercase "it" don't match.
        assert!(
            !is_test_path("src/Audit.java", Some("java")),
            "Audit ends in lowercase 'it', not IT"
        );
        assert!(!is_test_path("src/testkit.rs", Some("rust")), "no *Test suffix rule for rust");
    }

    // ── display_name (1b Step 1) ────────────────────────────────────────

    #[test]
    fn display_name_default_title_cases_common_slugs() {
        // Adapters that don't override display_name get the Title-Cased slug.
        assert_eq!(adapter_for_ext(".rs").unwrap().display_name(), "Rust");
        assert_eq!(adapter_for_ext(".ts").unwrap().display_name(), "TypeScript");
        assert_eq!(adapter_for_ext(".js").unwrap().display_name(), "JavaScript");
        assert_eq!(adapter_for_ext(".py").unwrap().display_name(), "Python");
        assert_eq!(adapter_for_ext(".java").unwrap().display_name(), "Java");
        assert_eq!(adapter_for_ext(".svelte").unwrap().display_name(), "Svelte");
        assert_eq!(adapter_for_ext(".vue").unwrap().display_name(), "Vue");
        assert_eq!(adapter_for_ext(".swift").unwrap().display_name(), "Swift");
        assert_eq!(adapter_for_ext(".kt").unwrap().display_name(), "Kotlin");
        // PHP is claimed here for DETECTION only — v2 owns its parse. Without
        // the entry `.php` resolves to no adapter at all, which is how 2,251
        // files came to carry no language: `classifiers` counts them as source
        // off a hardcoded extension list while nothing can say what they are.
        // The same gap C# had, and the same fix.
        assert_eq!(adapter_for_ext(".php").unwrap().display_name(), "PHP");
    }

    /// Every extension a registered adapter claims reports THAT adapter's
    /// language — not `other`.
    ///
    /// The property, not an instance. `language_for_ext_slug` used to consult a
    /// second hand-written table beside the registry, and a language reached
    /// this list only if someone remembered to add it there too. Two of them
    /// had not been: `.cs` and `.php` both resolved to an adapter that named
    /// itself and then came back `other`, which is what `nodes.language` was
    /// being written from — so every C# node in the graph carried the wrong
    /// language while `adapter_for_ext(".cs")` answered correctly.
    ///
    /// MUTATION: reintroduce the table and drop one entry — this fails for that
    /// language, where a per-language assert only fails for the ones somebody
    /// thought to write down.
    #[test]
    fn every_registered_extension_reports_its_own_adapters_language() {
        for adapter in all_adapters() {
            for ext in adapter.extensions() {
                let bare = ext.trim_start_matches('.');
                assert_eq!(
                    language_for_ext_slug(bare),
                    Some(adapter.language()),
                    "{ext} is claimed by {} and must report it",
                    adapter.language()
                );
            }
        }
    }

    #[test]
    fn display_name_overrides_for_acronyms_and_single_letters() {
        // SQL, PHP and C need explicit overrides — the default Title-Case
        // gives "Sql", "Php", and leaves a single letter alone.
        assert_eq!(adapter_for_ext(".sql").unwrap().display_name(), "SQL");
        assert_eq!(adapter_for_ext(".ddl").unwrap().display_name(), "SQL");
        assert_eq!(adapter_for_ext(".php").unwrap().display_name(), "PHP");
        assert_eq!(adapter_for_ext(".c").unwrap().display_name(), "C");
        assert_eq!(adapter_for_ext(".h").unwrap().display_name(), "C");
    }
}
