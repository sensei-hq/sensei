//! Pure scan logic — no DB, no task queue, no events.
//! These functions are called by the scan_root handler.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// A discovered folder with its classification.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredFolder {
    pub name: String,
    pub path: PathBuf,
    pub kind: FolderKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FolderKind {
    Git,
    WorkspaceMember,
    Subtree,
    Sibling,
    Standalone,
}

/// A project grouping discovered folders.
#[derive(Debug, Clone)]
pub struct DiscoveredProject {
    pub name: String,
    pub folders: Vec<DiscoveredFolder>,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// Find all .git directories under root up to max_depth.
/// Returns parent directories of .git (the actual git folders).
pub fn find_git_folders(root: &Path, max_depth: u32) -> Vec<PathBuf> {
    let mut result = Vec::new();
    walk_for_git(root, 0, max_depth, &mut result);
    result.sort();
    result
}

fn walk_for_git(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PathBuf>) {
    if depth > max_depth { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() || name.starts_with('.') { continue; }
        if ["node_modules", "dist", "build", "target"].contains(&name.as_str()) { continue; }

        if path.join(".git").is_dir() {
            out.push(path);
        } else {
            walk_for_git(&path, depth + 1, max_depth, out);
        }
    }
}

/// Compute the set of ancestor directories from git folders up to root.
pub fn ancestor_set(root: &Path, git_folders: &[PathBuf]) -> std::collections::HashSet<PathBuf> {
    let mut ancestors = std::collections::HashSet::new();
    for gf in git_folders {
        let mut current = gf.parent();
        while let Some(p) = current {
            if p == root { break; }
            ancestors.insert(p.to_path_buf());
            current = p.parent();
        }
    }
    ancestors
}

/// Collect all non-ignored subdirectories under root (one level deep per directory, recursive).
pub fn all_directories(root: &Path, max_depth: u32) -> Vec<PathBuf> {
    let mut result = Vec::new();
    walk_dirs(root, 0, max_depth, &mut result);
    result.sort();
    result
}

fn walk_dirs(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PathBuf>) {
    if depth > max_depth { return; }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() || name.starts_with('.') { continue; }
        if ["node_modules", "dist", "build", "target"].contains(&name.as_str()) { continue; }

        out.push(path.clone());
        // Don't recurse into git folders
        if !path.join(".git").is_dir() {
            walk_dirs(&path, depth + 1, max_depth, out);
        }
    }
}

/// Classify all directories: git, sibling, standalone, or ignored (ancestor).
/// Returns only non-ancestor, non-git-subfolder directories.
pub fn classify_folders(
    root: &Path,
    git_folders: &[PathBuf],
    all_dirs: &[PathBuf],
) -> Vec<DiscoveredFolder> {
    let git_set: std::collections::HashSet<&PathBuf> = git_folders.iter().collect();
    let ancestors = ancestor_set(root, git_folders);

    // Parents of git folders — used for sibling detection.
    // Exclude root itself: direct children of scan root are standalone, not siblings.
    let git_parents: std::collections::HashSet<PathBuf> = git_folders.iter()
        .filter_map(|gf| gf.parent().map(|p| p.to_path_buf()))
        .filter(|p| p != root)
        .collect();

    let mut result = Vec::new();

    for dir in all_dirs {
        // Skip git folders themselves (handled separately)
        if git_set.contains(dir) {
            let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
            result.push(DiscoveredFolder { name, path: dir.clone(), kind: FolderKind::Git });
            continue;
        }

        // Skip ancestors of git folders (intermediate directories)
        if ancestors.contains(dir) { continue; }

        // Skip subdirectories of git folders (internal to the repo)
        if git_folders.iter().any(|gf| dir.starts_with(gf)) { continue; }

        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

        // Sibling: shares parent with a git folder
        if let Some(parent) = dir.parent()
            && git_parents.contains(&parent.to_path_buf()) {
                result.push(DiscoveredFolder { name, path: dir.clone(), kind: FolderKind::Sibling });
                continue;
            }

        // Standalone: non-git, not a sibling
        result.push(DiscoveredFolder { name, path: dir.clone(), kind: FolderKind::Standalone });
    }

    result
}

