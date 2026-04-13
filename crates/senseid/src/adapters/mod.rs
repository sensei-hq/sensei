pub mod python;
pub mod rust_lang;
pub mod typescript;
pub mod java;
pub mod sql;
pub mod swift;
pub mod kotlin;
pub mod svelte;
pub mod vue;

use crate::types::ParsedFile;

/// Trait for language-specific adapters.
pub trait LanguageAdapter: Send + Sync {
    fn language(&self) -> &str;
    fn extensions(&self) -> &[&str];
    fn parse(&self, source: &str, file_path: &str) -> ParsedFile;
}

/// Get the adapter for a file extension, or None if unsupported.
pub fn adapter_for_ext(ext: &str) -> Option<Box<dyn LanguageAdapter>> {
    match ext {
        ".py" => Some(Box::new(python::PythonAdapter)),
        ".rs" => Some(Box::new(rust_lang::RustAdapter)),
        ".ts" | ".tsx" => Some(Box::new(typescript::TypeScriptAdapter)),
        ".js" | ".jsx" | ".mjs" | ".cjs" => Some(Box::new(typescript::JavaScriptAdapter)),
        ".java" => Some(Box::new(java::JavaAdapter)),
        ".sql" => Some(Box::new(sql::SqlAdapter)),
        ".swift" => Some(Box::new(swift::SwiftAdapter)),
        ".kt" | ".kts" => Some(Box::new(kotlin::KotlinAdapter)),
        ".svelte" => Some(Box::new(svelte::SvelteAdapter)),
        ".vue" => Some(Box::new(vue::VueAdapter)),
        _ => None,
    }
}

/// List all supported extensions.
pub fn supported_extensions() -> &'static [&'static str] {
    &[
        ".py", ".rs", ".java", ".sql",
        ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs",
        ".swift", ".kt", ".kts",
        ".svelte", ".vue",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_for_known_extensions() {
        for ext in &[".py", ".rs", ".java", ".sql", ".ts", ".tsx", ".js", ".jsx", ".swift", ".kt", ".kts", ".svelte", ".vue"] {
            assert!(adapter_for_ext(ext).is_some(), "Missing adapter for {}", ext);
        }
    }

    #[test]
    fn adapter_for_unknown_extension() {
        assert!(adapter_for_ext(".xyz").is_none());
    }

    #[test]
    fn supported_exts_list() {
        let exts = supported_extensions();
        assert!(exts.contains(&".py"));
        assert!(exts.contains(&".rs"));
        assert!(exts.contains(&".swift"));
        assert!(exts.contains(&".kt"));
        assert!(exts.contains(&".svelte"));
        assert!(exts.contains(&".vue"));
    }
}
