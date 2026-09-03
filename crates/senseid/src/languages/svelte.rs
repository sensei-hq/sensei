use super::LanguageAdapter;
use super::common::extract_script_blocks;
use crate::ir::IRParsedFile;
use crate::types::{ParsedFile, ParsedSymbol, SymbolKind};

pub struct SvelteAdapter;

impl LanguageAdapter for SvelteAdapter {
    fn supports_fqn(&self) -> bool {
        true
    }

    fn extensions(&self) -> &[&'static str] {
        &[".svelte"]
    }

    /// Composes over TypeScript: `<script>` blocks are handed to the TS
    /// adapter, which is why this file type's symbols carry `typescript·` fqns.
    fn host_language(&self) -> Option<&'static str> {
        Some("typescript")
    }

    fn language(&self) -> &str {
        "svelte"
    }

    fn fqn_output(
        &self,
        abs_path: &str,
        _rel_path: &str,
        content: &str,
    ) -> Option<super::fqn::FqnFileOutput> {
        super::common::sfc_fqn_output(abs_path, content)
    }

    fn parse_to_ir(&self, source: &str, file_path: &str) -> crate::ir::IRParsedFile {
        parse_to_ir(source, file_path)
    }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        // Extract <script> block(s) and parse with oxc (TypeScript)
        let mut all_symbols = Vec::new();
        let mut all_imports = Vec::new();

        // Detect component name from filename
        let component_name = std::path::Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Component")
            .to_string();

        // Add component symbol
        all_symbols.push(ParsedSymbol {
            name: component_name,
            kind: SymbolKind::Component,
            signature: None,
            docstring: None,
            line_start: 1,
            line_end: source.lines().count() as u32,
            is_exported: true,
            parent: None,
        });

        for (script_src, offset, is_ts) in extract_script_blocks(source) {
            let ext = if is_ts { "script.ts" } else { "script.js" };
            let ts_adapter = super::typescript::TypeScriptAdapter;
            let parsed = ts_adapter.parse(&script_src, ext);

            for mut sym in parsed.symbols {
                // Adjust line numbers by script block offset
                sym.line_start += offset;
                sym.line_end += offset;
                // Detect Svelte 5 runes
                if (sym.kind == SymbolKind::Const || sym.kind == SymbolKind::Function)
                    && let Some(sig) = &sym.signature
                    && (sig.contains("$state")
                        || sig.contains("$derived")
                        || sig.contains("$effect"))
                {
                    sym.kind = SymbolKind::Hook;
                }
                all_symbols.push(sym);
            }
            all_imports.extend(parsed.imports);
        }

        ParsedFile {
            file_path: file_path.to_string(),
            language: "svelte".to_string(),
            symbols: all_symbols,
            edges: vec![],
            imports: all_imports,
        }
    }
}

/// Parse Svelte SFC into IR — delegates script blocks to TypeScript adapter.
pub fn parse_to_ir(source: &str, file_path: &str) -> IRParsedFile {
    let blocks = extract_script_blocks(source);
    if blocks.is_empty() {
        return IRParsedFile {
            file_path: file_path.into(),
            language: "svelte".into(),
            ..Default::default()
        };
    }
    // Parse the first (main) script block with TS adapter
    let script = &blocks[0].0;
    let ext = if blocks[0].2 { "component.svelte.ts" } else { "component.svelte.js" };
    let mut ir = super::typescript::parse_to_ir(script, ext);
    ir.file_path = file_path.into();
    ir.language = "svelte".into();
    ir
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile {
        SvelteAdapter.parse(src, "App.svelte")
    }

    #[test]
    fn svelte_script_fqn() {
        // Phase 6.8: an SFC's <script> block is TypeScript — its defs/refs get FQNs
        // in the file's module (the component), reusing the TS producer.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("package.json"), "{\"name\":\"app\"}").unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        let file = root.join("src/Card.svelte");
        let content = "<script lang=\"ts\">\nimport { helper } from './util';\nexport function build() { helper(); }\n</script>\n<div>hi</div>\n";
        std::fs::write(&file, content).unwrap();
        let out = SvelteAdapter
            .fqn_output(&file.to_string_lossy(), "src/lib/Comp.svelte", content)
            .unwrap();
        assert!(
            out.defs.iter().any(|d| d.fqn == "typescript·app·Card·build"),
            "component script def, got: {:?}",
            out.defs.iter().map(|d| &d.fqn).collect::<Vec<_>>()
        );
        assert!(
            out.refs.iter().any(|r| r.target_fqn.as_deref() == Some("typescript·app·util·helper")),
            "relative import resolved from the SFC, got: {:?}",
            out.refs
        );
    }

    #[test]
    fn svelte_component_name() {
        let pf = parse("<script>\nlet x = 1;\n</script>\n<div>{x}</div>");
        assert_eq!(pf.symbols[0].name, "App");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Component);
    }

    #[test]
    fn svelte_script_extraction() {
        let pf = parse(
            "<script lang=\"ts\">\nimport { onMount } from 'svelte';\nfunction hello() { return 1; }\n</script>",
        );
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert_eq!(pf.imports.len(), 1);
        assert_eq!(pf.imports[0].target_path, "svelte");
    }

    #[test]
    fn svelte_rune_detection() {
        let pf = parse(
            "<script lang=\"ts\">\nlet count = $state(0);\nlet doubled = $derived(count * 2);\n</script>",
        );
        let hooks: Vec<&str> = pf
            .symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Hook)
            .map(|s| s.name.as_str())
            .collect();
        assert!(
            hooks.contains(&"count") || hooks.contains(&"doubled"),
            "Expected rune-based symbols to be detected as hooks, got: {:?}",
            hooks
        );
    }

    #[test]
    fn svelte_language() {
        let pf = parse("<div>hello</div>");
        assert_eq!(pf.language, "svelte");
    }

    #[test]
    fn extract_blocks() {
        let blocks = extract_script_blocks(
            "<script lang=\"ts\">\ncode1\n</script>\n<template/>\n<script context=\"module\">\ncode2\n</script>",
        );
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].2); // is_ts
        assert!(blocks[0].0.contains("code1"));
        assert!(blocks[1].0.contains("code2"));
    }
}
