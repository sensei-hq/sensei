use std::path::Path;
use std::collections::HashSet;

use crate::types::NodeKind;

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
    #[serde(default, rename = "description")]
    pub _description: Option<String>,
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
/// Public wrapper for use by task handlers.
pub fn parse_frontmatter_pub(content: &str) -> DocFrontmatter { parse_frontmatter(content) }
pub fn classify_doc_pub(rel_path: &str, fm: &DocFrontmatter) -> DocClassification { classify_doc(rel_path, fm) }
pub fn create_traceability_edges_pub(_repo_id: &str) -> Result<(), String> { Ok(()) }

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
    // Matches both "marketplace/skills/..." (parent repo) and "skills/..." (subtree repo)
    if lower.starts_with("marketplace/skills/") || lower.starts_with("marketplace/commands/")
        || lower.starts_with("marketplace/plugins/") || lower.starts_with("marketplace/hooks/")
        || lower.starts_with("skills/") || lower.starts_with("commands/")
        || lower.starts_with("plugins/") || lower.starts_with("hooks/")
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

    // Sensei workflow phases
    if lower.contains("/ideas/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "idea".into(), doc_category: Some("idea".into()) };
    }
    if lower.contains("/analysis/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "analysis".into(), doc_category: Some("analysis".into()) };
    }
    if lower.contains("/blueprints/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "blueprint".into(), doc_category: Some("blueprint".into()) };
    }
    if lower.contains("/experiments/") {
        return DocClassification { kind: NodeKind::Doc, doc_type: "experiment".into(), doc_category: Some("experiment".into()) };
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

/// Parse a markdown file into an IRDoc.
/// This is the doc adapter's parse() implementation.
#[cfg(test)]
pub fn parse_to_ir(content: &str, rel_path: &str, repo_path: &str) -> crate::ir::IRDoc {
    let fm = parse_frontmatter(content);
    let classification = classify_doc(rel_path, &fm);
    let title = fm.name.clone()
        .or_else(|| fm.title.clone())
        .or_else(|| extract_title(content));
    let sections = extract_sections(content);
    let code_blocks = extract_code_blocks(content);
    let file_refs = extract_file_refs(content, repo_path);
    let fn_mentions = extract_fn_mentions(content);
    let doc_refs = extract_doc_refs(content);

    // Build raw frontmatter HashMap
    let mut frontmatter = std::collections::HashMap::new();
    if let Some(ref v) = fm.name { frontmatter.insert("name".into(), v.clone()); }
    if let Some(ref v) = fm.title { frontmatter.insert("title".into(), v.clone()); }
    if let Some(ref v) = fm.doc_type { frontmatter.insert("type".into(), v.clone()); }
    if let Some(ref v) = fm.category { frontmatter.insert("category".into(), v.clone()); }
    if let Some(ref v) = fm._description { frontmatter.insert("description".into(), v.clone()); }
    // Also extract additional frontmatter fields we haven't typed
    extract_raw_frontmatter(content, &mut frontmatter);

    // Infer category from path if not in frontmatter
    let category = classification.doc_category.clone()
        .or_else(|| infer_category_from_path(rel_path));

    // Infer extension
    let ext = std::path::Path::new(rel_path).extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e));

    crate::ir::IRDoc {
        base: crate::ir::IRBase {
            name: title.clone().unwrap_or_else(|| {
                std::path::Path::new(rel_path).file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            }),
            file: rel_path.into(),
            line_start: 0,
            line_end: content.lines().count() as u32,
            extension: ext,
            language: Some("markdown".into()),
            node_type: Some("doc".into()),
            category,
            tags: vec!["doc".into()],
            ..Default::default()
        },
        doc_type: Some(classification.doc_type),
        frontmatter,
        status: fm.doc_type.as_deref()
            .and_then(|_| None) // status not in DocFrontmatter yet — use raw
            .or_else(|| extract_frontmatter_field(content, "status")),
        origin: extract_frontmatter_field(content, "origin"),
        description: fm._description.clone(),
        date: extract_frontmatter_field(content, "date"),
        title,
        sections,
        code_blocks,
        file_references: file_refs,
        symbol_references: fn_mentions,
        doc_references: doc_refs,
    }
}

/// Extract sections split by headings.
#[cfg(test)]
fn extract_sections(content: &str) -> Vec<crate::ir::IRSection> {
    let lines: Vec<&str> = content.lines().collect();
    let mut sections = Vec::new();
    let mut current_heading: Option<(String, u8, u32)> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some((level, text)) = parse_heading(trimmed) {
            // Close previous section
            if let Some((heading, lvl, start)) = current_heading.take() {
                let end = i as u32;
                let preview = make_preview(&lines, start as usize + 1, end as usize);
                sections.push(crate::ir::IRSection {
                    heading, level: lvl, line_start: start + 1, line_end: end,
                    content_preview: preview,
                });
            }
            current_heading = Some((text, level, i as u32));
        }
    }

    // Close last section
    if let Some((heading, level, start)) = current_heading {
        let end = lines.len() as u32;
        let preview = make_preview(&lines, start as usize + 1, end as usize);
        sections.push(crate::ir::IRSection {
            heading, level, line_start: start + 1, line_end: end,
            content_preview: preview,
        });
    }

    sections
}

