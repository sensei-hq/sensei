//! Intermediate Representation (IR) — the common output format for all adapters.
//!
//! Three node types: IRDoc, IRModule, IRClass — each with shared IRBase.
//! Adapters produce ParsedFile containing any combination of these.
//! Graph writer and downstream processors work on the IR, not adapter-specific output.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Base ────────────────────────────────────────────────────────────────────

/// Fields common to every node in the IR — code, docs, config all share these.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRBase {
    pub name: String,
    pub file: String,              // repo-relative path
    pub line_start: u32,
    pub line_end: u32,

    // Classification (universal)
    pub extension: Option<String>,  // .rs, .md, .py, .svelte
    pub language: Option<String>,   // rust, markdown, python, svelte
    pub framework: Option<String>,  // sveltekit, axum, express, django
    pub node_type: Option<String>,  // function, class, doc, config, test, module
    pub category: Option<String>,   // design, feature, adapter, handler, utility

    // Content
    pub docstring: Option<String>,
    pub is_exported: bool,
    pub tags: Vec<String>,
}

// ── Document ────────────────────────────────────────────────────────────────

/// A document — markdown, text, yaml, config.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRDoc {
    pub base: IRBase,

    // Classification
    pub doc_type: Option<String>,       // idea, analysis, blueprint, design, feature, etc.

    // Frontmatter (all optional — may be absent initially)
    pub frontmatter: HashMap<String, String>,  // raw key-value pairs
    pub status: Option<String>,
    pub origin: Option<String>,         // parent doc path (for traceability)
    pub date: Option<String>,
    pub description: Option<String>,

    // Structure
    pub title: Option<String>,
    pub sections: Vec<IRSection>,
    pub code_blocks: Vec<IRCodeBlock>,

    // References found in content
    pub file_references: Vec<String>,    // backtick file paths → COVERS edges
    pub symbol_references: Vec<String>,  // backtick function/type names → MENTIONS edges
    pub doc_references: Vec<String>,     // links to other docs → TRACES_TO edges
}

/// A section of a document — split by headings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRSection {
    pub heading: String,
    pub level: u8,
    pub line_start: u32,
    pub line_end: u32,
    pub content_preview: Option<String>,  // first 200 chars
}

/// A code block within a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRCodeBlock {
    pub language: Option<String>,
    pub content: String,
    pub line_start: u32,
    pub line_end: u32,
}

// ── Module (functional) ─────────────────────────────────────────────────────

/// A module — a file or namespace containing functions, constants, imports.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRModule {
    pub base: IRBase,

    pub functions: Vec<IRFunction>,
    pub constants: Vec<IRConstant>,
    pub imports: Vec<IRImport>,
    pub type_aliases: Vec<IRTypeAlias>,

    pub is_test: bool,
    pub is_config: bool,
}

/// A free function at module level.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRFunction {
    pub base: IRBase,
    pub params: Vec<IRParam>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub decorators: Vec<String>,
    pub calls: Vec<String>,            // unresolved function names called
    pub complexity: u32,
    pub body_hash: Option<String>,
}

/// A function/method parameter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRParam {
    pub name: String,
    pub type_: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
}

/// An import statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRImport {
    pub source: String,
    pub names: Vec<String>,
    pub is_reexport: bool,
}

/// A constant value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRConstant {
    pub base: IRBase,
    pub type_: Option<String>,
    pub value_preview: Option<String>,
}

/// A type alias.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRTypeAlias {
    pub base: IRBase,
    pub target: String,
}

// ── Class (OO) ──────────────────────────────────────────────────────────────

/// A class, struct, interface, trait, or enum.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRClass {
    pub base: IRBase,
    pub class_kind: ClassKind,

    pub methods: Vec<IRMethod>,
    pub properties: Vec<IRProperty>,
    pub constants: Vec<IRConstant>,

    // Relationships (names, resolved to IDs later)
    pub implements: Vec<String>,
    pub extends: Option<String>,
    pub mixins: Vec<String>,

    pub decorators: Vec<String>,
    pub generic_params: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ClassKind {
    #[default]
    Class,
    Struct,
    Interface,
    Trait,
    Enum,
    Component,
    Protocol,
}

