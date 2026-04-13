pub mod python;
pub mod rust_lang;
pub mod typescript;
pub mod java;
pub mod sql;

use crate::types::ParsedFile;

/// Trait for language-specific tree-sitter adapters.
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
        // TS/JS disabled — tree-sitter-typescript 0.23 has ABI incompatibility with tree-sitter 0.24
        // Will re-enable when grammar publishes a compatible version
        // ".ts" | ".tsx" => Some(Box::new(typescript::TypeScriptAdapter)),
        // ".js" | ".jsx" | ".mjs" | ".cjs" => Some(Box::new(typescript::JavaScriptAdapter)),
        ".java" => Some(Box::new(java::JavaAdapter)),
        ".sql" => Some(Box::new(sql::SqlAdapter)),
        _ => None,
    }
}

/// List all supported extensions.
pub fn supported_extensions() -> &'static [&'static str] {
    &[".py", ".rs", ".java", ".sql"]
    // TS/JS temporarily disabled due to tree-sitter grammar ABI issue
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_for_known_extensions() {
        assert!(adapter_for_ext(".py").is_some());
        assert!(adapter_for_ext(".rs").is_some());
        assert!(adapter_for_ext(".java").is_some());
        assert!(adapter_for_ext(".sql").is_some());
        // TS/JS disabled due to tree-sitter grammar ABI issue
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
        assert!(exts.contains(&".java"));
    }
}