#[cfg(test)]
fn parse_heading(line: &str) -> Option<(u8, String)> {
    if line.starts_with("######") { Some((6, line[6..].trim().into())) }
    else if line.starts_with("#####") { Some((5, line[5..].trim().into())) }
    else if line.starts_with("####") { Some((4, line[4..].trim().into())) }
    else if line.starts_with("###") { Some((3, line[3..].trim().into())) }
    else if line.starts_with("##") { Some((2, line[2..].trim().into())) }
    else if line.starts_with("# ") { Some((1, line[2..].trim().into())) }
    else { None }
}

#[cfg(test)]
fn make_preview(lines: &[&str], start: usize, end: usize) -> Option<String> {
    let text: String = lines.get(start..end.min(lines.len()))
        .map(|s| s.join("\n"))
        .unwrap_or_default()
        .chars()
        .take(200)
        .collect();
    if text.trim().is_empty() { None } else { Some(text.trim().into()) }
}

/// Extract code blocks with language tags.
#[cfg(test)]
fn extract_code_blocks(content: &str) -> Vec<crate::ir::IRCodeBlock> {
    let lines: Vec<&str> = content.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("```") {
            let lang = trimmed[3..].trim();
            let lang = if lang.is_empty() { None } else { Some(lang.to_string()) };
            let start = i as u32 + 1;
            i += 1;
            let mut block_lines = Vec::new();
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                block_lines.push(lines[i]);
                i += 1;
            }
            let end = i as u32;
            let content_str = block_lines.join("\n");
            if !content_str.trim().is_empty() {
                blocks.push(crate::ir::IRCodeBlock {
                    language: lang,
                    content: content_str,
                    line_start: start + 1,
                    line_end: end,
                });
            }
        }
        i += 1;
    }

    blocks
}

/// Extract doc-to-doc references from markdown links.
#[cfg(test)]
fn extract_doc_refs(content: &str) -> Vec<String> {
    let mut refs = HashSet::new();
    // Match [text](path.md) style links
    for line in content.lines() {
        let mut remaining = line;
        while let Some(start) = remaining.find("](") {
            let after = &remaining[start + 2..];
            if let Some(end) = after.find(')') {
                let path = &after[..end];
                if (path.ends_with(".md") || path.ends_with(".txt"))
                    && !path.starts_with("http")
                    && path.len() < 200
                {
                    // Normalize: strip leading ./ and ../
                    let normalized = path.trim_start_matches("./");
                    refs.insert(normalized.to_string());
                }
                remaining = &after[end..];
            } else {
                break;
            }
        }
    }
    refs.into_iter().collect()
}

/// Extract a specific frontmatter field by name from raw content.
#[cfg(test)]
fn extract_frontmatter_field(content: &str, field: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") { return None; }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    let yaml = &after_first[..end];
    for line in yaml.lines() {
        let line = line.trim();
        if line.starts_with(&format!("{}:", field)) {
            let val = line[field.len() + 1..].trim();
            if val.is_empty() { return None; }
            return Some(val.to_string());
        }
    }
    None
}

/// Extract all raw frontmatter key-value pairs.
#[cfg(test)]
fn extract_raw_frontmatter(content: &str, map: &mut std::collections::HashMap<String, String>) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") { return; }
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let yaml = &after_first[..end];
        for line in yaml.lines() {
            let line = line.trim();
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let val = line[colon_pos + 1..].trim().to_string();
                if !key.is_empty() && !val.is_empty() && !key.contains(' ') {
                    map.entry(key).or_insert(val);
                }
            }
        }
    }
}

/// Infer category from path.
#[cfg(test)]
fn infer_category_from_path(rel_path: &str) -> Option<String> {
    let lower = rel_path.to_lowercase();
    if lower.contains("/ideas/") { Some("idea".into()) }
    else if lower.contains("/analysis/") { Some("analysis".into()) }
    else if lower.contains("/blueprints/") { Some("blueprint".into()) }
    else if lower.contains("/experiments/") { Some("experiment".into()) }
    else if lower.contains("/plans/") { Some("plan".into()) }
    else if lower.contains("/design/") { Some("design".into()) }
    else if lower.contains("/features/") { Some("feature".into()) }
    else if lower.contains("/reference/") { Some("reference".into()) }
    else { None }
}

