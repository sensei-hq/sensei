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
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile;
    fn parse_to_ir(&self, source: &str, file_path: &str) -> IRParsedFile;
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
}