/// Group classified folders into projects.
/// Git folders sharing a parent → one project.
/// Siblings join the same project as their git neighbours.
/// Standalone folders each get their own project.
pub fn group_into_projects(folders: &[DiscoveredFolder]) -> Vec<DiscoveredProject> {
    use std::collections::HashMap;

    let mut parent_groups: HashMap<PathBuf, Vec<DiscoveredFolder>> = HashMap::new();
    let mut standalones = Vec::new();

    for f in folders {
        match f.kind {
            FolderKind::Git | FolderKind::Sibling => {
                let parent = f.path.parent().unwrap_or(&f.path).to_path_buf();
                parent_groups.entry(parent).or_default().push(f.clone());
            }
            FolderKind::Standalone => {
                standalones.push(f.clone());
            }
            _ => {} // workspace members and subtrees handled elsewhere
        }
    }

    let mut projects = Vec::new();

    for (parent, members) in &parent_groups {
        let git_count = members.iter().filter(|f| f.kind == FolderKind::Git).count();

        if git_count >= 2 {
            // Multi-folder project: named after parent directory
            let name = parent.file_name().and_then(|n| n.to_str()).unwrap_or("project").to_string();
            projects.push(DiscoveredProject {
                name,
                folders: members.clone(),
                confidence: Confidence::High,
            });
        } else {
            // Solo git folder(s) under the same parent — each gets its own project
            for f in members {
                projects.push(DiscoveredProject {
                    name: f.name.clone(),
                    folders: vec![f.clone()],
                    confidence: Confidence::Medium,
                });
            }
        }
    }

    for f in &standalones {
        projects.push(DiscoveredProject {
            name: f.name.clone(),
            folders: vec![f.clone()],
            confidence: Confidence::Low,
        });
    }

    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects
}

/// Detect if a git folder is a monorepo (has workspace config).
pub fn is_monorepo(path: &Path) -> bool {
    // Cargo workspace
    if let Ok(content) = std::fs::read_to_string(path.join("Cargo.toml"))
        && content.contains("[workspace]") { return true; }
    // npm/pnpm workspace
    if let Ok(content) = std::fs::read_to_string(path.join("package.json"))
        && content.contains("\"workspaces\"") { return true; }
    if path.join("pnpm-workspace.yaml").exists() { return true; }
    // Go workspace
    if path.join("go.work").exists() { return true; }
    false
}

/// Detect technology stack from config files in a git folder.
pub fn detect_stack(path: &Path) -> Vec<String> {
    let mut stack = vec![];
    if path.join("Cargo.toml").exists() { stack.push("rust".into()); }
    if let Ok(pkg) = std::fs::read_to_string(path.join("package.json")) {
        if pkg.contains("\"svelte\"") || pkg.contains("\"@sveltejs/kit\"") { stack.push("svelte".into()); }
        else if pkg.contains("\"react\"") { stack.push("react".into()); }
        else if pkg.contains("\"vue\"") { stack.push("vue".into()); }
        else if pkg.contains("\"next\"") { stack.push("nextjs".into()); }
        else { stack.push("typescript".into()); }
    }
    if path.join("go.mod").exists() { stack.push("go".into()); }
    if path.join("pyproject.toml").exists() || path.join("requirements.txt").exists() { stack.push("python".into()); }
    if path.join("Package.swift").exists() { stack.push("swift".into()); }
    if path.join("Gemfile").exists() { stack.push("ruby".into()); }
    stack
}

