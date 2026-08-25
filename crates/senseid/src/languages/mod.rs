pub mod c_lang;
pub mod common;
pub mod fqn;
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
pub fn adapter_for_ext(ext: &str) -> Option<Box<dyn LanguageAdapter>> {
    match ext {
        ".py" => Some(Box::new(python::PythonAdapter)),
        ".rs" => Some(Box::new(rust_lang::RustAdapter)),
        ".ts" | ".tsx" | ".cts" => Some(Box::new(typescript::TypeScriptAdapter)),
        ".js" | ".jsx" | ".mjs" | ".cjs" => Some(Box::new(typescript::JavaScriptAdapter)),
        ".java" => Some(Box::new(java::JavaAdapter)),
        ".sql" | ".ddl" => Some(Box::new(sql::SqlAdapter)),
        ".swift" => Some(Box::new(swift::SwiftAdapter)),
        ".kt" | ".kts" => Some(Box::new(kotlin::KotlinAdapter)),
        ".svelte" => Some(Box::new(svelte::SvelteAdapter)),
        ".vue" => Some(Box::new(vue::VueAdapter)),
        ".c" | ".h" | ".cpp" | ".hpp" | ".cc" => Some(Box::new(c_lang::CAdapter)),
        _ => None,
    }
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
        "md" | "markdown" => Some("markdown"),
        "toml" => Some("toml"),
        "yaml" | "yml" => Some("yaml"),
        "json" => Some("json"),
        "css" => Some("css"),
        "html" => Some("html"),
        _ => None,
    }
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
    if file_lower.contains(".test.")
        || file_lower.contains(".spec.")
        || stem_lower.ends_with("_test")
        || stem_lower.starts_with("test_")
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
