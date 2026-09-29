pub mod fqn;
pub mod import_target;

// The language REGISTRY that used to live here is gone, with the parsers it
// dispatched to. `crate::indexer::lang` is the only one, and every language it
// reads is in `PRODUCTION_LANGUAGES`, so a second registry could only ever
// disagree with it about which extension is code.
//
// What remains below is language-ADJACENT and producer-agnostic: naming a
// language from a path or from content, deciding whether a path is a test, and
// counting branches. None of it parses anything.

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
    // `name()`, NOT `language()`. They answer different questions and the
    // difference is load-bearing: `.js` and `.ts` share one `Language` so an
    // import across them merges to one identity, while `nodes.language` is what
    // a reader filters on and "javascript" is a distinction they want kept.
    // Collapsing the two relabelled every .js node typescript.
    if let Some(adapter) = crate::indexer::lang::adapter_for_ext(&dotted) {
        return Some(adapter.name());
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
    // COMPOUND EXTENSION FIRST — `foo.svelte.ts` is typescript, not svelte. The
    // indexer's registry declares the compound forms, so asking it by the
    // longest matching suffix keeps one answer rather than two.
    let lower = file_path.to_ascii_lowercase();
    if let Some(adapter) = crate::indexer::lang::all_adapters()
        .iter()
        .flat_map(|a| a.extensions().iter().map(move |e| (*e, *a)))
        .filter(|(ext, _)| lower.ends_with(*ext))
        .max_by_key(|(ext, _)| ext.len())
        .map(|(_, a)| a)
    {
        return Some(adapter.name());
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
            assert!(
                crate::indexer::lang::adapter_for_ext(ext).is_none(),
                "{ext} is C++, and tree-sitter-c does not parse it"
            );
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
}
