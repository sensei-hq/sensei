use std::path::Path;
use std::collections::HashSet;
use walkdir::WalkDir;
use crate::indexer::graph::GraphDb;
use crate::types::{NodeKind, HierarchyNode};

const DOC_EXTENSIONS: &[&str] = &["md", "mdx"];

/// Frontmatter parsed from YAML between --- fences.
#[derive(Default, serde::Deserialize)]
pub struct DocFrontmatter {
    #[serde(rename = "type")]
    pub doc_type: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Classification result for a doc/extension file.
pub struct DocClassification {
    pub kind: NodeKind,     // Doc or Extension
    pub doc_type: String,   // requirement, design, feature, usage, changelog, extension, doc
    pub doc_category: Option<String>, // sub-category from frontmatter
}

/// Index documentation files (.md, .mdx) into the graph.
/// Classifies docs by frontmatter + path heuristics.
/// Marketplace files (skills/commands/plugins) are classified as extensions.
pub fn index_docs(
    graph_db: &GraphDb,
    repo_path: &str,
    repo_id: &str,
) -> Result<u32, String> {
    let repo = Path::new(repo_path);
    let mut docs_indexed = 0u32;

    let doc_files: Vec<_> = WalkDir::new(repo)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0 || {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let path = e.path().to_string_lossy();
            !path.contains("node_modules") && !path.contains("/dist/") && !path.contains("/target/")
        })
        .filter(|e| {
            e.path().extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| DOC_EXTENSIONS.contains(&ext))
                .unwrap_or(false)
        })
        .collect();

    // Prune stale docs/extensions
    {
        let current_doc_paths: HashSet<String> = doc_files.iter()
            .map(|e| format!("doc:{}", e.path().to_string_lossy()))
            .collect();
        let existing = graph_db.get_nodes(repo_id).unwrap_or_default();
        for node in &existing {
            if (node.kind == "doc" || node.kind == "extension") && !current_doc_paths.contains(&node.id) {
                graph_db.delete_node(&node.id).ok();
            }
        }
    }

    for entry in &doc_files {
        let abs_path = entry.path().to_string_lossy().to_string();
        let rel_path = entry.path().strip_prefix(repo)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .to_string();

        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let frontmatter = parse_frontmatter(&content);
        let classification = classify_doc(&rel_path, &frontmatter);

        let title = frontmatter.title
            .or(frontmatter.name)
            .or_else(|| extract_title(&content))
            .unwrap_or_else(|| rel_path.clone());

        let doc_id = format!("doc:{}", abs_path);

        let node = HierarchyNode::doc(
            doc_id.clone(),
            title,
            classification.kind,
            abs_path.clone(),
            Some(classification.doc_type),
            classification.doc_category,
            repo_id.into(),
        );
        graph_db.merge_node(&node)?;

        // Extract file references from the doc content
        let file_refs = extract_file_refs(&content, repo_path);
        for file_ref in &file_refs {
            let file_id = format!("file:{}", file_ref);
            graph_db.merge_edge(&doc_id, &file_id, "COVERS").ok();
        }

        // Extract function mentions (backtick identifiers)
        let fn_mentions = extract_fn_mentions(&content);
        for fn_name in &fn_mentions {
            if let Ok(Some(fn_id)) = graph_db.find_function_by_name(fn_name, repo_id) {
                graph_db.merge_edge(&doc_id, &fn_id, "MENTIONS_FN").ok();
            }
        }

        docs_indexed += 1;
    }

    // Traceability edges between doc stages
    create_traceability_edges(graph_db, repo_id)?;

    Ok(docs_indexed)
}

/// Public wrapper for use by task handlers.
pub fn parse_frontmatter_pub(content: &str) -> DocFrontmatter { parse_frontmatter(content) }
pub fn classify_doc_pub(rel_path: &str, fm: &DocFrontmatter) -> DocClassification { classify_doc(rel_path, fm) }
pub fn create_traceability_edges_pub(graph: &GraphDb, repo_id: &str) -> Result<(), String> { create_traceability_edges(graph, repo_id) }

/// Parse YAML frontmatter from between --- fences.
fn parse_frontmatter(content: &str) -> DocFrontmatter {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return DocFrontmatter::default();
    }
    // Find the closing ---
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let yaml = &after_first[..end];
        serde_yaml::from_str(yaml).unwrap_or_default()
    } else {
        DocFrontmatter::default()
    }
}

