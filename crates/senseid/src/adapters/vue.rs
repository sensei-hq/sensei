use super::common::extract_script_blocks;
use crate::types::{ParsedFile, ParsedSymbol, SymbolKind};
use super::LanguageAdapter;

pub struct VueAdapter;

impl LanguageAdapter for VueAdapter {
    fn language(&self) -> &str { "vue" }

    fn parse(&self, source: &str, file_path: &str) -> ParsedFile {
        let mut all_symbols = Vec::new();
        let mut all_imports = Vec::new();

        // Component name from filename
        let component_name = std::path::Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Component")
            .to_string();

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
                sym.line_start += offset;
                sym.line_end += offset;
                // Detect Vue composables (useXxx pattern)
                if (sym.kind == SymbolKind::Function || sym.kind == SymbolKind::Const)
                    && sym.name.starts_with("use") && sym.name.len() > 3
                    && sym.name.chars().nth(3).map_or(false, |c| c.is_uppercase())
                {
                    sym.kind = SymbolKind::Hook;
                }
                all_symbols.push(sym);
            }
            all_imports.extend(parsed.imports);
        }

        ParsedFile {
            file_path: file_path.to_string(),
            language: "vue".to_string(),
            symbols: all_symbols,
            edges: vec![],
            imports: all_imports,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> ParsedFile { VueAdapter.parse(src, "App.vue") }

    #[test]
    fn vue_component_name() {
        let pf = parse("<script setup>\nconst msg = 'hello'\n</script>\n<template><div>{{ msg }}</div></template>");
        assert_eq!(pf.symbols[0].name, "App");
        assert_eq!(pf.symbols[0].kind, SymbolKind::Component);
    }

    #[test]
    fn vue_script_setup() {
        let pf = parse("<script setup lang=\"ts\">\nimport { ref } from 'vue';\nconst count = ref(0);\nfunction increment() { count.value++ }\n</script>");
        let names: Vec<&str> = pf.symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"increment"));
        assert!(names.contains(&"count"));
        assert_eq!(pf.imports[0].target_path, "vue");
    }

    #[test]
    fn vue_composable_detection() {
        let pf = parse("<script setup lang=\"ts\">\nfunction useCounter() { return { count: 0 } }\n</script>");
        let hooks: Vec<&str> = pf.symbols.iter()
            .filter(|s| s.kind == SymbolKind::Hook)
            .map(|s| s.name.as_str())
            .collect();
        assert!(hooks.contains(&"useCounter"));
    }

    #[test]
    fn vue_language() {
        let pf = parse("<template><div/></template>");
        assert_eq!(pf.language, "vue");
    }
}
