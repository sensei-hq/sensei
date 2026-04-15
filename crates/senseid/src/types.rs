use serde::{Deserialize, Serialize};

// ── Symbol kinds ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Type,
    Interface,
    Enum,
    Const,
    Method,
    Component,
    Hook,
    Unknown,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "function"),
            SymbolKind::Class => write!(f, "class"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Type => write!(f, "type"),
            SymbolKind::Interface => write!(f, "interface"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Const => write!(f, "const"),
            SymbolKind::Method => write!(f, "method"),
            SymbolKind::Component => write!(f, "component"),
            SymbolKind::Hook => write!(f, "hook"),
            SymbolKind::Unknown => write!(f, "unknown"),
        }
    }
}

// ── Parsed file output ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub file_path: String,
    pub language: String,
    pub symbols: Vec<ParsedSymbol>,
    pub edges: Vec<ParsedEdge>,
    pub imports: Vec<ParsedImport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub is_exported: bool,
    /// Parent class/struct name for methods (e.g. "Foo" for Foo.bar).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEdge {
    pub caller_name: String,
    pub callee_name: String,
    pub callee_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedImport {
    pub target_path: String,
    pub names: Vec<String>,
}

// ── Project & Solution ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub repo_id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
    #[serde(default)]
    pub stack: Vec<String>,
    #[serde(default)]
    pub libs: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "active".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub repos: Vec<SolutionRepo>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

fn default_category() -> String {
    "active".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolutionRepo {
    pub repo_id: String,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

fn default_role() -> String {
    "unknown".to_string()
}

// ── Package / Module info ────────────────────────────────────────────────────

/// A workspace member / crate / sub-package discovered inside a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// npm_workspace, cargo_crate, pip_package, go_module, etc.
    pub pkg_type: String,
}

// ── Indexing ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexProgress {
    pub repo_id: String,
    pub current_file: String,
    pub files_processed: u32,
    pub files_total: u32,
    pub files_unchanged: u32,
    pub files_skipped: u32,
    pub files_failed: u32,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    pub files_indexed: u32,
    pub files_skipped: u32,
    pub files_failed: u32,
    pub functions_indexed: u32,
    pub types_indexed: u32,
    pub packages_indexed: u32,
    pub modules_indexed: u32,
    pub edges_created: u32,
    pub docs_indexed: u32,
    pub libs: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexError {
    pub repo_id: String,
    pub file_path: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    pub timestamp: String,
}

// ── Manifest ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub mtime: u64,
    pub hash: String,
}

// ── Graph query results ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub complexity: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    #[serde(rename = "type")]
    pub edge_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDetail {
    pub id: String,
    pub name: String,
    pub file: String,
    pub line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docstring: Option<String>,
    pub complexity: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDetail {
    pub id: String,
    pub name: String,
    pub file: String,
    pub line: u32,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSummary {
    pub total_symbols: u32,
    pub total_edges: u32,
    pub communities: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_kind_display() {
        assert_eq!(SymbolKind::Function.to_string(), "function");
        assert_eq!(SymbolKind::Class.to_string(), "class");
        assert_eq!(SymbolKind::Method.to_string(), "method");
    }

    #[test]
    fn project_serialization() {
        let p = Project {
            repo_id: "test".into(),
            name: "test".into(),
            path: "/tmp/test".into(),
            remote_url: None,
            indexed_at: None,
            last_error: None,
            duplicate_of: None,
            stack: vec!["typescript".into()],
            libs: vec![],
            tags: vec![],
            status: "active".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"repo_id\":\"test\""));
        // Optional None fields should be skipped
        assert!(!json.contains("remote_url"));
    }

    #[test]
    fn solution_defaults() {
        let json = r#"{"id":"1","name":"Test","repos":[]}"#;
        let s: Solution = serde_json::from_str(json).unwrap();
        assert_eq!(s.category, "active");
        assert!(s.client.is_none());
        assert!(s.tags.is_empty());
    }

    #[test]
    fn parsed_file_roundtrip() {
        let pf = ParsedFile {
            file_path: "src/main.py".into(),
            language: "python".into(),
            symbols: vec![ParsedSymbol {
                name: "hello".into(),
                kind: SymbolKind::Function,
                signature: Some("def hello(name: str) -> str".into()),
                docstring: Some("Say hello".into()),
                line_start: 1,
                line_end: 3,
                is_exported: true,
                parent: None,
            }],
            edges: vec![],
            imports: vec![ParsedImport {
                target_path: "os".into(),
                names: vec!["path".into()],
            }],
        };
        let json = serde_json::to_string(&pf).unwrap();
        let pf2: ParsedFile = serde_json::from_str(&json).unwrap();
        assert_eq!(pf2.symbols.len(), 1);
        assert_eq!(pf2.symbols[0].name, "hello");
        assert_eq!(pf2.imports[0].target_path, "os");
    }
}
