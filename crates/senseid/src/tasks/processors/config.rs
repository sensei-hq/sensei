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

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf()
    }

    #[test]
    fn package_json() {
        let root = repo_root();
        let abs = root.join("apps/desktop/package.json");
        let r = process(&abs.to_string_lossy(), "apps/desktop/package.json", "json");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
        assert_eq!(r.language.as_deref(), Some("json"));
        assert!(r.symbols.is_empty(), "config files have no symbols");
    }

    #[test]
    fn cargo_toml() {
        let root = repo_root();
        let abs = root.join("crates/sensei-mcp/Cargo.toml");
        let r = process(&abs.to_string_lossy(), "crates/sensei-mcp/Cargo.toml", "toml");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
        assert_eq!(r.language.as_deref(), Some("toml"));
    }

    #[test]
    fn hooks_json() {
        let root = repo_root();
        let abs = root.join("marketplace/plugins/sensei/hooks/hooks.json");
        if abs.exists() {
            let r = process(&abs.to_string_lossy(), "hooks/hooks.json", "json");
            assert_eq!(r.tags, "config");
        }
    }

    #[test]
    fn plugin_config_json() {
        let root = repo_root();
        let abs = root.join("marketplace/plugins/sensei-mcp/config.json");
        if abs.exists() {
            let r = process(&abs.to_string_lossy(), "plugins/sensei-mcp/config.json", "json");
            assert_eq!(r.tags, "config");
        }
    }
}
