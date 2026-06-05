//! Pure scan logic — no DB, no task queue, no events.
//! These functions are called by the scan_root handler.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Directory names skipped during the scan walk: dependency/build output and
/// generated/OS junk that never contains first-party source. Kept in one place
/// so `walk_for_git` and `walk_dirs` agree.
const IGNORED_DIRS: &[&str] = &[
    "node_modules", "dist", "build", "target", "__pycache__", "__MACOSX",
];

/// A discovered folder with its classification.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredFolder {
    pub name: String,
    pub path: PathBuf,
    pub kind: FolderKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FolderKind {
    /// A real git repository (has a `.git`) — a project root.
    Git,
    /// A non-git directory that looks like a project the developer started but
    /// never `git init`'d ("quasi-repo") — also treated as a project root.
    Standalone,
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
        if IGNORED_DIRS.contains(&name.as_str()) { continue; }

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
        if IGNORED_DIRS.contains(&name.as_str()) { continue; }

        out.push(path.clone());
        // Don't recurse into git folders
        if !path.join(".git").is_dir() {
            walk_dirs(&path, depth + 1, max_depth, out);
        }
    }
}

/// Classify directories into **project roots** only: real git repos (`Git`) and
/// "quasi-repos" — non-git directories that look like a project the developer
/// started but never `git init`'d (`Standalone`).
///
/// A non-git directory is a quasi-repo when it sits at a project-root position
/// (not inside any git repo or another quasi-repo, and not a grouping container
/// of git repos) AND `has_code` reports indexable source for it. Candidates are
/// considered shallowest-first so a nested directory is recognised as content of
/// the project root above it rather than promoted to its own project.
///
/// Everything else — grouping containers, code-less loose folders, and any
/// subfolder inside a project root — is intentionally NOT returned. The scan
/// tracks project roots; it never promotes subfolders to repos (those become
/// `kind=folder` rows under their parent, handled separately).
pub fn classify_folders(
    root: &Path,
    git_folders: &[PathBuf],
    all_dirs: &[PathBuf],
    has_code: impl Fn(&Path) -> bool,
) -> Vec<DiscoveredFolder> {
    let git_set: std::collections::HashSet<&PathBuf> = git_folders.iter().collect();
    let ancestors = ancestor_set(root, git_folders);

    let mut result: Vec<DiscoveredFolder> = git_folders.iter()
        .map(|gf| DiscoveredFolder {
            name: gf.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string(),
            path: gf.clone(),
            kind: FolderKind::Git,
        })
        .collect();

    // Project roots grow as quasi-repos are discovered; seed with git repos so a
    // non-git dir inside a repo is never re-promoted.
    let mut project_roots: Vec<PathBuf> = git_folders.to_vec();

    // Candidate non-git directories, shallowest first.
    let mut candidates: Vec<&PathBuf> = all_dirs.iter()
        .filter(|d| !git_set.contains(*d))                              // not a git repo
        .filter(|d| !ancestors.contains(*d))                           // not a git-repo grouping container
        .filter(|d| !git_folders.iter().any(|gf| d.starts_with(gf)))   // not inside a git repo
        .collect();
    candidates.sort_by_key(|d| d.components().count());

    for dir in candidates {
        // Inside a project root already chosen (git or quasi)? → it's content, skip.
        if project_roots.iter().any(|pr| pr != dir && dir.starts_with(pr)) {
            continue;
        }
        if has_code(dir) {
            project_roots.push((*dir).clone());
            result.push(DiscoveredFolder {
                name: dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string(),
                path: (*dir).clone(),
                kind: FolderKind::Standalone,
            });
        }
        // else: code-less loose directory → not a project root, not registered.
    }

    result
}

