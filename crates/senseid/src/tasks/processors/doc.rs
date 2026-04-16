//! Document file processor — .md, .mdx files with frontmatter + classification.

use super::types::*;
use crate::indexer::doc_indexer;

/// Process a markdown/mdx document file.
pub fn process(abs_path: &str, rel_path: &str, content: &str, _repo_id: &str, repo_path: &str) -> FileProcessResult {
    let frontmatter = doc_indexer::parse_frontmatter_pub(content);
    let classification = doc_indexer::classify_doc_pub(rel_path, &frontmatter);

    let title = frontmatter.title
        .or(frontmatter.name)
        .or_else(|| doc_indexer::extract_title(content));

    let file_refs = doc_indexer::extract_file_refs(content, repo_path);
    let fn_mentions = doc_indexer::extract_fn_mentions(content);

    FileProcessResult {
        file_id: format!("file:{}", abs_path),
        rel_path: rel_path.to_string(),
        abs_path: abs_path.to_string(),
        kind: classification.kind.as_str().to_string(),
        tags: "doc".into(),
        language: None,
        doc_type: Some(classification.doc_type),
        doc_category: classification.doc_category,
        title,
        symbols: vec![],
        unresolved_imports: vec![],
        unresolved_calls: vec![],
        parent_refs: vec![],
        file_refs,
        fn_mentions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent().unwrap().parent().unwrap().to_path_buf()
    }

    fn process_doc_file(rel: &str) -> FileProcessResult {
        let root = repo_root();
        let abs = root.join(rel);
        let content = std::fs::read_to_string(&abs).expect(&format!("not found: {}", abs.display()));
        process(&abs.to_string_lossy(), rel, &content, "sensei", &root.to_string_lossy())
    }

    fn process_subtree_doc(rel: &str, subtree: &str) -> FileProcessResult {
        let root = repo_root();
        let subtree_root = root.join(subtree);
        let abs = root.join(rel);
        let sub_rel = abs.strip_prefix(&subtree_root).unwrap_or(&abs).to_string_lossy().to_string();
        let content = std::fs::read_to_string(&abs).expect(&format!("not found: {}", abs.display()));
        process(&abs.to_string_lossy(), &sub_rel, &content, &format!("sensei:{}", subtree), &subtree_root.to_string_lossy())
    }

    #[test]
    fn design_doc_architecture() {
        let r = process_doc_file("docs/design/01-architecture.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.tags, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("design"));
        assert!(r.title.is_some(), "should extract title from # heading");
        assert!(r.title.as_deref().unwrap().len() > 0);
    }

    #[test]
    fn feature_doc_codebase_intelligence() {
        let r = process_doc_file("docs/features/01-codebase-intelligence.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("feature"));
    }

    #[test]
    fn roadmap_doc_paradigm_shift() {
        let r = process_doc_file("docs/roadmap/01-paradigm-shift.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("design")); // roadmap → design
    }

    #[test]
    fn gap_analysis() {
        let r = process_doc_file("docs/gap-analysis.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.tags, "doc");
    }

    #[test]
    fn root_readme() {
        let r = process_doc_file("README.md");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("usage"));
        assert!(r.title.is_some());
    }

    #[test]
    fn homebrew_readme() {
        let r = process_subtree_doc("homebrew/README.md", "homebrew");
        assert_eq!(r.kind, "doc");
        assert_eq!(r.doc_type.as_deref(), Some("usage"));
    }

    #[test]
    fn marketplace_skill_is_extension() {
        let r = process_subtree_doc(
            "marketplace/skills/auditing-skill-descriptions/SKILL.md",
            "marketplace",
        );
        assert_eq!(r.kind, "extension", "marketplace skill should be extension, not doc");
        assert_eq!(r.doc_type.as_deref(), Some("extension"));
    }

    #[test]
    fn llms_txt_is_usage_doc() {
        let root = repo_root();
        let abs = root.join("docs/llms/index.txt");
        if abs.exists() {
            let content = std::fs::read_to_string(&abs).unwrap();
            let r = process(&abs.to_string_lossy(), "docs/llms/index.txt", &content, "sensei", &root.to_string_lossy());
            assert_eq!(r.kind, "doc");
            assert_eq!(r.doc_type.as_deref(), Some("usage"), "llms.txt should be usage doc");
        }
    }

    #[test]
    fn design_doc_has_frontmatter_type() {
        // docs/design files with frontmatter type: design should use frontmatter
        let r = process_doc_file("docs/design/41-task-queue-architecture.md");
        assert_eq!(r.doc_type.as_deref(), Some("design"));
    }
}

