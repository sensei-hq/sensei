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
    fn language(&self) -> &str;
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile;
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
pub fn adapter_for_filename(filename: &str) -> Option<(Box<dyn LanguageAdapter>, &str)> {
    let lower = filename.to_lowercase();

    // Compound svelte extensions: .svelte.ts, .svelte.js
    if lower.ends_with(".svelte.ts") || lower.ends_with(".svelte.tsx") {
        return Some((Box::new(typescript::TypeScriptAdapter), "typescript"));
    }
    if lower.ends_with(".svelte.js") || lower.ends_with(".svelte.jsx") {
        return Some((Box::new(typescript::JavaScriptAdapter), "javascript"));
    }

    // Fall back to regular extension
    let ext = std::path::Path::new(filename).extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    adapter_for_ext(&ext).map(|a| {
        let lang = a.language().to_string();
        (a, Box::leak(lang.into_boxed_str()) as &str)
    })
}

/// Get the IR parse for a file extension, or None if unsupported.
pub fn parse_to_ir_for_ext(ext: &str, source: &str, file_path: &str) -> Option<IRParsedFile> {
    match ext {
        ".py" => Some(python::parse_to_ir(source, file_path)),
        ".rs" => Some(rust_lang::parse_to_ir(source, file_path)),
        ".ts" | ".tsx" | ".cts" => Some(typescript::parse_to_ir(source, file_path)),
        ".js" | ".jsx" | ".mjs" | ".cjs" => Some(typescript::parse_to_ir(source, file_path)),
        ".java" => Some(java::parse_to_ir(source, file_path)),
        ".sql" | ".ddl" => Some(sql::parse_to_ir(source, file_path)),
        ".swift" => Some(swift::parse_to_ir(source, file_path)),
        ".kt" | ".kts" => Some(kotlin::parse_to_ir(source, file_path)),
        ".svelte" => Some(svelte::parse_to_ir(source, file_path)),
        ".vue" => Some(vue::parse_to_ir(source, file_path)),
        ".c" | ".h" | ".cpp" | ".hpp" | ".cc" => Some(c_lang::parse_to_ir(source, file_path)),
        _ => None,
    }
}

/// Get the IR parse for a filename, handling compound extensions.
pub fn parse_to_ir_for_filename(filename: &str, source: &str, file_path: &str) -> Option<IRParsedFile> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".svelte.ts") || lower.ends_with(".svelte.tsx")
        || lower.ends_with(".svelte.js") || lower.ends_with(".svelte.jsx")
    {
        return Some(typescript::parse_to_ir(source, file_path));
    }
    let ext = std::path::Path::new(filename).extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();
    parse_to_ir_for_ext(&ext, source, file_path)
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