/// True if a directory looks like a project root with indexable source: it has a
/// recognised manifest (`detect_stack`) or at least one non-binary source file
/// directly inside it. Distinguishes a "quasi-repo" (a project the developer
/// forgot to `git init`) from a data / asset / junk folder.
pub fn has_indexable_code(dir: &Path) -> bool {
    if !detect_stack(dir).is_empty() {
        return true;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return false; };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.is_empty() { continue; }
        if super::helpers::is_binary_ext(ext) { continue; }
        return true;
    }
    false
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

    let walker = super::helpers::build_walker(path).build();

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
    fn classify_returns_only_git_when_non_git_dirs_have_no_code() {
        // meeting_notes and random_docs in the fixture have no files → not
        // quasi-repos. classify should return just the 4 git repos.
        let fixture = create_fixture();
        let gits = find_git_folders(fixture.path(), 3);
        let dirs = all_directories(fixture.path(), 3);
        let classified = classify_folders(fixture.path(), &gits, &dirs, has_indexable_code);

        let git_names: Vec<&str> = classified.iter()
            .filter(|f| f.kind == FolderKind::Git)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(git_names.len(), 4);
        assert!(git_names.contains(&"fldr_1"));
        assert!(git_names.contains(&"standalone"));

        // No quasi-repos: the only non-git dirs hold no indexable code.
        assert!(!classified.iter().any(|f| f.kind == FolderKind::Standalone));
    }

    #[test]
    fn classify_detects_quasi_repo_with_code() {
        // A non-git directory at project-root position WITH a manifest is a
        // quasi-repo; a code-less sibling next to it is not.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // a real git repo so quasi-repo siblings are at project-root position
        std::fs::create_dir_all(root.join("real-repo/.git")).unwrap();
        std::fs::write(root.join("real-repo/Cargo.toml"), "[package]\nname=\"r\"").unwrap();
        // forgot-to-git-init project (manifest) → quasi-repo
        std::fs::create_dir_all(root.join("forgotten")).unwrap();
        std::fs::write(root.join("forgotten/package.json"), r#"{"name":"f"}"#).unwrap();
        // a top-level non-git dir with a loose source file → quasi-repo
        std::fs::create_dir_all(root.join("scripts")).unwrap();
        std::fs::write(root.join("scripts/run.py"), "print('hi')\n").unwrap();
        // junk: only data/binary + nothing → not a quasi-repo
        std::fs::create_dir_all(root.join("Archive")).unwrap();
        std::fs::write(root.join("Archive/photo.png"), [0u8; 8]).unwrap();

        let gits = find_git_folders(root, 3);
        let dirs = all_directories(root, 3);
        let classified = classify_folders(root, &gits, &dirs, has_indexable_code);

        let quasi: Vec<&str> = classified.iter()
            .filter(|f| f.kind == FolderKind::Standalone)
            .map(|f| f.name.as_str())
            .collect();
        assert!(quasi.contains(&"forgotten"), "manifest folder is a quasi-repo");
        assert!(quasi.contains(&"scripts"), "loose-source folder is a quasi-repo");
        assert!(!quasi.contains(&"Archive"), "binary-only folder is not a quasi-repo");
    }

    #[test]
    fn classify_does_not_promote_subfolders_of_a_quasi_repo() {
        // A quasi-repo's own subdirectories must not each become a project root.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("proj/src")).unwrap();
        std::fs::write(root.join("proj/Cargo.toml"), "[package]\nname=\"p\"").unwrap();
        std::fs::write(root.join("proj/src/main.rs"), "fn main() {}").unwrap();

        let gits = find_git_folders(root, 3);
        let dirs = all_directories(root, 3);
        let classified = classify_folders(root, &gits, &dirs, has_indexable_code);

        // Exactly one project root: `proj`. `proj/src` is content, not a repo.
        assert_eq!(classified.len(), 1);
        assert_eq!(classified[0].name, "proj");
        assert_eq!(classified[0].kind, FolderKind::Standalone);
    }

    #[test]
    fn has_indexable_code_distinguishes_projects_from_junk() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = tmp.path().join("m");
        std::fs::create_dir_all(&manifest).unwrap();
        std::fs::write(manifest.join("go.mod"), "module m").unwrap();
        assert!(has_indexable_code(&manifest), "manifest => code");

        let source = tmp.path().join("s");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("util.ts"), "export const x = 1;").unwrap();
        assert!(has_indexable_code(&source), "loose source => code");

        let assets = tmp.path().join("a");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("logo.png"), [0u8; 8]).unwrap();
        assert!(!has_indexable_code(&assets), "binary-only => no code");

        let empty = tmp.path().join("e");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(!has_indexable_code(&empty), "empty dir => no code");
    }

    #[test]
    fn walk_skips_generated_dirs() {
        // __pycache__ and __MACOSX must not be walked or classified.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/.git")).unwrap();
        std::fs::write(root.join("repo/Cargo.toml"), "[package]\nname=\"r\"").unwrap();
        std::fs::create_dir_all(root.join("__pycache__")).unwrap();
        std::fs::write(root.join("__pycache__/x.pyc"), [0u8; 4]).unwrap();
        std::fs::create_dir_all(root.join("__MACOSX")).unwrap();

        let dirs = all_directories(root, 3);
        assert!(!dirs.iter().any(|d| d.ends_with("__pycache__")));
        assert!(!dirs.iter().any(|d| d.ends_with("__MACOSX")));
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
        let classified = classify_folders(root, &gits, &dirs, has_indexable_code);

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
        std::fs::write(tmp.path().join("image.png"), [0u8; 10]).unwrap();
        std::fs::write(tmp.path().join("font.woff2"), [0u8; 10]).unwrap();

        let (_, count) = count_indexable_files(tmp.path());
        assert_eq!(count, 1, "only .rs should be counted, not .png or .woff2");
    }
}