/// Classify a doc by frontmatter type field, then path heuristics.
fn classify_doc(rel_path: &str, frontmatter: &DocFrontmatter) -> DocClassification {
    // 1. Frontmatter type takes priority
    if let Some(ref ft) = frontmatter.doc_type {
        let ft_lower = ft.to_lowercase();
        let kind = if ft_lower == "extension" || ft_lower == "skill" || ft_lower == "command" || ft_lower == "plugin" {
            NodeKind::Extension
        } else {
            NodeKind::Doc
        };
        return DocClassification {
            kind,
            doc_type: ft_lower,
            doc_category: frontmatter.category.clone(),
        };
    }

    // 2. Path-based heuristics
    let lower = rel_path.to_lowercase();

    // Marketplace files are extensions, not docs
    if lower.starts_with("marketplace/skills/") || lower.starts_with("marketplace/commands/")
        || lower.starts_with("marketplace/plugins/") || lower.starts_with("marketplace/hooks/")
    {
        let sub = if lower.contains("/skills/") { "skill" }
            else if lower.contains("/commands/") { "command" }
            else if lower.contains("/plugins/") { "plugin" }
            else { "hook" };
        return DocClassification {
            kind: NodeKind::Extension,
            doc_type: "extension".into(),
            doc_category: Some(sub.into()),
        };
    }

    // Requirement docs
    if lower.contains("requirement") || lower.contains("/prd") || lower.contains("openspec/product/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "requirement".into(), doc_category: frontmatter.category.clone() };
    }

    // Design docs
    if lower.contains("/design/") || lower.contains("architecture") || lower.contains("/adr/")
        || lower.contains("/plans/") || lower.contains("openspec/specs/") && lower.contains("design")
    {
        return DocClassification { kind: NodeKind::Doc, doc_type: "design".into(), doc_category: frontmatter.category.clone() };
    }

    // Feature specs
    if lower.contains("/features/") || lower.contains("/specs/") || lower.contains("openspec/specs/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "feature".into(), doc_category: frontmatter.category.clone() };
    }

    // Usage / overview
    if lower.contains("readme") || lower.contains("llms") || lower.contains("/guide/") || lower.contains("/usage/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "usage".into(), doc_category: frontmatter.category.clone() };
    }

    // API specs
    if lower.contains("/api/") || lower.contains("openapi") || lower.contains("swagger") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "api-spec".into(), doc_category: frontmatter.category.clone() };
    }

    // Changelog
    if lower.contains("changelog") || lower.contains("release") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "changelog".into(), doc_category: None };
    }

    // Roadmap
    if lower.contains("/roadmap/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "design".into(), doc_category: Some("roadmap".into()) };
    }

    // Operations
    if lower.contains("runbook") || lower.contains("/ops/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "operations".into(), doc_category: None };
    }

    // Default
    DocClassification { kind: NodeKind::Doc, doc_type: "doc".into(), doc_category: None }
}

