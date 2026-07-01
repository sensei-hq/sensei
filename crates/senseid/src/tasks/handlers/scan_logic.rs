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

/// Build the complete subfolder tree under a project root from the set of
/// directories that contain indexable files. Intermediate ancestors (dirs that
/// hold no files directly but lie between the root and a file-bearing dir) are
/// included so the tree has no gaps. Returns `(dir, parent_dir)` pairs ordered
/// parent-before-child; the parent of a top-level dir is `repo_path` itself.
/// The repo root is never included — storage starts at the project root, and
/// wrapper directories above it are never passed in.
pub fn subfolder_tree(repo_path: &Path, file_dirs: &[PathBuf]) -> Vec<(PathBuf, PathBuf)> {
    let mut all: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for d in file_dirs {
        if d == repo_path || !d.starts_with(repo_path) {
            continue;
        }
        let mut cur: Option<&Path> = Some(d.as_path());
        while let Some(p) = cur {
            if p == repo_path || !p.starts_with(repo_path) {
                break;
            }
            all.insert(p.to_path_buf());
            cur = p.parent();
        }
    }
    let mut sorted: Vec<PathBuf> = all.into_iter().collect();
    // Shallowest first so a parent is always created before its children.
    sorted.sort_by_key(|d| d.components().count());
    sorted
        .into_iter()
        .map(|d| {
            let parent = d.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| repo_path.to_path_buf());
            (d, parent)
        })
        .collect()
}

/// How confident we are that a non-git directory is a real project root.
/// Returned by [`classify_quasi_repo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuasiKind {
    /// Has a recognised manifest (`Cargo.toml`, `package.json`, `go.mod`, …) —
    /// strongly likely a real project the developer simply never `git init`'d.
    Manifest,
    /// No manifest, but holds recognised source / Markdown files — likely a
    /// project but unconfirmed (a scattered old code store, a docs folder).
    /// Indexed, but flagged for the user to keep / organise / discard.
    LooseCode,
}

/// True if a file extension marks first-party *source* the scanner treats as a
/// project signal: a language the parser supports, a common source language we
/// recognise without a parser adapter, or Markdown docs. Data, config, and
/// binaries (`.csv`, `.json`, `.txt`, `.png`, …) deliberately do NOT count — a
/// folder of only those is not a project.
pub fn is_project_source_ext(ext: &str) -> bool {
    let e = ext.trim_start_matches('.').to_ascii_lowercase();
    // Markdown docs: a docs-only folder is a documentation project.
    if matches!(e.as_str(), "md" | "mdx") {
        return true;
    }
    // Anything the parser has a language adapter for is code.
    if crate::languages::adapter_for_ext(&format!(".{e}")).is_some() {
        return true;
    }
    // Common source languages we recognise as code even without a parser adapter.
    matches!(
        e.as_str(),
        "go" | "rb" | "sh" | "bash" | "zsh" | "fish" | "pl" | "pm" | "php"
            | "lua" | "r" | "jl" | "scala" | "ex" | "exs" | "erl" | "hs" | "ml"
            | "dart" | "cs" | "fs" | "fsx" | "clj" | "cljs" | "groovy" | "m" | "mm"
            | "cxx" | "hh" | "hxx" | "swift" | "scss" | "css" | "html"
    )
}

/// Classify a non-git directory as a quasi-repo (a project the developer never
/// `git init`'d) and how confident we are. `None` means "not a project root" —
/// only data / config / binaries / nothing recognised — so it is not promoted.
///
/// Tier 1 [`QuasiKind::Manifest`]: a recognised manifest → confident project.
/// Tier 2 [`QuasiKind::LooseCode`]: ≥1 recognised source / `.md` file but no
/// manifest → indexed, then flagged for review.
pub fn classify_quasi_repo(dir: &Path) -> Option<QuasiKind> {
    if !detect_stack(dir).is_empty() {
        return Some(QuasiKind::Manifest);
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && is_project_source_ext(ext)
        {
            return Some(QuasiKind::LooseCode);
        }
    }
    None
}

