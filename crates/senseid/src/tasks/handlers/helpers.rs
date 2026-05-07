//! Shared helpers used by process and other handler modules.

/// Check if a file extension indicates a binary (non-text) file.
pub(crate) fn is_binary_ext(ext: &str) -> bool {
    ["png","jpg","jpeg","gif","ico","svg","woff","woff2","ttf","eot",
     "zip","tar","gz","bz2","xz","7z","rar",
     "exe","dll","so","dylib","o","a","lib",
     "db","sqlite","sqlite3","profraw",
     "wasm","map","DS_Store","lock"].contains(&ext)
}

pub(crate) fn build_globset() -> globset::GlobSet {
    let patterns = &[
        "**/node_modules/**", "**/dist/**", "**/build/**", "**/target/**",
        "**/.next/**", "**/.svelte-kit/**",
        "**/__pycache__/**", "**/.venv/**", "**/venv/**",
        "**/*.spec.ts", "**/*.spec.tsx", "**/*.spec.js",
        "**/*.test.ts", "**/*.test.tsx", "**/*.test.js",
        "**/*_test.py", "**/*_test.go", "**/*_test.rs",
        "**/*.d.ts",
    ];
    let mut builder = globset::GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = globset::Glob::new(p) { builder.add(g); }
    }
    builder.build().unwrap_or_else(|_| globset::GlobSetBuilder::new().build().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_binary_ext_recognises_binaries() {
        assert!(is_binary_ext("png"));
        assert!(is_binary_ext("exe"));
        assert!(is_binary_ext("wasm"));
        assert!(is_binary_ext("sqlite3"));
        assert!(is_binary_ext("lock"));
    }

    #[test]
    fn is_binary_ext_rejects_source_extensions() {
        assert!(!is_binary_ext("rs"));
        assert!(!is_binary_ext("ts"));
        assert!(!is_binary_ext("py"));
        assert!(!is_binary_ext("md"));
        assert!(!is_binary_ext("json"));
        assert!(!is_binary_ext(""));
    }

    #[test]
    fn build_globset_matches_excluded_paths() {
        let gs = build_globset();
        assert!(gs.is_match("node_modules/foo/bar.js"));
        assert!(gs.is_match("src/foo.spec.ts"));
        assert!(gs.is_match("src/foo.test.tsx"));
        assert!(gs.is_match("tests/foo_test.py"));
        assert!(gs.is_match("pkg/foo_test.go"));
        assert!(gs.is_match("src/types.d.ts"));
        assert!(gs.is_match("dist/bundle.js"));
        assert!(gs.is_match("target/debug/foo"));
        assert!(gs.is_match("__pycache__/foo.pyc"));
    }

    #[test]
    fn build_globset_allows_normal_source_files() {
        let gs = build_globset();
        assert!(!gs.is_match("src/main.rs"));
        assert!(!gs.is_match("lib/utils.ts"));
        assert!(!gs.is_match("app.py"));
        assert!(!gs.is_match("docs/readme.md"));
    }
}
