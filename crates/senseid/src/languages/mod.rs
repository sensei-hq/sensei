pub mod common;
pub mod python;
pub mod rust_lang;
pub mod typescript;
pub mod java;
pub mod sql;
pub mod swift;
pub mod kotlin;
pub mod svelte;
pub mod vue;
pub mod c_lang;

use crate::types::ParsedFile;
use crate::ir::IRParsedFile;

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
    let ext = std::path::Path::new(filename).extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    adapter_for_ext(&ext)
}

/// Cyclomatic complexity estimate from source text.
pub fn compute_complexity(body: &str) -> u32 {
    let patterns = ["if ", "else if ", "elif ", "else ", "for ", "while ", "catch ",
        "case ", "&&", "||", "? ", "try ", "match ", "except "];
    let mut n: u32 = 1;
    for pat in &patterns { n += body.matches(pat).count() as u32; }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_for_known_extensions() {
        for ext in &[".py", ".rs", ".java", ".sql", ".ddl", ".ts", ".tsx", ".cts", ".js", ".jsx", ".swift", ".kt", ".kts", ".svelte", ".vue", ".c", ".h", ".cpp"] {
            assert!(adapter_for_ext(ext).is_some(), "Missing adapter for {}", ext);
        }
    }

    #[test]
    fn adapter_for_unknown_extension() {
        assert!(adapter_for_ext(".xyz").is_none());
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
