use std::path::Path;
use walkdir::WalkDir;
use crate::db::Store;
use crate::indexer::graph::GraphDb;

/// Discover and index llms.txt files from a repo.
/// Also generates llms-style docs from the call graph for repos without llms.txt.
pub fn index_llms(
    store: &Store,
    graph_db: &GraphDb,
    repo_path: &str,
    repo_id: &str,
) -> Result<u32, String> {
    let repo = Path::new(repo_path);
    let mut indexed = 0u32;
    let now = chrono::Utc::now().to_rfc3339();

    // 1. Discover llms.txt files in standard locations
    let llms_files = discover_llms_files(repo);

    if !llms_files.is_empty() {
        // Index each discovered llms.txt
        for (component_name, file_path) in &llms_files {
            let content = match std::fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let title = content.lines().next()
                .map(|l| l.trim_start_matches('#').trim().to_string())
                .unwrap_or_else(|| component_name.clone());

            let summary = content.lines().skip(1).take(3)
                .collect::<Vec<_>>().join(" ").trim().chars().take(200).collect::<String>();

            let id = format!("lib:{}:{}", repo_id, component_name);

            store.upsert_lib_doc(
                &id, repo_id, &title, None, &summary,
                Some(&content[..content.len().min(10000)]),
                "llms-txt", Some(component_name.as_str()), &now,
            ).ok();
            indexed += 1;
        }

        // Update lib_meta
        store.upsert_lib_meta(repo_id, "local", Some(repo_path), None, &now).ok();
    } else {
        // 2. No llms.txt found — generate docs from call graph
        indexed += generate_llms_from_graph(store, graph_db, repo_path, repo_id, &now)?;
    }

    Ok(indexed)
}

/// Find llms.txt files in standard locations within a repo.
/// Returns (component_name, file_path) pairs.
fn discover_llms_files(repo: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();

    // Check standard locations:
    // 1. root llms.txt
    // 2. llms/ directory
    // 3. docs/llms/ directory
    // 4. site/static/llms/ directory (common for doc sites)
    // 5. **/**/llms.txt (recursive)

    let search_dirs = [
        repo.join("llms"),
        repo.join("docs/llms"),
        repo.join("site/static/llms"),
        repo.join("static/llms"),
    ];

    // Root llms.txt
    let root_llms = repo.join("llms.txt");
    if root_llms.exists() {
        found.push(("index".into(), root_llms.to_string_lossy().to_string()));
    }

    // Search known directories
    for dir in &search_dirs {
        if !dir.exists() { continue; }
        for entry in WalkDir::new(dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !entry.file_type().is_file() || (ext != "txt" && ext != "md") { continue; }

            // Derive component name from path relative to the llms dir
            let rel = path.strip_prefix(dir).unwrap_or(path);
            let name = rel.with_extension("")
                .to_string_lossy()
                .replace('\\', "/")
                .replace('/', "-")
                .to_string();

            if name.is_empty() { continue; }
            found.push((name, path.to_string_lossy().to_string()));
        }
    }

    // Also find llms.txt in component/route directories
    for entry in WalkDir::new(repo).max_depth(6).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() != "llms.txt" { continue; }
        let path = entry.path();
        // Skip if already found via known dirs
        let path_str = path.to_string_lossy().to_string();
        if found.iter().any(|(_, p)| p == &path_str) { continue; }
        // Skip node_modules, .git, etc
        if path_str.contains("node_modules") || path_str.contains(".git/") { continue; }

        let parent = path.parent().and_then(|p| p.file_name())
            .and_then(|n| n.to_str()).unwrap_or("unknown");
        found.push((parent.to_string(), path_str));
    }

    found
}

/// Generate llms-style documentation from the indexed call graph.
/// Creates an overview doc with exported functions, types, and their signatures.
fn generate_llms_from_graph(
    store: &Store,
    graph_db: &GraphDb,
    repo_path: &str,
    repo_id: &str,
    now: &str,
) -> Result<u32, String> {
    let (fn_count, type_count) = graph_db.count_symbols(repo_id)?;
    if fn_count == 0 && type_count == 0 { return Ok(0); }

    // Get all functions with their details
    let functions = graph_db.search_functions("", repo_id)?;
    let types = graph_db.search_types("", repo_id)?;

    if functions.is_empty() && types.is_empty() { return Ok(0); }

    // Group functions by file (module)
    let mut modules: std::collections::HashMap<String, Vec<&crate::types::FunctionDetail>> = std::collections::HashMap::new();
    for f in &functions {
        let module = f.file.split('/').last().unwrap_or(&f.file).to_string();
        modules.entry(module).or_default().push(f);
    }

    // Build index doc
    let repo_name = std::path::Path::new(repo_path)
        .file_name().and_then(|n| n.to_str()).unwrap_or(repo_id);

    let mut doc = format!("# {} — Auto-generated API Reference\n\n", repo_name);
    doc.push_str(&format!("> {} functions, {} types across {} modules\n\n", fn_count, type_count, modules.len()));

    // Types section
    if !types.is_empty() {
        doc.push_str("## Types\n\n");
        for t in types.iter().take(30) {
            let file_short = t.file.split('/').last().unwrap_or(&t.file);
            doc.push_str(&format!("- `{}` ({}) — {}\n", t.name, t.kind, file_short));
        }
        if types.len() > 30 {
            doc.push_str(&format!("- ... and {} more\n", types.len() - 30));
        }
        doc.push('\n');
    }

    // Modules section (top functions per module)
    doc.push_str("## Modules\n\n");
    let mut sorted_modules: Vec<_> = modules.iter().collect();
    sorted_modules.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

    for (module, fns) in sorted_modules.iter().take(20) {
        doc.push_str(&format!("### {}\n\n", module));
        for f in fns.iter().take(10) {
            let sig = f.signature.as_deref().unwrap_or("");
            let sig_short = if sig.len() > 80 { &sig[..80] } else { sig };
            doc.push_str(&format!("- `{}`", f.name));
            if !sig_short.is_empty() {
                doc.push_str(&format!(" — `{}`", sig_short));
            }
            doc.push('\n');
        }
        if fns.len() > 10 {
            doc.push_str(&format!("- ... and {} more functions\n", fns.len() - 10));
        }
        doc.push('\n');
    }

    let summary = format!("{} functions, {} types — auto-generated from code graph", fn_count, type_count);
    let id = format!("lib:{}:index", repo_id);

    store.upsert_lib_doc(
        &id, repo_id, &format!("{} API Reference", repo_name), None,
        &summary, Some(&doc[..doc.len().min(10000)]),
        "auto-generated", Some("index"), now,
    ).ok();

    store.upsert_lib_meta(repo_id, "local", Some(repo_path), None, now).ok();

    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_no_llms() {
        let dir = tempfile::TempDir::new().unwrap();
        let found = discover_llms_files(dir.path());
        assert!(found.is_empty());
    }

    #[test]
    fn discover_root_llms() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("llms.txt"), "# My Lib\n").unwrap();
        let found = discover_llms_files(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "index");
    }

    #[test]
    fn discover_llms_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("llms/components")).unwrap();
        std::fs::write(dir.path().join("llms/index.txt"), "# Overview\n").unwrap();
        std::fs::write(dir.path().join("llms/components/button.txt"), "# Button\n").unwrap();
        let found = discover_llms_files(dir.path());
        assert!(found.len() >= 2);
    }
}
