pub mod python;

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
        // TODO: add more adapters
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_for_known_extension() {
        assert!(adapter_for_ext(".py").is_some());
    }

    #[test]
    fn adapter_for_unknown_extension() {
        assert!(adapter_for_ext(".xyz").is_none());
    }
}
