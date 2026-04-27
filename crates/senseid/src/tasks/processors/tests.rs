//! Tests for file processors using actual repo files as fixtures.

#[cfg(test)]
mod tests {
    use crate::tasks::processors::process_file;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf()
    }

    fn process(rel: &str) -> crate::tasks::processors::FileProcessResult {
        let root = repo_root();
        let abs = root.join(rel);
        assert!(abs.exists(), "Test file not found: {}", abs.display());
        process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap()
    }

    fn process_subtree(rel: &str, subtree: &str) -> crate::tasks::processors::FileProcessResult {
        let root = repo_root();
        let subtree_root = root.join(subtree);
        let abs = root.join(rel);
        assert!(abs.exists(), "Test file not found: {}", abs.display());
        process_file(&abs.to_string_lossy(), &subtree_root.to_string_lossy(), &format!("sensei:{}", subtree)).unwrap()
    }

    // ═══ Code files ═══════════════════════════════════════════════════

    #[test]
    fn rust_adapter_svelte_rs() {
        let r = process("crates/senseid/src/languages/svelte.rs");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
        assert_eq!(r.language.as_deref(), Some("rust"));
        assert!(r.symbols.len() > 0);
        let names: Vec<&str> = r.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"SvelteAdapter"), "should find SvelteAdapter struct");
    }

    #[test]
    fn svelte_component() {
        let r = process("apps/desktop/src/lib/setup/wizard/StepHeader.svelte");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
        assert_eq!(r.language.as_deref(), Some("svelte"));
        let kinds: Vec<&str> = r.symbols.iter().map(|s| s.kind.as_str()).collect();
        assert!(kinds.contains(&"component"), "should find component");
    }

    #[test]
    fn svelte_ts_appstate() {
        let r = process("apps/desktop/src/lib/appstate.svelte.ts");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
        assert!(r.symbols.len() > 0, "should extract symbols from .svelte.ts");
    }

    #[test]
    fn svelte_page_route() {
        let r = process("apps/desktop/src/routes/(app)/observatory/+page.svelte");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "src");
    }

    // ═══ Doc files ════════════════════════════════════════════════════

    #[test]
    fn design_doc() {
        let r = process("docs/design/01-daemon/architecture.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.tags, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("design"));
        assert!(r.title.is_some());
    }

    #[test]
    fn feature_doc() {
        let r = process("docs/features/01-workflow-commands.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("feature"));
    }

    #[test]
    fn idea_doc() {
        let r = process("docs/ideas/01-workflow-system.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.tags, "doc");
    }

    #[test]
    fn gap_analysis_doc() {
        let r = process("docs/reference/gap-analysis.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.tags, "doc");
    }

    #[test]
    fn root_readme() {
        let r = process("README.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("usage"));
    }

    #[test]
    fn homebrew_readme() {
        let r = process_subtree("homebrew/README.md", "homebrew");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("usage"));
    }

    // ═══ Marketplace / Extension files ════════════════════════════════

    #[test]
    fn marketplace_skill_md() {
        let r = process_subtree(
            "marketplace/plugins/sensei/skills/analyze/SKILL.md",
            "marketplace",
        );
        assert_eq!(r.kind, "extension");
        assert_eq!(r.tags, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("extension"));
    }

    // ═══ Config files ═════════════════════════════════════════════════

    #[test]
    fn desktop_package_json() {
        let r = process("apps/desktop/package.json");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
    }

    #[test]
    fn sensei_mcp_cargo_toml() {
        let r = process("crates/sensei-mcp/Cargo.toml");
        assert_eq!(r.kind, "file");
        assert_eq!(r.tags, "config");
    }

    // ═══ Non-code files (no adapter) ══════════════════════════════════

    #[test]
    fn c_parser_large_file() {
        // parser.c is a 678K-line generated file — C adapter skips it by size
        let root = repo_root();
        let abs = root.join("crates/senseid/grammars/kotlin/src/parser.c");
        if abs.exists() {
            let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei").unwrap();
            assert_eq!(r.kind, "file");
            assert_eq!(r.tags, "src");
            assert!(r.symbols.is_empty(), "large generated C file should have no symbols");
        }
    }

    #[test]
    fn marketplace_hooks_json() {
        let root = repo_root();
        let abs = root.join("marketplace/plugins/sensei/hooks/hooks.json");
        if abs.exists() {
            let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei:marketplace").unwrap();
            assert_eq!(r.kind, "file");
            assert_eq!(r.tags, "config");
        }
    }

    #[test]
    fn marketplace_plugin_config() {
        let root = repo_root();
        let abs = root.join("marketplace/plugins/sensei-mcp/config.json");
        if abs.exists() {
            let r = process_file(&abs.to_string_lossy(), &root.to_string_lossy(), "sensei:marketplace").unwrap();
            assert_eq!(r.kind, "file");
            assert_eq!(r.tags, "config");
        }
    }

    // ═══ Tag classification ═══════════════════════════════════════════

    #[test]
    fn test_file_detected() {
        use crate::tasks::processors::types::classify_file_tag;
        assert_eq!(classify_file_tag("src/main.spec.ts", "ts"), "test");
        assert_eq!(classify_file_tag("src/main.test.ts", "ts"), "test");
        assert_eq!(classify_file_tag("tests/unit/foo.rs", "rs"), "test");
        assert_eq!(classify_file_tag("src/__tests__/foo.ts", "ts"), "test");
        assert_eq!(classify_file_tag("test_helper.py", "py"), "test");
    }

    #[test]
    fn e2e_file_detected() {
        use crate::tasks::processors::types::classify_file_tag;
        assert_eq!(classify_file_tag("e2e/login.spec.ts", "ts"), "e2e");
        assert_eq!(classify_file_tag("src/app.e2e.ts", "ts"), "e2e");
    }

    #[test]
    fn config_file_detected() {
        use crate::tasks::processors::types::classify_file_tag;
        assert_eq!(classify_file_tag("package.json", "json"), "config");
        assert_eq!(classify_file_tag("Cargo.toml", "toml"), "config");
        assert_eq!(classify_file_tag("config.yaml", "yaml"), "config");
    }

    #[test]
    fn src_file_default() {
        use crate::tasks::processors::types::classify_file_tag;
        assert_eq!(classify_file_tag("src/main.rs", "rs"), "src");
        assert_eq!(classify_file_tag("lib/utils.ts", "ts"), "src");
    }
}
