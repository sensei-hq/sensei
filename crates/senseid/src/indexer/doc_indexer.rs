use std::path::Path;
use std::collections::HashSet;
use walkdir::WalkDir;
use crate::indexer::graph::GraphDb;

const DOC_EXTENSIONS: &[&str] = &["md", "mdx"];

/// Index documentation files (.md, .mdx) into the graph.
/// Creates Doc nodes and COVERS edges to File nodes.
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

        let title = extract_title(&content).unwrap_or_else(|| rel_path.clone());
        let doc_type = classify_doc(&rel_path);
        let doc_id = format!("doc:{}", abs_path);

        // Create Doc node
        graph_db.merge_doc(&doc_id, &abs_path, &title, &doc_type, repo_id)?;

        // Extract file references from the doc content
        let file_refs = extract_file_refs(&content, repo_path);
        for file_ref in &file_refs {
            let file_id = format!("file:{}", file_ref);
            graph_db.merge_edge(&doc_id, &file_id, "COVERS").ok();
        }

        // Extract function mentions (backtick identifiers)
        let fn_mentions = extract_fn_mentions(&content);
        for fn_name in &fn_mentions {
            // Try to find the function by name in the graph
            if let Ok(Some(fn_id)) = graph_db.find_function_by_name(fn_name, repo_id) {
                graph_db.merge_edge(&doc_id, &fn_id, "MENTIONS_FN").ok();
            }
        }

        docs_indexed += 1;
    }

    Ok(docs_indexed)
}

/// Extract the first H1 title from markdown.
fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed[2..].trim().to_string());
        }
    }
    None
}

/// Classify a doc by its path.
fn classify_doc(rel_path: &str) -> String {
    let lower = rel_path.to_lowercase();
    if lower.contains("requirement") || lower.contains("spec/") || lower.contains("prd") {
        "requirement".to_string()
    } else if lower.contains("design") || lower.contains("architecture") || lower.contains("adr") {
        "design".to_string()
    } else if lower.contains("api") || lower.contains("openapi") || lower.contains("swagger") {
        "api-spec".to_string()
    } else if lower.contains("changelog") || lower.contains("release") {
        "changelog".to_string()
    } else if lower.contains("readme") {
        "overview".to_string()
    } else if lower.contains("runbook") || lower.contains("ops") {
        "operations".to_string()
    } else {
        "doc".to_string()
    }
}

/// Extract file path references from markdown content.
fn extract_file_refs(content: &str, repo_path: &str) -> Vec<String> {
    let mut refs = HashSet::new();
    let repo = Path::new(repo_path);

    // Backtick paths: `src/foo.ts`
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
fn extract_fn_mentions(content: &str) -> Vec<String> {
    let mut names = HashSet::new();
    let parts: Vec<&str> = content.split('`').collect();
    for (i, part) in parts.iter().enumerate() {
        if i % 2 == 1 { // inside backticks
            let trimmed = part.trim();
            // Accept plain identifiers (no spaces, paths, or operators)
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
        assert_eq!(classify_doc("docs/requirements/auth.md"), "requirement");
        assert_eq!(classify_doc("docs/design/api.md"), "design");
        assert_eq!(classify_doc("docs/architecture/overview.md"), "design");
        assert_eq!(classify_doc("README.md"), "overview");
        assert_eq!(classify_doc("CHANGELOG.md"), "changelog");
        assert_eq!(classify_doc("docs/api/reference.md"), "api-spec");
        assert_eq!(classify_doc("docs/guide/setup.md"), "doc");
    }

    #[test]
    fn extract_fn_mentions_from_backticks() {
        let content = "Use `greet` to say hello. The `validate` function checks input. Ignore `src/foo.ts`.";
        let names = extract_fn_mentions(content);
        assert!(names.contains(&"greet".to_string()));
        assert!(names.contains(&"validate".to_string()));
        assert!(!names.iter().any(|n| n.contains("src"))); // paths excluded
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
    }
}