/// True if a directory is a project root worth indexing — a quasi-repo of either
/// tier (manifest-backed or loose source). Used by [`classify_folders`] to gate
/// promotion; the tier itself ([`classify_quasi_repo`]) drives the review flag.
pub fn has_indexable_code(dir: &Path) -> bool {
    classify_quasi_repo(dir).is_some()
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

/// The incremental re-index decision for a folder: which files to (re)process
/// and which to drop. Pure output of [`incremental_plan`].
#[derive(Debug, Default, PartialEq)]
pub struct IncrementalPlan {
    /// Files to (re)index: new, or whose mtime changed since the last index.
    pub changed: std::collections::HashSet<String>,
    /// Files indexed before but no longer present on disk — drop their nodes.
    pub removed: Vec<String>,
}

/// Diff the working tree against the last index. `current` is the set of
/// indexable files as `(rel_path, mtime_ms)` found on disk now; `prior` maps
/// each previously-indexed `rel_path` to its recorded mtime. A file is
/// `changed` when it is new or its mtime differs; `removed` when it was indexed
/// before but is gone now. An empty `prior` (first index) marks everything
/// changed. Pure — all I/O happens in the caller.
pub fn incremental_plan(
    current: &[(String, i64)],
    prior: &std::collections::HashMap<String, i64>,
) -> IncrementalPlan {
    let current_set: std::collections::HashSet<&String> = current.iter().map(|(p, _)| p).collect();
    let changed = current.iter()
        .filter(|(path, mtime)| prior.get(path).is_none_or(|prev| prev != mtime))
        .map(|(path, _)| path.clone())
        .collect();
    let removed = prior.keys()
        .filter(|path| !current_set.contains(*path))
        .cloned()
        .collect();
    IncrementalPlan { changed, removed }
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
    // .NET — solution/project manifests use globbed names (Foo.sln, Bar.csproj),
    // so scan the directory rather than checking a fixed filename. A fixed
    // global.json (SDK pin) also marks a .NET root.
    if path.join("global.json").exists() || dir_has_ext(path, &["sln", "csproj", "fsproj", "vbproj"]) {
        stack.push("dotnet".into());
    }
    stack
}

/// Infer a folder's semantic role (a `folder_role` enum value) from its manifest
/// and layout — for workspace members / folders with no explicit README `role:`
/// frontmatter (frontmatter always wins; see process::reconcile_repo_identity).
/// Reads the folder's manifests, then delegates to the pure [`classify_role`].
/// Returns `None` to leave the role unset.
pub fn infer_role(path: &Path) -> Option<&'static str> {
    let cargo = std::fs::read_to_string(path.join("Cargo.toml")).ok();
    let pkg = std::fs::read_to_string(path.join("package.json")).ok();
    classify_role(
        cargo.as_deref(),
        pkg.as_deref(),
        path.join("src/lib.rs").exists(),
        path.join("src").join("routes").exists(),
        path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
    )
}

