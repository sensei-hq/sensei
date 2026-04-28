//! Config file processor — .json, .toml, .yaml, .yml files.
//! Extracts metadata without parsing symbols.

use super::types::*;

/// Process a config file.
pub fn process(abs_path: &str, rel_path: &str, ext: &str) -> FileProcessResult {
    let mut r = FileProcessResult::minimal(
        format!("file:{}", abs_path),
        rel_path.to_string(),
        abs_path.to_string(),
        "file",
        "config",
    );
    r.language = Some(ext.to_string());
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf()
    }

    fn fixtures() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    #[test]
    fn package_json() {
        let abs = fixtures().join("config/package.json");
        let r = process(&abs.to_string_lossy(), "config/package.json", "json");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
        assert_eq!(r.language.as_deref(), Some("json"));
        assert!(r.symbols.is_empty(), "config files have no symbols");
    }

    #[test]
    fn cargo_toml() {
        let root = workspace_root();
        let abs = root.join("crates/mcp/Cargo.toml");
        let r = process(&abs.to_string_lossy(), "crates/mcp/Cargo.toml", "toml");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
        assert_eq!(r.language.as_deref(), Some("toml"));
    }
}