/// Count indexable files in a git folder (respecting ignore patterns).
/// Returns (file_paths, total_count).
pub fn count_indexable_files(path: &Path) -> (Vec<PathBuf>, u32) {
    let exclude = super::helpers::build_globset();
    let mut files = Vec::new();

    let walker = ignore::WalkBuilder::new(path)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry in walker.flatten() {
        if !entry.path().is_file() { continue; }
        let rel = entry.path().strip_prefix(path).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy();
        if exclude.is_match(&*rel_str) { continue; }

        let ext = entry.path().extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext.is_empty() { continue; }
        if super::helpers::is_binary_ext(ext) { continue; }

        files.push(entry.path().to_path_buf());
    }

    let count = files.len() as u32;
    (files, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_fixture() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // proj_a/fldr_1 — git
        std::fs::create_dir_all(root.join("proj_a/fldr_1/.git")).unwrap();
        std::fs::write(root.join("proj_a/fldr_1/Cargo.toml"), "[package]\nname=\"f1\"").unwrap();

        // proj_a/fldr_2 — git
        std::fs::create_dir_all(root.join("proj_a/fldr_2/.git")).unwrap();
        std::fs::write(root.join("proj_a/fldr_2/package.json"), r#"{"name":"f2"}"#).unwrap();

        // proj_a/fldr_3 — git
        std::fs::create_dir_all(root.join("proj_a/fldr_3/.git")).unwrap();
        std::fs::write(root.join("proj_a/fldr_3/go.mod"), "module f3").unwrap();

        // proj_a/meeting_notes — no .git (sibling)
        std::fs::create_dir_all(root.join("proj_a/meeting_notes")).unwrap();

        // standalone — git, solo
        std::fs::create_dir_all(root.join("standalone/.git")).unwrap();

        // random_docs — no .git (standalone)
        std::fs::create_dir_all(root.join("random_docs")).unwrap();

        tmp
    }

    #[test]
    fn find_git_folders_discovers_all() {
        let fixture = create_fixture();
        let gits = find_git_folders(fixture.path(), 3);
        let names: Vec<&str> = gits.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names.len(), 4);
        assert!(names.contains(&"fldr_1"));
        assert!(names.contains(&"fldr_2"));
        assert!(names.contains(&"fldr_3"));
        assert!(names.contains(&"standalone"));
    }

    #[test]
    fn ancestor_set_computes_intermediates() {
        let fixture = create_fixture();
        let gits = find_git_folders(fixture.path(), 3);
        let anc = ancestor_set(fixture.path(), &gits);
        // proj_a is an ancestor of fldr_1, fldr_2, fldr_3
        assert!(anc.contains(&fixture.path().join("proj_a")));
        // root is NOT included (we stop at root)
        assert!(!anc.contains(&fixture.path().to_path_buf()));
    }

    #[test]
    fn classify_finds_git_sibling_standalone() {
        let fixture = create_fixture();
        let gits = find_git_folders(fixture.path(), 3);
        let dirs = all_directories(fixture.path(), 3);
        let classified = classify_folders(fixture.path(), &gits, &dirs);

        let git_names: Vec<&str> = classified.iter()
            .filter(|f| f.kind == FolderKind::Git)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(git_names.len(), 4);
        assert!(git_names.contains(&"fldr_1"));
        assert!(git_names.contains(&"standalone"));

        let siblings: Vec<&str> = classified.iter()
            .filter(|f| f.kind == FolderKind::Sibling)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(siblings, vec!["meeting_notes"]);

        let standalones: Vec<&str> = classified.iter()
            .filter(|f| f.kind == FolderKind::Standalone)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(standalones, vec!["random_docs"]);
    }

    #[test]
    fn group_creates_projects() {
        let fixture = create_fixture();
        let gits = find_git_folders(fixture.path(), 3);
        let dirs = all_directories(fixture.path(), 3);
        let classified = classify_folders(fixture.path(), &gits, &dirs);
        let projects = group_into_projects(&classified);

        // proj_a: 3 git + 1 sibling = 4 folders
        let proj_a = projects.iter().find(|p| p.name == "proj_a");
        assert!(proj_a.is_some());
        assert_eq!(proj_a.unwrap().folders.len(), 4);
        assert_eq!(proj_a.unwrap().confidence, Confidence::High);

        // standalone: 1 git folder → standalone project
        let standalone_proj = projects.iter().find(|p| p.name == "standalone");
        assert!(standalone_proj.is_some());
        assert_eq!(standalone_proj.unwrap().folders.len(), 1);
        assert_eq!(standalone_proj.unwrap().confidence, Confidence::Medium);

        // random_docs: standalone non-git
        let docs_proj = projects.iter().find(|p| p.name == "random_docs");
        assert!(docs_proj.is_some());
        assert_eq!(docs_proj.unwrap().confidence, Confidence::Low);
    }

    #[test]
    fn monorepo_detected() {
        let tmp = tempfile::tempdir().unwrap();
        let mono = tmp.path().join("mono");
        std::fs::create_dir_all(mono.join(".git")).unwrap();
        std::fs::write(mono.join("Cargo.toml"), "[workspace]\nmembers = [\"crates/*\"]").unwrap();
        assert!(is_monorepo(&mono));
    }

    #[test]
    fn non_monorepo_not_flagged() {
        let tmp = tempfile::tempdir().unwrap();
        let regular = tmp.path().join("regular");
        std::fs::create_dir_all(regular.join(".git")).unwrap();
        std::fs::write(regular.join("Cargo.toml"), "[package]\nname = \"regular\"").unwrap();
        assert!(!is_monorepo(&regular));
    }

    #[test]
    fn classify_excludes_git_subfolders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // git folder with src subdirectory
        std::fs::create_dir_all(root.join("myrepo/.git")).unwrap();
        std::fs::create_dir_all(root.join("myrepo/src")).unwrap();
        std::fs::create_dir_all(root.join("myrepo/tests")).unwrap();

        let gits = find_git_folders(root, 3);
        let dirs = all_directories(root, 3);
        let classified = classify_folders(root, &gits, &dirs);

        // Only myrepo should appear, not myrepo/src or myrepo/tests
        assert_eq!(classified.len(), 1);
        assert_eq!(classified[0].name, "myrepo");
        assert_eq!(classified[0].kind, FolderKind::Git);
    }

    // ── Stack detection ──────────────────────────────────────────

    #[test]
    fn detect_stack_rust() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"x\"").unwrap();
        assert_eq!(detect_stack(tmp.path()), vec!["rust"]);
    }

    #[test]
    fn detect_stack_svelte() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("package.json"), r#"{"dependencies":{"svelte":"^5"}}"#).unwrap();
        assert_eq!(detect_stack(tmp.path()), vec!["svelte"]);
    }

    #[test]
    fn detect_stack_go() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("go.mod"), "module x\ngo 1.22").unwrap();
        assert_eq!(detect_stack(tmp.path()), vec!["go"]);
    }

    #[test]
    fn detect_stack_multiple() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(tmp.path().join("pyproject.toml"), "[project]").unwrap();
        let stack = detect_stack(tmp.path());
        assert!(stack.contains(&"rust".to_string()));
        assert!(stack.contains(&"python".to_string()));
    }

    #[test]
    fn detect_stack_empty_for_no_config() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(detect_stack(tmp.path()).is_empty());
    }

    // ── File counting ────────────────────────────────────────────

    #[test]
    fn count_indexable_files_in_fixture() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(tmp.path().join("src/lib.rs"), "pub fn x() {}").unwrap();

        let (files, count) = count_indexable_files(tmp.path());
        assert!(count >= 2, "expected at least 2 files (main.rs, lib.rs), got {}", count);
        assert!(files.iter().any(|f| f.to_string_lossy().contains("main.rs")));
    }

    #[test]
    fn count_excludes_binary_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join("code.rs"), "fn x() {}").unwrap();
        std::fs::write(tmp.path().join("image.png"), &[0u8; 10]).unwrap();
        std::fs::write(tmp.path().join("font.woff2"), &[0u8; 10]).unwrap();

        let (_, count) = count_indexable_files(tmp.path());
        assert_eq!(count, 1, "only .rs should be counted, not .png or .woff2");
    }
}