/// Pure role classifier from manifest contents + layout flags. Precedence:
/// tool (ships a binary) > website (web app framework) > library (publishable
/// lib) > docs. Kept pure so it is unit-testable without a filesystem.
pub fn classify_role(
    cargo: Option<&str>,
    pkg: Option<&str>,
    has_lib_rs: bool,
    has_routes: bool,
    dir_name: &str,
) -> Option<&'static str> {
    // 1. Tool — a crate/package that ships an executable: an explicit Rust
    //    `[[bin]]` target or a node `bin` field. A bare `main.rs` is
    //    deliberately NOT enough: a daemon (e.g. senseid, axum-based) is a
    //    binary too, so requiring an explicit bin declaration avoids
    //    mislabeling a backend service as a CLI tool.
    let rust_bin = cargo.is_some_and(|c| c.contains("[[bin]]"));
    let node_bin = pkg.is_some_and(|p| p.contains("\"bin\""));
    if rust_bin || node_bin {
        return Some("tool");
    }

    // 2. Website — a web-app framework (checked before library: an app's
    //    package.json also has a name, but it is not a publishable lib). For
    //    SvelteKit/Vite require an actual `src/routes` app tree: a *library* can
    //    list `@sveltejs/kit` as a peer/dev dep (e.g. a UnoCSS preset) without
    //    being an app, so the manifest marker alone isn't enough.
    let is_web_app = pkg.is_some_and(|p| p.contains("\"next\"") || p.contains("\"astro\""))
        || (has_routes && pkg.is_some_and(|p| p.contains("\"@sveltejs/kit\"") || p.contains("\"vite\"")));
    if is_web_app {
        return Some("website");
    }

    // 3. Library — a lib crate (src/lib.rs or [lib]) or a publishable package
    //    (has a name plus an entry point, and no bin/app markers above).
    let rust_lib = has_lib_rs || cargo.is_some_and(|c| c.contains("[lib]"));
    let node_lib = pkg.is_some_and(|p| {
        p.contains("\"name\"")
            && (p.contains("\"exports\"")
                || p.contains("\"main\"")
                || p.contains("\"module\":")
                || p.contains("\"svelte\""))
    });
    if rust_lib || node_lib {
        return Some("library");
    }

    // 4. Docs directory.
    if dir_name == "docs" {
        return Some("docs");
    }
    None
}

/// Find nested sub-project directories under a repo root: directories (other
/// than the root) that carry their own `Cargo.toml` or `package.json`. This
/// covers declared workspace members (`packages/*`, `crates/*`, `apps/*`) *and*
/// standalone sub-apps that are not workspace members (e.g. a `site/` inside a
/// Cargo workspace). Does not descend into a sub-project once found (a package's
/// own subdirectories are not separate sub-projects) nor into ignored/build/OS
/// dirs, and is depth-bounded so the walk stays cheap on large trees.
pub fn find_subprojects(root: &Path, max_depth: u32) -> Vec<PathBuf> {
    let mut out = Vec::new();
    find_subprojects_walk(root, 0, max_depth, &mut out);
    out.sort();
    out
}

fn find_subprojects_walk(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PathBuf>) {
    if depth >= max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || IGNORED_DIRS.contains(&name) {
            continue;
        }
        if p.join("Cargo.toml").exists() || p.join("package.json").exists() {
            // A sub-project boundary: record it and stop descending into it.
            out.push(p);
            continue;
        }
        find_subprojects_walk(&p, depth + 1, max_depth, out);
    }
}

/// True if the directory directly contains a file with one of the given
/// (lowercase, no-dot) extensions. Used for manifests whose names are globbed
/// rather than fixed (e.g. .NET `*.csproj` / `*.sln`).
fn dir_has_ext(path: &Path, exts: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else { return false; };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() { continue; }
        if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && exts.contains(&ext.to_ascii_lowercase().as_str())
        {
            return true;
        }
    }
    false
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

/// True if a directory tree holds at least one indexable (non-binary) source
/// file, respecting the same ignore patterns the scan uses. Short-circuits on
/// the first match. Used by the scan reconcile to tell a provably-dead former
/// project root (empty / no content left on disk) from one that still carries
/// real content the user may want to keep.
pub fn dir_has_indexable_content(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let exclude = super::helpers::build_globset();
    let walker = super::helpers::build_walker(path).build();
    for entry in walker.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(path).unwrap_or(entry.path());
        if exclude.is_match(&*rel.to_string_lossy()) {
            continue;
        }
        let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.is_empty() || super::helpers::is_binary_ext(ext) {
            continue;
        }
        return true;
    }
    false
}

