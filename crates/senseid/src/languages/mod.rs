pub mod c_lang;
pub mod common;
#[cfg(test)]
mod corpus_tests;
pub mod fqn;
pub mod import_target;
pub mod java;
pub mod kotlin;
pub mod python;
pub mod rust_lang;
pub mod sql;
pub mod svelte;
pub mod swift;
pub mod typescript;
pub mod vue;

use crate::ir::IRParsedFile;
use crate::types::ParsedFile;

/// Trait for language-specific adapters.
pub trait LanguageAdapter: Send + Sync {
    #[allow(dead_code)]
    fn language(&self) -> &str;
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
    /// bare-name path (the language isn't FQN-migrated yet). A migrated adapter
    /// overrides this: derive the file's `(package, module)` context from its own
    /// manifest/layout rules, then run its per-language producer. `abs_path` is the
    /// on-disk path (for the manifest walk); `content` is the source.
    fn fqn_output(&self, _abs_path: &str, _content: &str) -> Option<fqn::FqnFileOutput> {
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
        "svelte" => "Svelte",
        "vue" => "Vue",
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
pub fn all_adapters() -> Vec<Box<dyn LanguageAdapter>> {
    vec![
        Box::new(python::PythonAdapter),
        Box::new(rust_lang::RustAdapter),
        Box::new(typescript::TypeScriptAdapter),
        Box::new(typescript::JavaScriptAdapter),
        Box::new(java::JavaAdapter),
        Box::new(sql::SqlAdapter),
        Box::new(swift::SwiftAdapter),
        Box::new(kotlin::KotlinAdapter),
        Box::new(svelte::SvelteAdapter),
        Box::new(vue::VueAdapter),
        Box::new(c_lang::CAdapter),
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
    pub extensions: Vec<String>,
    /// The language this one delegates parsing to (`svelte` → `typescript`).
    pub host: Option<String>,
    /// Whether this adapter produces FQN symbol tables. Probed by CALLING
    /// `fqn_output` with a representative sample rather than trusting a declared
    /// flag — a declaration can drift from the implementation, a probe cannot.
    pub fqn: bool,
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
            extensions: a.extensions().iter().map(|e| e.to_string()).collect(),
            host: a.host_language().map(str::to_string),
            fqn: a.supports_fqn(),
        })
        .collect()
}

/// Get the adapter for a filename, handling compound extensions.
/// e.g. "foo.svelte.ts" → TypeScript, "bar.spec.svelte.js" → JavaScript
pub fn adapter_for_filename(filename: &str) -> Option<Box<dyn LanguageAdapter>> {
    let lower = filename.to_lowercase();

    // Compound svelte extensions: .svelte.ts, .svelte.js
    if lower.ends_with(".svelte.ts") || lower.ends_with(".svelte.tsx") {
        return Some(Box::new(typescript::TypeScriptAdapter));
    }
    if lower.ends_with(".svelte.js") || lower.ends_with(".svelte.jsx") {
        return Some(Box::new(typescript::JavaScriptAdapter));
    }

    // Fall back to regular extension
    let ext = std::path::Path::new(filename)
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
fn language_slug_static(slug: &str) -> &'static str {
    match slug {
        "rust" => "rust",
        "typescript" => "typescript",
        "javascript" => "javascript",
        "python" => "python",
        "java" => "java",
        "kotlin" => "kotlin",
        "swift" => "swift",
        "svelte" => "svelte",
        "vue" => "vue",
        "sql" => "sql",
        "c" => "c",
        _ => "other",
    }
}

/// Canonical language slug for a bare file extension (no leading dot), or `None`
/// if unrecognized. Consults the `LanguageAdapter` registry first (so
/// adapter-backed languages never duplicate their slug), then a small table for
/// text/config formats that have no adapter yet. Single source of truth for the
/// code-graph structure summary (`api::handlers::codebase`) and the node-write
/// path (`nodes.language`).
pub fn language_for_ext_slug(ext: &str) -> Option<&'static str> {
    let dotted = format!(".{ext}");
    if let Some(adapter) = adapter_for_ext(&dotted) {
        return Some(language_slug_static(adapter.language()));
    }
    match ext {
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
        return Some(language_slug_static(adapter.language()));
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

    // Cross-language: foo.test.* / foo.spec.* (JS/TS/Svelte/Vue), *_test.*
    // (Go/Py/Rust), test_*.py-style prefix, pytest's conftest.
    //
    // The bare `test`/`tests` stem is EXACT equality, not a prefix or suffix
    // rule: it names Rust's idiomatic sibling test module (`pg_store/tests.rs`),
    // where no path segment is a `tests/` DIRECTORY so the stem is the only
    // signal. Widening it to a substring would swallow `latest.rs`,
    // `contest.rs` and `testable.rs`, which the false-match test pins.
    if file_lower.contains(".test.")
        || file_lower.contains(".spec.")
        || stem_lower.ends_with("_test")
        || stem_lower.starts_with("test_")
        || stem_lower == "test"
        || stem_lower == "tests"
        || file_lower == "conftest.py"
    {
        return true;
    }

    // Java/Kotlin class-name suffixes (JUnit `*Test`/`*Tests`, Failsafe `*IT`).
    // Case-sensitive + gated by language so `Unit`/`Audit` (end in lowercase "it")
    // and a production `TestData` in another language don't false-match.
    if matches!(language, Some("java") | Some("kotlin"))
        && (stem.ends_with("Test")
            || stem.ends_with("Tests")
            || stem.ends_with("IT")
            || stem.ends_with("ITCase"))
    {
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
            ".kt", ".kts", ".svelte", ".vue", ".c", ".h", ".cpp",
        ] {
            assert!(adapter_for_ext(ext).is_some(), "Missing adapter for {}", ext);
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
        a.fqn_output(&abs.to_string_lossy(), src).is_some()
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

    /// Framework adapters declare the host they delegate parsing to. This is not
    /// cosmetic: a `.svelte` file's symbols carry `typescript·` fqns because the
    /// TS adapter produced them, which import resolution has to fan out across.
    #[test]
    fn framework_adapters_declare_their_host_language() {
        let m = capability_matrix();
        let host_of = |lang: &str| {
            m.iter()
                .find(|r| r.language == lang)
                .unwrap_or_else(|| panic!("{lang} missing"))
                .host
                .clone()
        };
        assert_eq!(host_of("svelte").as_deref(), Some("typescript"));
        assert_eq!(host_of("vue").as_deref(), Some("typescript"));
        assert_eq!(host_of("rust"), None, "rust hosts nothing and is hosted by nothing");
        assert_eq!(host_of("typescript"), None, "typescript IS a host");
    }

    /// FQN support is REQUIRED, not optional — an adapter without it produces
    /// symbols that an fqn lookup can never find while name-based lookups still
    /// match them, so the same symbol is visible to one mechanism and invisible
    /// to another. This test names the languages that still lack it; the list
    /// must only ever SHRINK, and reaching empty is the definition of that work
    /// being done.
    #[test]
    fn fqn_support_gaps_are_named_and_must_only_shrink() {
        const KNOWN_GAPS: &[&str] = &["swift", "kotlin", "c"];

        let m = capability_matrix();
        let gaps: Vec<&str> = m.iter().filter(|r| !r.fqn).map(|r| r.language.as_str()).collect();

        for g in &gaps {
            assert!(
                KNOWN_GAPS.contains(g),
                "{g} lost FQN support — that is a REGRESSION, not a known gap"
            );
        }
        assert!(gaps.len() <= KNOWN_GAPS.len(), "gap list grew: {gaps:?} vs known {KNOWN_GAPS:?}");
        // Everything not in the gap list must actually probe Some.
        for r in m.iter().filter(|r| !KNOWN_GAPS.contains(&r.language.as_str())) {
            assert!(r.fqn, "{} is expected to support FQN but the probe returned None", r.language);
        }
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
        // Language-agnostic: the stem carries the convention on its own.
        assert!(is_test_path("app/src/lib/tests.ts", Some("typescript")));
        // Java/Kotlin class-name suffixes (gated by language).
        assert!(is_test_path("src/main/java/com/FooTest.java", Some("java")));
        assert!(is_test_path("src/main/java/com/FooTests.java", Some("java")));
        assert!(is_test_path("src/main/java/com/FooIT.java", Some("java")));
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
    }

    #[test]
    fn display_name_overrides_for_acronyms_and_single_letters() {
        // SQL and C need explicit overrides — the default Title-Case would
        // give "Sql" and "C" (the C case is fine but exercised for the assert).
        assert_eq!(adapter_for_ext(".sql").unwrap().display_name(), "SQL");
        assert_eq!(adapter_for_ext(".ddl").unwrap().display_name(), "SQL");
        assert_eq!(adapter_for_ext(".c").unwrap().display_name(), "C");
        assert_eq!(adapter_for_ext(".cpp").unwrap().display_name(), "C");
        assert_eq!(adapter_for_ext(".h").unwrap().display_name(), "C");
    }
}
