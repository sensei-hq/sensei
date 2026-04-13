use crate::types::{ParsedFile, ParsedSymbol, ParsedImport, ParsedEdge, SymbolKind};
use super::LanguageAdapter;

pub struct SvelteAdapter;

impl LanguageAdapter for SvelteAdapter {
    fn language(&self) -> &str { "svelte" }
    fn extensions(&self) -> &[&str] { &[".svelte"] }

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
                if sym.kind == SymbolKind::Const || sym.kind == SymbolKind::Function {
                    if let Some(sig) = &sym.signature {
                        if sig.contains("$state") || sig.contains("$derived") || sig.contains("$effect") {
                            sym.kind = SymbolKind::Hook;
                        }
                    }
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

/// Extract script blocks from Svelte SFC. Returns (source, line_offset, is_typescript).
fn extract_script_blocks(source: &str) -> Vec<(String, u32, bool)> {
    let mut blocks = Vec::new();
    let mut remaining = source;
    let mut global_offset = 0u32;

    while let Some(start_idx) = remaining.find("<script") {
        let lines_before = remaining[..start_idx].matches('\n').count() as u32;
        // Find end of opening tag
        let after_tag = &remaining[start_idx..];
        let tag_end = match after_tag.find('>') {
            Some(i) => i,
            None => break,
        };
        let tag = &after_tag[..tag_end];
        let is_ts = tag.contains("lang=\"ts\"") || tag.contains("lang='ts'");

        // Content starts after '>'
        let content_start = start_idx + tag_end + 1;
        let content_after = &remaining[content_start..];

        let close_idx = match content_after.find("</script>") {
            Some(i) => i,
            None => break,
        };

        let script_content = &content_after[..close_idx];
        let line_offset = global_offset + lines_before + remaining[start_idx..content_start].matches('\n').count() as u32;

        blocks.push((script_content.to_string(), line_offset, is_ts));

        let advance = content_start + close_idx + "</script>".len();
        global_offset += remaining[..advance].matches('\n').count() as u32;
        remaining = &remaining[advance..];
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile { SvelteAdapter.parse(src, "App.svelte") }

    #[test]
    fn svelte_component_name() {
        let pf = parse("<script>\nlet x = 1;\n</script>\n<div>{x}</div>");
        assert_eq!(pf.symbols[0].name, "App");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Component);
    }

    #[test]
    fn svelte_script_extraction() {
        let pf = parse("<script lang=\"ts\">\nimport { onMount } from 'svelte';\nfunction hello() { return 1; }\n</script>");
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert_eq!(pf.imports.len(), 1);
        assert_eq!(pf.imports[0].target_path, "svelte");
    }

    #[test]
    fn svelte_rune_detection() {
        let pf = parse("<script lang=\"ts\">\nlet count = $state(0);\nlet doubled = $derived(count * 2);\n</script>");
        let hooks: Vec<&str> = pf.symbols.iter()
            .filter(|s| s.kind == SymbolKind::Hook)
            .map(|s| s.name.as_str())
            .collect();
        assert!(hooks.contains(&"count") || hooks.contains(&"doubled"),
            "Expected rune-based symbols to be detected as hooks, got: {:?}", hooks);
    }

    #[test]
    fn svelte_language() {
        let pf = parse("<div>hello</div>");
        assert_eq!(pf.language, "svelte");
    }

    #[test]
    fn extract_blocks() {
        let blocks = extract_script_blocks("<script lang=\"ts\">\ncode1\n</script>\n<template/>\n<script context=\"module\">\ncode2\n</script>");
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].2); // is_ts
        assert!(blocks[0].0.contains("code1"));
        assert!(blocks[1].0.contains("code2"));
    }
}
