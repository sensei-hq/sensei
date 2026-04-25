use std::path::Path;
use walkdir::WalkDir;

/// Discover and index llms.txt files from a repo.
#[allow(dead_code)]
pub fn index_llms(
    repo_path: &str,
    _repo_id: &str,
) -> Result<u32, String> {
    let repo = Path::new(repo_path);
    let llms_files = discover_llms_files(repo);
    Ok(llms_files.len() as u32)
}

/// Find llms.txt files in standard locations within a repo.
/// Returns (component_name, file_path) pairs.
#[allow(dead_code)]
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
#[allow(dead_code)]
fn generate_llms_from_graph(
    _repo_path: &str,
    _repo_id: &str,
    _now: &str,
) -> Result<u32, String> {
    // TODO: implement
    Ok(0)
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