/// Create traceability edges between doc stages.
/// SPECIFIES: requirement → design (matched by shared path segments)
/// IMPLEMENTS: design → code module (matched by module names in doc content)
/// DOCUMENTS: usage → code module (matched by directory co-location)
fn create_traceability_edges(graph_db: &GraphDb, repo_id: &str) -> Result<(), String> {
    let nodes = graph_db.get_nodes(repo_id).unwrap_or_default();

    // Collect docs by type
    let mut requirements: Vec<(&str, &str)> = Vec::new(); // (id, file)
    let mut designs: Vec<(&str, &str)> = Vec::new();
    let mut features: Vec<(&str, &str)> = Vec::new();
    let mut usage_docs: Vec<(&str, &str)> = Vec::new();
    let mut modules: Vec<(&str, &str)> = Vec::new(); // (id, name)

    for node in &nodes {
        match node.kind.as_str() {
            "doc" => {
                // We need to check doc_type but GraphNode doesn't have it.
                // Use the ID prefix and file path to infer.
                let file = &node.file;
                let lower = file.to_lowercase();
                if lower.contains("requirement") || lower.contains("openspec/product") {
                    requirements.push((&node.id, &node.file));
                } else if lower.contains("/design/") || lower.contains("/plans/") || lower.contains("/adr/") {
                    designs.push((&node.id, &node.file));
                } else if lower.contains("/features/") || lower.contains("/specs/") {
                    features.push((&node.id, &node.file));
                } else if lower.contains("readme") || lower.contains("llms") || lower.contains("/guide/") {
                    usage_docs.push((&node.id, &node.file));
                }
            }
            "module" => {
                modules.push((&node.id, &node.name));
            }
            _ => {}
        }
    }

    // SPECIFIES: requirement → design (when design path contains similar component name)
    for (req_id, req_file) in &requirements {
        let req_name = Path::new(req_file).file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        for (design_id, design_file) in &designs {
            let design_name = Path::new(design_file).file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            // Match if names share significant words
            if !req_name.is_empty() && !design_name.is_empty() {
                let req_words: HashSet<&str> = req_name.split(|c: char| c == '-' || c == '_').filter(|w| w.len() > 2).collect();
                let design_words: HashSet<&str> = design_name.split(|c: char| c == '-' || c == '_').filter(|w| w.len() > 2).collect();
                let overlap = req_words.intersection(&design_words).count();
                if overlap >= 1 {
                    graph_db.merge_edge(req_id, design_id, "SPECIFIES").ok();
                }
            }
        }
    }

    // IMPLEMENTS: design → code module (when module name appears in design doc path)
    for (design_id, design_file) in &designs {
        let design_lower = design_file.to_lowercase();
        for (mod_id, mod_name) in &modules {
            let mod_lower = mod_name.to_lowercase();
            // Check if design doc references the module's directory name
            let mod_leaf = mod_lower.rsplit('/').next().unwrap_or(&mod_lower);
            if mod_leaf.len() > 2 && design_lower.contains(mod_leaf) {
                graph_db.merge_edge(design_id, mod_id, "IMPLEMENTS").ok();
            }
        }
    }

    // DOCUMENTS: usage doc → code module (co-located in same directory tree)
    for (usage_id, usage_file) in &usage_docs {
        let usage_dir = Path::new(usage_file).parent().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
        for (mod_id, mod_name) in &modules {
            let mod_lower = mod_name.to_lowercase();
            if !usage_dir.is_empty() && !mod_lower.is_empty() && usage_dir.contains(&mod_lower) {
                graph_db.merge_edge(usage_id, mod_id, "DOCUMENTS").ok();
            }
        }
    }

    Ok(())
}

/// Extract the first H1 title from markdown.
pub fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed[2..].trim().to_string());
        }
    }
    None
}

/// Extract file path references from markdown content.
pub fn extract_file_refs(content: &str, repo_path: &str) -> Vec<String> {
    let mut refs = HashSet::new();
    let repo = Path::new(repo_path);

    for cap in content.split('`') {
        let trimmed = cap.trim();
        if trimmed.contains('/') && !trimmed.contains(' ') && trimmed.len() < 200 {
            let abs = repo.join(trimmed);
            if abs.exists() {
                refs.insert(abs.to_string_lossy().to_string());
            }
        }
    }

    refs.into_iter().collect()
}