/// A method — function belonging to a class.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRMethod {
    pub base: IRBase,
    pub params: Vec<IRParam>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub is_static: bool,
    pub is_abstract: bool,
    pub visibility: Visibility,
    pub decorators: Vec<String>,
    pub calls: Vec<String>,
    pub complexity: u32,
    pub body_hash: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
    Internal,
}

/// A class property/field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRProperty {
    pub base: IRBase,
    pub type_: Option<String>,
    pub is_readonly: bool,
    pub visibility: Visibility,
}

// ── Parsed File (adapter output) ────────────────────────────────────────────

/// The complete IR output for one file. An adapter returns this.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IRParsedFile {
    pub file_path: String,
    pub language: String,

    pub modules: Vec<IRModule>,
    pub classes: Vec<IRClass>,
    pub docs: Vec<IRDoc>,

    pub is_test_file: bool,
    pub is_config_file: bool,
    pub file_hash: Option<String>,
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_base_defaults() {
        let base = IRBase::default();
        assert_eq!(base.name, "");
        assert!(!base.is_exported);
        assert!(base.tags.is_empty());
        assert!(base.language.is_none());
        assert!(base.framework.is_none());
    }

    #[test]
    fn ir_doc_with_frontmatter() {
        let mut fm = HashMap::new();
        fm.insert("name".into(), "Workflow System".into());
        fm.insert("status".into(), "complete".into());
        fm.insert("origin".into(), "docs/ideas/01.md".into());

        let doc = IRDoc {
            base: IRBase {
                name: "Workflow System".into(),
                file: "docs/ideas/01-workflow.md".into(),
                extension: Some(".md".into()),
                language: Some("markdown".into()),
                node_type: Some("doc".into()),
                category: Some("idea".into()),
                ..Default::default()
            },
            doc_type: Some("idea".into()),
            frontmatter: fm,
            status: Some("complete".into()),
            origin: Some("docs/ideas/01.md".into()),
            title: Some("Workflow System".into()),
            ..Default::default()
        };

        assert_eq!(doc.base.name, "Workflow System");
        assert_eq!(doc.doc_type, Some("idea".into()));
        assert_eq!(doc.frontmatter["status"], "complete");
        assert_eq!(doc.origin, Some("docs/ideas/01.md".into()));
    }

    #[test]
    fn ir_doc_without_frontmatter() {
        let doc = IRDoc {
            base: IRBase {
                name: "README".into(),
                file: "README.md".into(),
                ..Default::default()
            },
            title: Some("Project Readme".into()),
            ..Default::default()
        };

        assert!(doc.frontmatter.is_empty());
        assert!(doc.status.is_none());
        assert!(doc.origin.is_none());
        assert_eq!(doc.title, Some("Project Readme".into()));
    }

    #[test]
    fn ir_doc_with_sections() {
        let doc = IRDoc {
            sections: vec![
                IRSection { heading: "Problem".into(), level: 2, line_start: 5, line_end: 15, content_preview: Some("AI-assisted dev...".into()) },
                IRSection { heading: "Solution".into(), level: 2, line_start: 16, line_end: 30, content_preview: Some("Sensei provides...".into()) },
            ],
            ..Default::default()
        };

        assert_eq!(doc.sections.len(), 2);
        assert_eq!(doc.sections[0].heading, "Problem");
        assert_eq!(doc.sections[0].level, 2);
        assert_eq!(doc.sections[1].line_start, 16);
    }

    #[test]
    fn ir_doc_with_references() {
        let doc = IRDoc {
            file_references: vec!["src/main.rs".into(), "src/lib.rs".into()],
            symbol_references: vec!["parse_file".into()],
            doc_references: vec!["docs/ideas/01.md".into()],
            ..Default::default()
        };

        assert_eq!(doc.file_references.len(), 2);
        assert_eq!(doc.symbol_references[0], "parse_file");
        assert_eq!(doc.doc_references[0], "docs/ideas/01.md");
    }

    #[test]
    fn ir_function_with_params() {
        let func = IRFunction {
            base: IRBase {
                name: "parse_file".into(),
                node_type: Some("function".into()),
                ..Default::default()
            },
            params: vec![
                IRParam { name: "path".into(), type_: Some("PathBuf".into()), ..Default::default() },
                IRParam { name: "content".into(), type_: Some("&str".into()), ..Default::default() },
            ],
            return_type: Some("Vec<Symbol>".into()),
            is_async: false,
            complexity: 5,
            ..Default::default()
        };

        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0].name, "path");
        assert_eq!(func.params[0].type_, Some("PathBuf".into()));
        assert_eq!(func.return_type, Some("Vec<Symbol>".into()));
        assert_eq!(func.complexity, 5);
    }

    #[test]
    fn ir_class_with_methods_and_implements() {
        let class = IRClass {
            base: IRBase {
                name: "RustAdapter".into(),
                node_type: Some("class".into()),
                category: Some("adapter".into()),
                ..Default::default()
            },
            class_kind: ClassKind::Struct,
            implements: vec!["LanguageAdapter".into()],
            methods: vec![
                IRMethod {
                    base: IRBase { name: "parse".into(), ..Default::default() },
                    params: vec![
                        IRParam { name: "source".into(), type_: Some("&str".into()), ..Default::default() },
                    ],
                    return_type: Some("ParsedFile".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert_eq!(class.class_kind, ClassKind::Struct);
        assert_eq!(class.implements, vec!["LanguageAdapter"]);
        assert_eq!(class.methods.len(), 1);
        assert_eq!(class.methods[0].base.name, "parse");
    }

    #[test]
    fn ir_method_visibility() {
        let method = IRMethod {
            visibility: Visibility::Private,
            is_static: true,
            is_abstract: false,
            ..Default::default()
        };

        assert_eq!(method.visibility, Visibility::Private);
        assert!(method.is_static);
        assert!(!method.is_abstract);
    }

    #[test]
    fn ir_parsed_file_mixed_content() {
        let pf = IRParsedFile {
            file_path: "src/lib.rs".into(),
            language: "rust".into(),
            modules: vec![IRModule {
                base: IRBase { name: "lib".into(), ..Default::default() },
                functions: vec![IRFunction {
                    base: IRBase { name: "helper".into(), ..Default::default() },
                    ..Default::default()
                }],
                ..Default::default()
            }],
            classes: vec![IRClass {
                base: IRBase { name: "MyStruct".into(), ..Default::default() },
                class_kind: ClassKind::Struct,
                ..Default::default()
            }],
            ..Default::default()
        };

        assert_eq!(pf.modules.len(), 1);
        assert_eq!(pf.classes.len(), 1);
        assert_eq!(pf.modules[0].functions[0].base.name, "helper");
        assert_eq!(pf.classes[0].class_kind, ClassKind::Struct);
    }

    #[test]
    fn ir_serializes_to_json() {
        let doc = IRDoc {
            base: IRBase { name: "test".into(), ..Default::default() },
            doc_type: Some("idea".into()),
            ..Default::default()
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"doc_type\":\"idea\""));

        // Deserialize back
        let parsed: IRDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.base.name, "test");
    }

    #[test]
    fn ir_import_struct() {
        let imp = IRImport {
            source: "std::path::PathBuf".into(),
            names: vec!["PathBuf".into()],
            is_reexport: false,
        };

        assert_eq!(imp.source, "std::path::PathBuf");
        assert!(!imp.is_reexport);
    }

    #[test]
    fn ir_constant_with_preview() {
        let c = IRConstant {
            base: IRBase { name: "MAX_RETRIES".into(), is_exported: true, ..Default::default() },
            type_: Some("u32".into()),
            value_preview: Some("3".into()),
        };

        assert!(c.base.is_exported);
        assert_eq!(c.type_, Some("u32".into()));
    }
}