/// Extract the first H1 title from markdown.
pub fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.trim().to_string());
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
        assert_eq!(fm._description, Some("Generate tests".into()));
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

    // index_docs and marketplace tests moved to tasks/processors/doc.rs and tests.rs

    // ── IR Doc Parser Tests ──────────────────────────────────────────

    #[test]
    fn parse_to_ir_with_full_frontmatter() {
        let content = "---\nname: Workflow System\ndescription: A phased workflow\ndate: 2026-04-17\nstatus: complete\norigin: conversation\n---\n\n# Workflow System\n\n## Problem\n\nAI starts cold.\n\n## Solution\n\nUse phases.\n";
        let doc = parse_to_ir(content, "docs/ideas/01-workflow.md", "/tmp/repo");

        assert_eq!(doc.base.name, "Workflow System");
        assert_eq!(doc.doc_type, Some("idea".into()));
        assert_eq!(doc.base.category, Some("idea".into()));
        assert_eq!(doc.status, Some("complete".into()));
        assert_eq!(doc.origin, Some("conversation".into()));
        assert_eq!(doc.date, Some("2026-04-17".into()));
        assert_eq!(doc.description, Some("A phased workflow".into()));
        assert_eq!(doc.title, Some("Workflow System".into()));
        assert_eq!(doc.frontmatter["name"], "Workflow System");
    }

    #[test]
    fn parse_to_ir_without_frontmatter() {
        let content = "# Plain Readme\n\nSome content here.\n\n## Section One\n\nDetails.\n";
        let doc = parse_to_ir(content, "README.md", "/tmp/repo");

        assert_eq!(doc.title, Some("Plain Readme".into()));
        assert!(doc.frontmatter.is_empty());
        assert!(doc.status.is_none());
        assert!(doc.origin.is_none());
        assert_eq!(doc.doc_type, Some("usage".into())); // README → usage
    }

    #[test]
    fn parse_to_ir_extracts_sections() {
        let content = "# Title\n\nIntro.\n\n## Problem\n\nAI is forgetful.\n\n## Solution\n\nUse documents.\n\n### Sub-section\n\nDetails.\n";
        let doc = parse_to_ir(content, "doc.md", "/tmp/repo");

        assert!(doc.sections.len() >= 3); // Title, Problem, Solution (maybe Sub-section)
        let problem = doc.sections.iter().find(|s| s.heading == "Problem").unwrap();
        assert_eq!(problem.level, 2);
        assert!(problem.content_preview.as_ref().unwrap().contains("forgetful"));
    }

    #[test]
    fn parse_to_ir_extracts_code_blocks() {
        let content = "# Doc\n\n```rust\nfn main() {}\n```\n\n```yaml\nkey: value\n```\n";
        let doc = parse_to_ir(content, "doc.md", "/tmp/repo");

        assert_eq!(doc.code_blocks.len(), 2);
        assert_eq!(doc.code_blocks[0].language, Some("rust".into()));
        assert!(doc.code_blocks[0].content.contains("fn main"));
        assert_eq!(doc.code_blocks[1].language, Some("yaml".into()));
    }

    #[test]
    fn parse_to_ir_extracts_symbol_references() {
        let content = "Use `parse_file` and `get_callers` for analysis. See `src/main.rs`.";
        let doc = parse_to_ir(content, "doc.md", "/tmp/repo");

        assert!(doc.symbol_references.contains(&"parse_file".to_string()));
        assert!(doc.symbol_references.contains(&"get_callers".to_string()));
        // src/main.rs should NOT be in symbol references (has / and .)
        assert!(!doc.symbol_references.iter().any(|s| s.contains("src")));
    }

    #[test]
    fn parse_to_ir_extracts_doc_refs() {
        let content = "See [ideas](./ideas/01-workflow.md) and [blueprint](../blueprints/01.md).";
        let doc = parse_to_ir(content, "doc.md", "/tmp/repo");

        assert!(doc.doc_references.iter().any(|r| r.contains("01-workflow.md")));
        assert!(doc.doc_references.iter().any(|r| r.contains("01.md")));
    }

    #[test]
    fn parse_to_ir_infers_doc_type_from_path() {
        let content = "# Some Design\n\nContent.";
        let doc = parse_to_ir(content, "docs/design/01-daemon/architecture.md", "/tmp/repo");
        assert_eq!(doc.doc_type, Some("design".into()));
        assert_eq!(doc.base.category, Some("design".into()));
    }

    #[test]
    fn parse_to_ir_base_fields() {
        let content = "---\nname: Test\n---\n# Test\nline 2\nline 3\n";
        let doc = parse_to_ir(content, "docs/test.md", "/tmp/repo");

        assert_eq!(doc.base.file, "docs/test.md");
        assert_eq!(doc.base.extension, Some(".md".into()));
        assert_eq!(doc.base.language, Some("markdown".into()));
        assert_eq!(doc.base.node_type, Some("doc".into()));
        assert!(doc.base.tags.contains(&"doc".to_string()));
    }
}
