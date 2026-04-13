pub mod python;
pub mod rust_lang;
pub mod typescript;

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
        ".ts" | ".tsx" => Some(Box::new(typescript::TypeScriptAdapter)),
        ".js" | ".jsx" | ".mjs" | ".cjs" => Some(Box::new(typescript::JavaScriptAdapter)),
        _ => None,
    }
}

/// List all supported extensions.
pub fn supported_extensions() -> &'static [&'static str] {
    &[".py", ".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_for_known_extensions() {
        assert!(adapter_for_ext(".py").is_some());
        assert!(adapter_for_ext(".rs").is_some());
        assert!(adapter_for_ext(".ts").is_some());
        assert!(adapter_for_ext(".tsx").is_some());
        assert!(adapter_for_ext(".js").is_some());
    }

    #[test]
    fn adapter_for_unknown_extension() {
        assert!(adapter_for_ext(".xyz").is_none());
    }

    #[test]
    fn supported_exts_list() {
        let exts = supported_extensions();
        assert!(exts.contains(&".py"));
        assert!(exts.contains(&".ts"));
        assert!(exts.contains(&".rs"));
    }
}