/// What to do with a DB-recorded project root the current scan did NOT
/// re-discover. Returned by [`classify_stale_root`].
#[derive(Debug, Clone, PartialEq)]
pub enum StaleAction {
    /// Still a live project root (re-discovered this scan) — leave it alone.
    Keep,
    /// Provably dead: the path is gone, is an empty husk whose indexed nodes no
    /// longer reflect disk, or now sits inside a still-live root that owns the
    /// subtree. Remove the row (cascading its nodes and subtree).
    Remove,
    /// The path still exists with real content but has no live owner — this is
    /// ambiguous (an archive the user kept? a container?), so tag it stale and
    /// let the user decide. The scan never auto-deletes unaccounted-for content.
    MarkStale,
}

/// Decide the fate of a DB-recorded project root (`kind` git/standalone/subtree)
/// that the current scan did not re-discover as a root. Pure: the disk facts
/// (`exists`, `has_content`) are injected so the rule is unit-testable.
///
/// `live_roots` is the set of project-root paths the scan just discovered (real
/// git repos + quasi-repos). The scan is otherwise additive — it only ever
/// *registers* roots it finds — so without this reconcile a root that lost its
/// `.git`, was emptied, or was moved would linger forever as a phantom project.
pub fn classify_stale_root(
    folder: &Path,
    live_roots: &std::collections::HashSet<PathBuf>,
    exists: bool,
    has_content: bool,
) -> StaleAction {
    if live_roots.contains(folder) {
        return StaleAction::Keep;
    }
    // Inside a still-live root → that root now owns this subtree (it gets
    // re-materialised as kind=folder rows), so drop the stale root row and let
    // it be recreated under the correct parent.
    if live_roots.iter().any(|r| r.as_path() != folder && folder.starts_with(r)) {
        return StaleAction::Remove;
    }
    if !exists || !has_content {
        return StaleAction::Remove; // gone, or an empty husk with stale nodes
    }
    StaleAction::MarkStale
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── role inference ───────────────────────────────────────────────────
    #[test]
    fn classify_role_tool_from_binary() {
        // Explicit Cargo [[bin]] → tool (dbd's cli crate, sensei cli/mcp).
        assert_eq!(classify_role(Some("[package]\nname=\"cli\"\n\n[[bin]]\nname=\"dbd\""), None, false, false, "cli"), Some("tool"));
        // Node package that ships a CLI (`bin` field) (rokkit packages/cli) → tool.
        assert_eq!(classify_role(None, Some("{\"name\":\"c\",\"bin\":{\"c\":\"./c.js\"}}"), false, false, "c"), Some("tool"));
        // A daemon binary (main.rs, NO [[bin]], server deps) is NOT a CLI tool —
        // it stays unclassified so a frontmatter role can label it backend.
        assert_eq!(classify_role(Some("[package]\nname=\"senseid\"\n\n[dependencies]\naxum = \"0.7\"\nclap = \"4\""), None, false, false, "senseid"), None);
    }

    #[test]
    fn classify_role_website_from_web_framework() {
        // SvelteKit app (rokkit apps/learn, dbd site) → website.
        assert_eq!(classify_role(None, Some("{\"name\":\"learn\",\"devDependencies\":{\"@sveltejs/kit\":\"^2\"}}"), false, true, "learn"), Some("website"));
        // Web-app markers win over the library marker (an app also has a name).
        assert_eq!(classify_role(None, Some("{\"name\":\"site\",\"type\":\"module\",\"devDependencies\":{\"@sveltejs/kit\":\"^2\"}}"), false, true, "site"), Some("website"));
    }

    #[test]
    fn classify_role_library_from_lib_crate_or_package() {
        // Rust lib crate (dbd's core crates) → library.
        assert_eq!(classify_role(Some("[package]\nname=\"core\""), None, true, false, "core"), Some("library"));
        // Publishable node package with exports (rokkit packages/*) → library.
        assert_eq!(classify_role(None, Some("{\"name\":\"@rokkit/ui\",\"exports\":{\".\":\"./index.js\"}}"), false, false, "ui"), Some("library"));
        // A library that only lists @sveltejs/kit as a peer/dev dep (no src/routes)
        // is NOT a website (rokkit's unocss preset) → library.
        assert_eq!(classify_role(None, Some("{\"name\":\"@rokkit/unocss\",\"exports\":{\".\":\"./i.js\"},\"peerDependencies\":{\"@sveltejs/kit\":\"^2\"}}"), false, false, "unocss"), Some("library"));
        // `"type": "module"` alone must NOT read as a library entry point.
        assert_eq!(classify_role(None, Some("{\"name\":\"x\",\"type\":\"module\"}"), false, false, "x"), None);
    }

    #[test]
    fn classify_role_none_or_docs() {
        assert_eq!(classify_role(None, None, false, false, "misc"), None);
        // A private root manifest with no name/entry stays unclassified.
        assert_eq!(classify_role(None, Some("{\"private\":true,\"workspaces\":[\"packages/*\"]}"), false, false, "root"), None);
        assert_eq!(classify_role(None, None, false, false, "docs"), Some("docs"));
    }

    #[test]
    fn infer_role_reads_manifest_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname=\"c\"\n[[bin]]\nname=\"c\"").unwrap();
        std::fs::write(tmp.path().join("src/main.rs"), "fn main(){}").unwrap();
        assert_eq!(infer_role(tmp.path()), Some("tool"));
    }

    #[test]
    fn find_subprojects_covers_members_and_standalone_apps() {
        // Model a Cargo-workspace repo (like dbd-rs): crates/* members plus a
        // non-member `site/` app, plus a build dir that must be ignored.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[\"crates/*\"]").unwrap();
        for c in ["dbd-cli", "dbd-core"] {
            std::fs::create_dir_all(root.join("crates").join(c)).unwrap();
            std::fs::write(root.join("crates").join(c).join("Cargo.toml"), "[package]").unwrap();
        }
        // A sub-project's internals must NOT be reported as sub-projects.
        std::fs::create_dir_all(root.join("crates/dbd-core/src")).unwrap();
        std::fs::write(root.join("crates/dbd-core/src/lib.rs"), "").unwrap();
        std::fs::create_dir_all(root.join("site")).unwrap();
        std::fs::write(root.join("site/package.json"), "{\"name\":\"site\"}").unwrap();
        // Build output must be skipped even though it may contain manifests.
        std::fs::create_dir_all(root.join("target/pkg")).unwrap();
        std::fs::write(root.join("target/pkg/Cargo.toml"), "[package]").unwrap();

        let found: Vec<String> = find_subprojects(root, 3)
            .iter()
            .map(|p| p.strip_prefix(root).unwrap().to_string_lossy().to_string())
            .collect();
        assert_eq!(found, vec!["crates/dbd-cli", "crates/dbd-core", "site"]);
    }

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
    fn is_project_source_ext_covers_code_and_md_not_data() {
        // parser languages + common unparsed source + markdown count
        for e in ["py", "rs", "ts", "cpp", "h", "go", "rb", "sh", "pl", "php", "lua", "md", "mdx"] {
            assert!(is_project_source_ext(e), "{e} should count as project source");
            assert!(is_project_source_ext(&format!(".{e}")), "leading-dot {e} should count too");
        }
        // data / config / binaries are NOT a project signal on their own
        for e in ["csv", "txt", "json", "yaml", "toml", "png", "lock", "log", "pdf"] {
            assert!(!is_project_source_ext(e), "{e} should NOT count as project source");
        }
    }

    #[test]
    fn classify_quasi_repo_tiers_manifest_loose_and_none() {
        let tmp = tempfile::tempdir().unwrap();

        // Tier 1 — manifest → confident project
        let manifest = tmp.path().join("manifest");
        std::fs::create_dir_all(&manifest).unwrap();
        std::fs::write(manifest.join("Cargo.toml"), "[package]\nname=\"m\"").unwrap();
        assert_eq!(classify_quasi_repo(&manifest), Some(QuasiKind::Manifest));

        // Tier 2 — loose code (no manifest) → flagged
        let cpp = tmp.path().join("cpp");
        std::fs::create_dir_all(&cpp).unwrap();
        std::fs::write(cpp.join("main.cpp"), "int main(){}").unwrap();
        std::fs::write(cpp.join("util.h"), "#pragma once").unwrap();
        assert_eq!(classify_quasi_repo(&cpp), Some(QuasiKind::LooseCode));

        // Tier 2 — markdown docs folder → flagged (treated as a docs project)
        let docs = tmp.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("guide.md"), "# Guide").unwrap();
        assert_eq!(classify_quasi_repo(&docs), Some(QuasiKind::LooseCode));

        // Tier 3 — data only (csv/txt) → NOT a project
        let data = tmp.path().join("data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("rows.csv"), "a,b\n1,2\n").unwrap();
        std::fs::write(data.join("notes.txt"), "scratch").unwrap();
        assert_eq!(classify_quasi_repo(&data), None);

        // Tier 3 — empty → NOT a project
        let empty = tmp.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(classify_quasi_repo(&empty), None);
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
    fn subfolder_tree_includes_intermediates_parent_first() {
        let root = Path::new("/repo");
        let dirs = vec![
            PathBuf::from("/repo/src/api"),
            PathBuf::from("/repo/src/db"),
            PathBuf::from("/repo/tests"),
        ];
        let tree = subfolder_tree(root, &dirs);
        let names: Vec<String> = tree.iter().map(|(d, _)| d.to_string_lossy().to_string()).collect();

        // The intermediate /repo/src (no direct files) is included.
        assert!(names.contains(&"/repo/src".to_string()), "intermediate missing: {names:?}");
        // The repo root itself is never stored.
        assert!(!names.contains(&"/repo".to_string()));
        // Parent appears before its children.
        let pos = |s: &str| names.iter().position(|n| n == s).unwrap();
        assert!(pos("/repo/src") < pos("/repo/src/api"));
        assert!(pos("/repo/src") < pos("/repo/src/db"));
        // Parent links resolve to the immediate parent (root for top-level dirs).
        for (d, parent) in &tree {
            match d.to_string_lossy().as_ref() {
                "/repo/src" => assert_eq!(parent, Path::new("/repo")),
                "/repo/tests" => assert_eq!(parent, Path::new("/repo")),
                "/repo/src/api" => assert_eq!(parent, Path::new("/repo/src")),
                "/repo/src/db" => assert_eq!(parent, Path::new("/repo/src")),
                other => panic!("unexpected dir {other}"),
            }
        }
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
    fn incremental_plan_classifies_new_changed_unchanged_removed() {
        use std::collections::HashMap;
        let prior: HashMap<String, i64> = [
            ("src/a.rs".to_string(), 100i64),
            ("src/b.rs".to_string(), 200i64),
            ("src/gone.rs".to_string(), 300i64),
        ].into_iter().collect();
        let current = vec![
            ("src/a.rs".to_string(), 100),   // unchanged
            ("src/b.rs".to_string(), 250),   // mtime changed
            ("src/new.rs".to_string(), 400), // new
        ];
        let plan = incremental_plan(&current, &prior);
        assert!(plan.changed.contains("src/b.rs"), "mtime change → reindex");
        assert!(plan.changed.contains("src/new.rs"), "new file → index");
        assert!(!plan.changed.contains("src/a.rs"), "unchanged mtime → skip");
        assert_eq!(plan.removed, vec!["src/gone.rs".to_string()], "vanished file → removed");
    }

    #[test]
    fn incremental_plan_first_index_marks_everything_changed() {
        use std::collections::HashMap;
        let prior: HashMap<String, i64> = HashMap::new();
        let current = vec![("a.rs".to_string(), 1), ("b.rs".to_string(), 2)];
        let plan = incremental_plan(&current, &prior);
        assert_eq!(plan.changed.len(), 2, "empty prior → full index");
        assert!(plan.removed.is_empty());
    }

    #[test]
    fn detect_stack_dotnet() {
        // Globbed project file
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("WebApi.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\" />").unwrap();
        assert_eq!(detect_stack(tmp.path()), vec!["dotnet"]);
        // Solution file
        let tmp2 = tempfile::tempdir().unwrap();
        std::fs::write(tmp2.path().join("App.sln"), "Microsoft Visual Studio Solution File").unwrap();
        assert_eq!(detect_stack(tmp2.path()), vec!["dotnet"]);
        // global.json SDK pin
        let tmp3 = tempfile::tempdir().unwrap();
        std::fs::write(tmp3.path().join("global.json"), "{\"sdk\":{\"version\":\"8.0.0\"}}").unwrap();
        assert_eq!(detect_stack(tmp3.path()), vec!["dotnet"]);
        // A .NET project root is a confident (manifest) quasi-repo, not loose code
        assert_eq!(classify_quasi_repo(tmp.path()), Some(QuasiKind::Manifest));
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

    // ── Reconcile: stale-root classification ─────────────────────

    fn live(paths: &[&str]) -> std::collections::HashSet<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn stale_root_kept_when_rediscovered() {
        let roots = live(&["/dev/a", "/dev/b"]);
        assert_eq!(
            classify_stale_root(Path::new("/dev/a"), &roots, true, true),
            StaleAction::Keep
        );
    }

    #[test]
    fn stale_root_removed_when_path_gone() {
        let roots = live(&["/dev/a"]);
        // /dev/zombie not a live root, no longer on disk → remove
        assert_eq!(
            classify_stale_root(Path::new("/dev/zombie"), &roots, false, false),
            StaleAction::Remove
        );
    }

    #[test]
    fn stale_root_removed_when_empty_husk() {
        let roots = live(&["/dev/a"]);
        // exists on disk but holds no indexable content (moved-out husk) → remove
        assert_eq!(
            classify_stale_root(Path::new("/dev/husk"), &roots, true, false),
            StaleAction::Remove
        );
    }

    #[test]
    fn stale_root_removed_when_inside_a_live_root() {
        let roots = live(&["/dev/repo"]);
        // a former nested repo now owned by the live root above it — remove even
        // though it still has content (the live root re-materialises the subtree)
        assert_eq!(
            classify_stale_root(Path::new("/dev/repo/sub"), &roots, true, true),
            StaleAction::Remove
        );
    }

    #[test]
    fn stale_root_marked_when_content_but_no_owner() {
        let roots = live(&["/dev/a"]);
        // real content on disk, not a live root, not under any live root →
        // ambiguous, never auto-delete: mark stale for the user to decide
        assert_eq!(
            classify_stale_root(Path::new("/dev/archive"), &roots, true, true),
            StaleAction::MarkStale
        );
    }

    #[test]
    fn dir_has_indexable_content_distinguishes_husk_from_content() {
        let tmp = tempfile::tempdir().unwrap();

        let empty = tmp.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(!dir_has_indexable_content(&empty), "empty dir => no content");

        let binary = tmp.path().join("binary");
        std::fs::create_dir_all(&binary).unwrap();
        std::fs::write(binary.join("logo.png"), [0u8; 8]).unwrap();
        assert!(!dir_has_indexable_content(&binary), "binary-only => no content");

        let nested = tmp.path().join("nested");
        std::fs::create_dir_all(nested.join("src/api")).unwrap();
        std::fs::write(nested.join("src/api/handler.rs"), "fn h() {}").unwrap();
        assert!(dir_has_indexable_content(&nested), "source in a subdir => content");
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