/// Extract function mentions: `functionName` in backticks.
pub fn extract_fn_mentions(content: &str) -> Vec<String> {
    let mut names = HashSet::new();
    let parts: Vec<&str> = content.split('`').collect();
    for (i, part) in parts.iter().enumerate() {
        if i % 2 == 1 {
            let trimmed = part.trim();
            if !trimmed.contains('/') && !trimmed.contains(' ') && !trimmed.contains('.')
                && trimmed.len() > 1 && trimmed.len() < 50
                && trimmed.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
                names.insert(trimmed.to_string());
            }
        }
    }
    names.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_title_from_h1() {
        assert_eq!(extract_title("# Hello World\nsome text"), Some("Hello World".to_string()));
        assert_eq!(extract_title("no heading here"), None);
        assert_eq!(extract_title("## Not H1"), None);
    }

    #[test]
    fn classify_docs() {
        let fm = DocFrontmatter::default();
        assert_eq!(classify_doc("docs/requirements/auth.md", &fm).doc_type, "requirement");
        assert_eq!(classify_doc("docs/design/api.md", &fm).doc_type, "design");
        assert_eq!(classify_doc("README.md", &fm).doc_type, "usage");
        assert_eq!(classify_doc("CHANGELOG.md", &fm).doc_type, "changelog");
        assert_eq!(classify_doc("docs/guide/setup.md", &fm).doc_type, "usage");
        assert_eq!(classify_doc("docs/misc/notes.md", &fm).doc_type, "doc");
    }

    #[test]
    fn classify_marketplace_as_extension() {
        let fm = DocFrontmatter::default();
        let c = classify_doc("marketplace/skills/test-gen/SKILL.md", &fm);
        assert!(matches!(c.kind, NodeKind::Extension));
        assert_eq!(c.doc_type, "extension");
        assert_eq!(c.doc_category, Some("skill".into()));

        let c2 = classify_doc("marketplace/commands/commit.md", &fm);
        assert!(matches!(c2.kind, NodeKind::Extension));
        assert_eq!(c2.doc_category, Some("command".into()));
    }

    #[test]
    fn classify_features_and_plans() {
        let fm = DocFrontmatter::default();
        assert_eq!(classify_doc("docs/features/01-codebase.md", &fm).doc_type, "feature");
        assert_eq!(classify_doc("docs/superpowers/plans/2026-plan.md", &fm).doc_type, "design");
        assert_eq!(classify_doc("docs/roadmap/gaps.md", &fm).doc_type, "design");
    }

    #[test]
    fn frontmatter_overrides_path() {
        let fm = DocFrontmatter { doc_type: Some("requirement".into()), ..Default::default() };
        let c = classify_doc("random/path/file.md", &fm);
        assert_eq!(c.doc_type, "requirement");
    }

    #[test]
    fn frontmatter_skill_type() {
        let fm = DocFrontmatter { doc_type: Some("skill".into()), name: Some("my-skill".into()), ..Default::default() };
        let c = classify_doc("marketplace/skills/x/SKILL.md", &fm);
        assert!(matches!(c.kind, NodeKind::Extension));
        assert_eq!(c.doc_type, "skill");
    }

    #[test]
    fn parse_frontmatter_basic() {
        let content = "---\ntype: design\ncategory: api\ntitle: API Design\n---\n# Content";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.doc_type, Some("design".into()));
        assert_eq!(fm.category, Some("api".into()));
        assert_eq!(fm.title, Some("API Design".into()));
    }

    #[test]
    fn parse_frontmatter_missing() {
        let fm = parse_frontmatter("# No frontmatter\nJust content.");
        assert!(fm.doc_type.is_none());
    }

    #[test]
    fn parse_frontmatter_skill() {
        let content = "---\nname: test-gen\ndescription: Generate tests\n---\n# Skill";
        let fm = parse_frontmatter(content);
        assert_eq!(fm.name, Some("test-gen".into()));
        assert_eq!(fm.description, Some("Generate tests".into()));
    }

    #[test]
    fn extract_fn_mentions_from_backticks() {
        let content = "Use `greet` to say hello. The `validate` function checks input. Ignore `src/foo.ts`.";
        let names = extract_fn_mentions(content);
        assert!(names.contains(&"greet".to_string()));
        assert!(names.contains(&"validate".to_string()));
        assert!(!names.iter().any(|n| n.contains("src")));
    }

    #[test]
    fn extract_file_refs_from_content() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.py"), "pass").unwrap();

        let content = "See `src/main.py` for the entry point.";
        let refs = extract_file_refs(content, &dir.path().to_string_lossy());
        assert_eq!(refs.len(), 1);
        assert!(refs[0].contains("src/main.py"));
    }

    #[test]
    fn index_docs_on_test_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("README.md"), "# My Project\n\nA test project.\n").unwrap();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("docs/design.md"), "# Design\n\nArchitecture notes.\n").unwrap();

        let graph = GraphDb::open_memory().unwrap();
        let count = index_docs(&graph, &dir.path().to_string_lossy(), "test").unwrap();
        assert_eq!(count, 2);

        // Verify node kinds
        let nodes = graph.get_nodes("test").unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().all(|n| n.kind == "doc"));
    }

    #[test]
    fn marketplace_indexed_as_extension() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("marketplace/skills/test")).unwrap();
        std::fs::write(dir.path().join("marketplace/skills/test/SKILL.md"),
            "---\nname: test-skill\ndescription: Test\n---\n# Skill content").unwrap();
        std::fs::create_dir_all(dir.path().join("marketplace/commands")).unwrap();
        std::fs::write(dir.path().join("marketplace/commands/commit.md"),
            "---\ndescription: Commit changes\n---\n# Commit").unwrap();

        let graph = GraphDb::open_memory().unwrap();
        let count = index_docs(&graph, &dir.path().to_string_lossy(), "test").unwrap();
        assert_eq!(count, 2);

        let nodes = graph.get_nodes("test").unwrap();
        assert!(nodes.iter().all(|n| n.kind == "extension"), "marketplace files should be extensions");
    }
}
