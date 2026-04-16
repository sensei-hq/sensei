//! Document file processor — .md, .mdx files with frontmatter + classification.

use super::types::*;
use crate::indexer::doc_indexer;

/// Process a markdown/mdx document file.
pub fn process(abs_path: &str, rel_path: &str, content: &str, repo_id: &str, repo_path: &str) -> FileProcessResult {
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
