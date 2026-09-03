//! Pure scan logic — no DB, no task queue, no events.
//! These functions are called by the scan_root handler.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Directory names skipped during the scan walk: dependency/build output and
/// generated/OS junk that never contains first-party source. Kept in one place
/// so `walk_for_git` and `walk_dirs` agree.
const IGNORED_DIRS: &[&str] =
    &["node_modules", "dist", "build", "target", "__pycache__", "__MACOSX"];

/// How deep the scan walk descends from a watch root. Lifted from the old
/// hardcoded `3` (D15): a submodule / vendored checkout nested a few levels
/// inside a repo that itself sits a level or two under the watch root lands well
/// past depth 3. The walk stays affordable because `IGNORED_DIRS` and symlinks
/// are pruned and it never descends into `.git`; the bound is only a backstop
/// against a pathological tree.
pub const MAX_SCAN_DEPTH: u32 = 8;

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

/// True if `path` is a git **checkout** — it holds a `.git` that is either a
/// directory (a normal clone) OR a file (a linked worktree / submodule
/// "gitlink", whose `.git` is a text file pointing at the real git dir).
/// The single source of truth for "is this a repo root on disk", so the three
/// walk boundaries (`walk_for_git`, `is_inside_git_repo`, `walk_dirs`) agree and
/// none of them mask a worktree/submodule the way a `.is_dir()`-only test does.
pub fn is_checkout(path: &Path) -> bool {
    let g = path.join(".git");
    g.is_dir() || g.is_file()
}

/// Find all .git directories under root up to max_depth.
/// Returns parent directories of .git (the actual git folders).
pub fn find_git_folders(root: &Path, max_depth: u32) -> Vec<PathBuf> {
    let mut result = Vec::new();
    walk_for_git(root, 0, max_depth, &mut result);
    result.sort();
    result
}

/// True if `path` is at or under any exclusion. THE one owner of that rule —
/// [`crate::watcher::root_watcher::RootWatcher::should_watch_path`] calls this
/// rather than keeping its own copy.
///
/// TWO FORMS, because users write both and the watcher has always honoured both:
///
/// * **absolute** (`/Users/dev/Developer/Code`) — a subtree prefix.
/// * **bare / relative** (`Code`, `a/b/c`) — matches that run of path SEGMENTS
///   anywhere in the path, so a user can exclude `Code` without knowing where
///   the watch root sits.
///
/// Boundary-safe in both: `/a/Code` never matches `/a/Coder`, because the
/// comparison is segment-anchored (`/Code/`), never a raw substring.
///
/// This function previously supported ONLY the absolute form while the watcher
/// supported both. Both live exclusions were stored in the bare form, so the
/// watcher honoured them and the scanner ignored them — the exclusion "gated the
/// watcher while pruning nothing", the incident cited at `import_target.rs:85`
/// and `graph.rs:513`. It cost 289,258 vendored OpenSSL `#define` nodes, 40% of
/// the graph, indexed from a path that HAD been excluded.
pub fn is_excluded(path: &Path, exclusions: &[String]) -> bool {
    let p = path.to_string_lossy();
    exclusions.iter().any(|ex| {
        let ex = ex.trim_start_matches('/').trim_end_matches('/');
        if ex.is_empty() {
            return false;
        }
        // Absolute form: an anchored subtree prefix.
        if p == format!("/{ex}") || p.starts_with(&format!("/{ex}/")) {
            return true;
        }
        // Bare/relative form: the same segment run anywhere in the path. Wrapped
        // in separators on BOTH sides so `Code` cannot match `Coder`, and
        // additionally allowed to terminate the path (`…/Code`).
        p.contains(&format!("/{ex}/")) || p.ends_with(&format!("/{ex}"))
    })
}

fn walk_for_git(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip symlinks: a symlinked repo (e.g. `sensei-hq/gateway` →
        // `strategos/gateway`) is already reached via its real path, so following
        // the link would classify the same repo twice → two projects.
        if entry.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
            continue;
        }
        if !path.is_dir() {
            continue;
        }

        // Detect-before-prune (D15d): record a checkout even when its own
        // directory name is dotfile-prefixed or an IGNORED_DIRS build-output
        // name (a repo literally named `build`). `is_checkout` also matches a
        // `.git`-FILE worktree/submodule, not just a `.git` directory. Then
        // descend INTO it so a nested checkout (submodule / vendored clone)
        // surfaces as its own root — the walk no longer halts at the first
        // `.git` (the core D15 fix). Descent stays affordable: `.git` internals
        // are dotfile-skipped and IGNORED_DIRS/symlinks are pruned below.
        if is_checkout(&path) {
            out.push(path.clone());
            walk_for_git(&path, depth + 1, max_depth, out);
            continue;
        }

        // A non-checkout dir: prune generated / hidden dirs, else keep descending.
        if name.starts_with('.') || IGNORED_DIRS.contains(&name.as_str()) {
            continue;
        }
        walk_for_git(&path, depth + 1, max_depth, out);
    }
}

/// Compute the set of ancestor directories from git folders up to root.
pub fn ancestor_set(root: &Path, git_folders: &[PathBuf]) -> std::collections::HashSet<PathBuf> {
    let mut ancestors = std::collections::HashSet::new();
    for gf in git_folders {
        let mut current = gf.parent();
        while let Some(p) = current {
            if p == root {
                break;
            }
            ancestors.insert(p.to_path_buf());
            current = p.parent();
        }
    }
    ancestors
}

/// True when `dir` lives INSIDE a git repository — i.e. any ancestor strictly
/// above it holds a `.git` directory. Walks the real filesystem upward, so it
/// still detects an enclosing repo whose `.git` sits AT or ABOVE the scan root
/// (or beyond the scan's depth bound) — cases the `git_folders` set (only the
/// repos discovered *under* the scan root) misses. This is the invariant behind
/// Bug 3: a manifest-bearing sub-dir inside a git repo (e.g. a moved
/// `crates/*`) must attribute to that repo, never be promoted to its own
/// `standalone` project just because it carries a `Cargo.toml`.
pub fn is_inside_git_repo(dir: &Path) -> bool {
    let mut cur = dir.parent();
    while let Some(p) = cur {
        if is_checkout(p) {
            return true;
        }
        cur = p.parent();
    }
    false
}

/// Collect all non-ignored subdirectories under root (one level deep per directory, recursive).
pub fn all_directories(root: &Path, max_depth: u32) -> Vec<PathBuf> {
    let mut result = Vec::new();
    walk_dirs(root, 0, max_depth, &mut result);
    result.sort();
    result
}

fn walk_dirs(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !path.is_dir() || name.starts_with('.') {
            continue;
        }
        if IGNORED_DIRS.contains(&name.as_str()) {
            continue;
        }

        out.push(path.clone());
        // Don't recurse into a checkout — its contents are that repo's content,
        // never quasi-repo candidates (`.git`-FILE worktrees included).
        if !is_checkout(&path) {
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

    let mut result: Vec<DiscoveredFolder> = git_folders
        .iter()
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
    let mut candidates: Vec<&PathBuf> = all_dirs
        .iter()
        .filter(|d| !git_set.contains(*d)) // not a git repo itself
        .filter(|d| !ancestors.contains(*d)) // not a git-repo grouping container
        .filter(|d| !is_inside_git_repo(d)) // not inside ANY git repo (fs-checked, incl. a repo at/above the scan root)
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
            let parent =
                d.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| repo_path.to_path_buf());
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
///
/// Thin wrapper over `classifiers::file_classifier()` — the source-ext list
/// itself now lives in the classifier module so adding a new language is one
/// edit, not two.
pub fn is_project_source_ext(ext: &str) -> bool {
    crate::classifiers::file_classifier().is_source_file(ext)
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
///
/// Delegates each known manifest to its `ManifestAdapter.is_workspace_root`,
/// plus keeps two filesystem-marker fallbacks for workspace formats that live
/// outside a per-ecosystem manifest: `pnpm-workspace.yaml` (pnpm-only
/// workspaces without a root `package.json`) and `go.work` (Go multi-module
/// workspaces).
pub fn is_monorepo(path: &Path) -> bool {
    for adapter in crate::adapters::manifest::registered_adapters() {
        for filename in adapter.manifest_filenames() {
            let Ok(content) = std::fs::read_to_string(path.join(filename)) else { continue };
            if adapter.is_workspace_root(&content) {
                return true;
            }
        }
    }
    // Filesystem-marker workspaces without a per-ecosystem manifest.
    if path.join("pnpm-workspace.yaml").exists() {
        return true;
    }
    if path.join("go.work").exists() {
        return true;
    }
    false
}

/// The incremental re-index decision for a folder. Pure output of
/// [`plan_reindex`]: the two-tier gate that keeps a no-op / touch-only re-scan
/// near-free.
#[derive(Debug, Default, PartialEq)]
pub struct ReindexPlan {
    /// Files to (re)index: new, or whose mtime AND content changed.
    pub changed: std::collections::HashSet<String>,
    /// Files whose mtime drifted but whose bytes are identical (touch, checkout,
    /// branch-switch-to-same-content). Their nodes/embeddings are still valid,
    /// so we DON'T reindex — we only refresh the stored mtime so the cheap gate
    /// hits next pass. `(rel_path, new_mtime, content_hash)`.
    pub touched: Vec<(String, i64, String)>,
    /// Files indexed before but no longer present on disk — drop their nodes.
    pub removed: Vec<String>,
    /// Count of files the cheap mtime gate skipped (never read, never hashed).
    /// Surfaced for logging so a no-op scan can be shown to be stats-only.
    pub unchanged: usize,
}

/// Diff the working tree against the last index with a two-tier gate so a
/// no-op or touch-only re-scan is near-free (this is what makes a *frequent*
/// safety-net reconcile affordable):
///
///   1. **mtime gate (cheap, stat-only):** a file whose on-disk mtime equals
///      its stored mtime is UNCHANGED — never read, never hashed, never
///      reindexed. This is the common case on a no-op scan.
///   2. **content-hash gate:** a file whose mtime differs (or that has no prior)
///      is a *candidate*. A file with a prior fingerprint is hashed (via the
///      injected `hash_file`) and compared to its stored hash — identical ⇒
///      `touched` (refresh mtime only), different ⇒ `changed` (reindex). A
///      brand-new file goes straight to `changed` (nothing to compare, so it is
///      never hashed here).
///
/// `current` is the set of indexable files as `(rel_path, mtime_ms)` on disk
/// now; `prior` maps each previously-indexed `rel_path` to its
/// `(mtime, content_hash)`. `hash_file(rel_path) -> Option<hex>` performs the
/// only I/O — injecting it keeps this function pure and lets tests spy the
/// hash-call count. A candidate whose hash can't be computed (unreadable) is
/// treated as `changed` so it is never silently dropped.
pub fn plan_reindex<F>(
    current: &[(String, i64)],
    prior: &std::collections::HashMap<String, (i64, String)>,
    mut hash_file: F,
) -> ReindexPlan
where
    F: FnMut(&str) -> Option<String>,
{
    let current_set: std::collections::HashSet<&String> = current.iter().map(|(p, _)| p).collect();
    let mut plan = ReindexPlan::default();
    for (path, mtime) in current {
        match prior.get(path) {
            // Cheap mtime gate: unchanged → skip without any read/hash.
            Some((prev_mtime, _)) if prev_mtime == mtime => plan.unchanged += 1,
            // mtime drifted with a prior on record: hash to tell a real edit
            // from a mere touch.
            Some((_, prev_hash)) => match hash_file(path) {
                Some(h) if &h == prev_hash => plan.touched.push((path.clone(), *mtime, h)),
                _ => {
                    plan.changed.insert(path.clone());
                }
            },
            // Brand-new file: reindex (no prior to compare against, so no hash).
            None => {
                plan.changed.insert(path.clone());
            }
        }
    }
    plan.removed = prior.keys().filter(|path| !current_set.contains(*path)).cloned().collect();
    plan
}

/// Detect technology stack from config files in a git folder.
///
/// Iterates registered `ManifestAdapter`s to pick up stack labels for every
/// manifest present (`Cargo.toml` → "rust", `package.json` → svelte / react /
/// vue / nextjs / typescript by framework detection, `pyproject.toml` →
/// "python", `go.mod` → "go"). Filesystem-only stacks that don't yet have an
/// adapter — `requirements.txt`, `Package.swift`, `Gemfile`, .NET
/// solution/project files, `global.json` — stay as explicit filesystem
/// signals until their adapters land.
pub fn detect_stack(path: &Path) -> Vec<String> {
    let mut stack: Vec<String> = Vec::new();
    for adapter in crate::adapters::manifest::registered_adapters() {
        for filename in adapter.manifest_filenames() {
            let Ok(content) = std::fs::read_to_string(path.join(filename)) else { continue };
            for label in adapter.stack_labels(&content) {
                stack.push(label.to_string());
            }
        }
    }
    // Filesystem-only stacks (no ManifestAdapter yet).
    if path.join("requirements.txt").exists() && !stack.iter().any(|s| s == "python") {
        stack.push("python".into());
    }
    if path.join("Package.swift").exists() {
        stack.push("swift".into());
    }
    if path.join("Gemfile").exists() {
        stack.push("ruby".into());
    }
    // .NET — solution/project manifests use globbed names (Foo.sln, Bar.csproj),
    // so scan the directory rather than checking a fixed filename. A fixed
    // global.json (SDK pin) also marks a .NET root.
    if path.join("global.json").exists()
        || dir_has_ext(path, &["sln", "csproj", "fsproj", "vbproj"])
    {
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
/// lib) > docs. Delegates the per-ecosystem rules to `ManifestAdapter.infer_role`
/// and applies the cross-ecosystem precedence here so the same set of rules
/// governs `folder_role` regardless of which manifest(s) the folder carries.
/// Kept as a pure function so it stays unit-testable without a filesystem.
pub fn classify_role(
    cargo: Option<&str>,
    pkg: Option<&str>,
    has_lib_rs: bool,
    has_routes: bool,
    dir_name: &str,
) -> Option<&'static str> {
    use crate::adapters::manifest::{FsSignals, manifest_adapter_for_filename};

    let fs = FsSignals { has_lib_rs, has_src_routes: has_routes, dir_name: dir_name.to_string() };
    let cargo_adapter = manifest_adapter_for_filename("Cargo.toml");
    let npm_adapter = manifest_adapter_for_filename("package.json");

    let cargo_role = cargo.zip(cargo_adapter).and_then(|(content, adapter)| {
        let parsed = adapter.parse_manifest(content);
        adapter.infer_role(&parsed, content, &fs)
    });
    let npm_role = pkg.zip(npm_adapter).and_then(|(content, adapter)| {
        let parsed = adapter.parse_manifest(content);
        adapter.infer_role(&parsed, content, &fs)
    });

    // Cross-ecosystem precedence: tool > website > library. A folder with both
    // a Cargo.toml [[bin]] and a package.json library is a tool.
    for role in ["tool", "website", "library"] {
        if cargo_role == Some(role) || npm_role == Some(role) {
            return Some(role);
        }
    }

    // 4. Docs directory (fallback that has no manifest to speak of).
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
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let manifest_filenames = crate::adapters::manifest::all_manifest_filenames();
    let manifest_extensions = crate::adapters::manifest::all_manifest_extensions();
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || IGNORED_DIRS.contains(&name) {
            continue;
        }
        // Any registered manifest filename marks a sub-project boundary.
        // Extension-keyed manifests (.csproj / .fsproj / .sln) get a second
        // pass so .NET reactors are recognised alongside package.json / Cargo.toml.
        let has_manifest = manifest_filenames.iter().any(|m| p.join(m).exists())
            || (!manifest_extensions.is_empty() && dir_has_ext(&p, &manifest_extensions));
        if has_manifest {
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
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
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
        if !entry.path().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(path).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy();
        if exclude.is_match(&*rel_str) {
            continue;
        }

        let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext.is_empty() {
            continue;
        }
        if super::helpers::is_binary_ext(ext) {
            continue;
        }

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

/// The final fate of a stale root once the deletion-avoidance signals are folded
/// into [`classify_stale_root`]'s base verdict. Returned by [`decide_stale_root`].
#[derive(Debug, Clone, PartialEq)]
pub enum StaleDisposition {
    /// Re-discovered this scan — leave it.
    Keep,
    /// A live root shares this (now-gone) root's git remote: the repo was renamed
    /// or moved. Re-point its history to that folder (the payload) rather than
    /// deleting — the transcript mappings must not dangle.
    Remap(uuid::Uuid),
    /// Gone with no live twin, but it carries history (sessions/transcripts) worth
    /// keeping: retain it as `archived` instead of a hard delete.
    Archive,
    /// Provably dead AND history-free — safe to delete (cascading nodes + subtree).
    Remove,
    /// Still on disk with real content, no live owner — user triages.
    MarkStale,
}

/// Fold the two deletion-avoidance signals into [`classify_stale_root`]'s verdict.
/// Pure so the policy is unit-testable; the impure facts (`remote_match`,
/// `has_history`) are injected by [`super::scan::reconcile_roots`].
///
/// A remote match and history only ever *upgrade* a `Remove` (the path is gone):
/// - a live root with the same git remote ⇒ the repo moved ⇒ **remap** its history;
/// - else if it has sessions/transcripts ⇒ **archive** (retain) rather than delete;
/// - else ⇒ **remove** as before.
///
/// `Keep` and `MarkStale` are never overridden: `Keep` means it's still live, and
/// `MarkStale` means the old path still exists with content — a same-remote clone
/// there is a duplicate to triage, not a rename to auto-absorb.
pub fn decide_stale_root(
    base: StaleAction,
    remote_match: Option<uuid::Uuid>,
    has_history: bool,
) -> StaleDisposition {
    match base {
        StaleAction::Keep => StaleDisposition::Keep,
        StaleAction::MarkStale => StaleDisposition::MarkStale,
        StaleAction::Remove => match remote_match {
            Some(to) => StaleDisposition::Remap(to),
            None if has_history => StaleDisposition::Archive,
            None => StaleDisposition::Remove,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── role inference ───────────────────────────────────────────────────
    #[test]
    fn classify_role_tool_from_binary() {
        // Explicit Cargo [[bin]] → tool (dbd's cli crate, sensei cli/mcp).
        assert_eq!(
            classify_role(
                Some("[package]\nname=\"cli\"\n\n[[bin]]\nname=\"dbd\""),
                None,
                false,
                false,
                "cli"
            ),
            Some("tool")
        );
        // Node package that ships a CLI (`bin` field) (rokkit packages/cli) → tool.
        assert_eq!(
            classify_role(
                None,
                Some("{\"name\":\"c\",\"bin\":{\"c\":\"./c.js\"}}"),
                false,
                false,
                "c"
            ),
            Some("tool")
        );
        // A daemon binary (main.rs, NO [[bin]], server deps) is NOT a CLI tool —
        // it stays unclassified so a frontmatter role can label it backend.
        assert_eq!(
            classify_role(
                Some("[package]\nname=\"senseid\"\n\n[dependencies]\naxum = \"0.7\"\nclap = \"4\""),
                None,
                false,
                false,
                "senseid"
            ),
            None
        );
    }

    #[test]
    fn classify_role_website_from_web_framework() {
        // SvelteKit app (rokkit apps/learn, dbd site) → website.
        assert_eq!(
            classify_role(
                None,
                Some("{\"name\":\"learn\",\"devDependencies\":{\"@sveltejs/kit\":\"^2\"}}"),
                false,
                true,
                "learn"
            ),
            Some("website")
        );
        // Web-app markers win over the library marker (an app also has a name).
        assert_eq!(
            classify_role(
                None,
                Some(
                    "{\"name\":\"site\",\"type\":\"module\",\"devDependencies\":{\"@sveltejs/kit\":\"^2\"}}"
                ),
                false,
                true,
                "site"
            ),
            Some("website")
        );
    }

    #[test]
    fn classify_role_library_from_lib_crate_or_package() {
        // Rust lib crate (dbd's core crates) → library.
        assert_eq!(
            classify_role(Some("[package]\nname=\"core\""), None, true, false, "core"),
            Some("library")
        );
        // Publishable node package with exports (rokkit packages/*) → library.
        assert_eq!(
            classify_role(
                None,
                Some("{\"name\":\"@rokkit/ui\",\"exports\":{\".\":\"./index.js\"}}"),
                false,
                false,
                "ui"
            ),
            Some("library")
        );
        // A library that only lists @sveltejs/kit as a peer/dev dep (no src/routes)
        // is NOT a website (rokkit's unocss preset) → library.
        assert_eq!(
            classify_role(
                None,
                Some(
                    "{\"name\":\"@rokkit/unocss\",\"exports\":{\".\":\"./i.js\"},\"peerDependencies\":{\"@sveltejs/kit\":\"^2\"}}"
                ),
                false,
                false,
                "unocss"
            ),
            Some("library")
        );
        // `"type": "module"` alone must NOT read as a library entry point.
        assert_eq!(
            classify_role(None, Some("{\"name\":\"x\",\"type\":\"module\"}"), false, false, "x"),
            None
        );
    }

    #[test]
    fn classify_role_none_or_docs() {
        assert_eq!(classify_role(None, None, false, false, "misc"), None);
        // A private root manifest with no name/entry stays unclassified.
        assert_eq!(
            classify_role(
                None,
                Some("{\"private\":true,\"workspaces\":[\"packages/*\"]}"),
                false,
                false,
                "root"
            ),
            None
        );
        assert_eq!(classify_role(None, None, false, false, "docs"), Some("docs"));
    }

    #[test]
    fn infer_role_reads_manifest_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname=\"c\"\n[[bin]]\nname=\"c\"")
            .unwrap();
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
        let names: Vec<&str> =
            gits.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect();
        assert_eq!(names.len(), 4);
        assert!(names.contains(&"fldr_1"));
        assert!(names.contains(&"fldr_2"));
        assert!(names.contains(&"fldr_3"));
        assert!(names.contains(&"standalone"));
    }

    #[test]
    #[cfg(unix)]
    fn find_git_folders_dedupes_symlinked_repo() {
        // A repo reachable via two paths (a real dir + a symlink to it) must be
        // ONE folder, not two — else it double-counts as two projects (the
        // sensei-hq/gateway → strategos/gateway case).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("strategos/gateway/.git")).unwrap();
        std::os::unix::fs::symlink(root.join("strategos/gateway"), root.join("sensei-hq_gateway"))
            .unwrap();
        let gits = find_git_folders(root, 3);
        assert_eq!(gits.len(), 1, "symlinked repo counted once, got {gits:?}");
        assert!(
            gits[0].ends_with("strategos/gateway"),
            "canonicalized to the real path, got {gits:?}"
        );
    }

    // ── D15: checkout detection + nested/deep discovery ──────────────────
    #[test]
    fn is_checkout_true_for_git_dir_and_git_file() {
        let tmp = tempfile::tempdir().unwrap();
        // `.git` DIRECTORY → a normal checkout.
        let dir_repo = tmp.path().join("dir_repo");
        std::fs::create_dir_all(dir_repo.join(".git")).unwrap();
        assert!(is_checkout(&dir_repo), "a dir with a .git DIRECTORY is a checkout");
        // `.git` FILE (a linked worktree / submodule gitlink) → also a checkout.
        let file_repo = tmp.path().join("file_repo");
        std::fs::create_dir_all(&file_repo).unwrap();
        std::fs::write(file_repo.join(".git"), "gitdir: /main/.git/worktrees/wt\n").unwrap();
        assert!(is_checkout(&file_repo), "a dir with a .git FILE (gitlink) is a checkout");
        // No `.git` at all → not a checkout.
        let plain = tmp.path().join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        assert!(!is_checkout(&plain), "a dir with no .git is not a checkout");
    }

    #[test]
    fn walk_for_git_descends_into_nested_checkout() {
        // A repo with a nested checkout (vendored dep / submodule) must yield
        // BOTH the outer repo AND the nested one — the recursion no longer halts
        // at the first `.git` (the core D15 fix). Previously only `repo` was
        // discovered and `repo/vendor/lib` was masked as its content.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("repo/.git")).unwrap();
        std::fs::create_dir_all(root.join("repo/vendor/lib/.git")).unwrap();
        let gits = find_git_folders(root, MAX_SCAN_DEPTH);
        assert!(gits.iter().any(|p| p.ends_with("repo")), "outer repo discovered: {gits:?}");
        assert!(
            gits.iter().any(|p| p.ends_with("repo/vendor/lib")),
            "nested checkout discovered: {gits:?}"
        );
    }

    #[test]
    fn find_git_folders_discovers_git_file_worktree_and_submodule() {
        // A checkout whose `.git` is a FILE (a linked worktree or a submodule
        // gitlink) must be discovered as a root — previously only `.git`
        // DIRECTORIES were, so every worktree/submodule was invisible.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let wt = root.join("worktree");
        std::fs::create_dir_all(&wt).unwrap();
        std::fs::write(wt.join(".git"), "gitdir: /main/.git/worktrees/wt\n").unwrap();
        let gits = find_git_folders(root, MAX_SCAN_DEPTH);
        assert!(
            gits.iter().any(|p| p.ends_with("worktree")),
            "gitlink checkout discovered: {gits:?}"
        );
    }

    #[test]
    fn find_git_folders_finds_checkout_below_depth_3() {
        // A checkout deeper than the old hardcoded depth-3 bound must be found
        // now that MAX_SCAN_DEPTH lifts it.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("l1/l2/l3/l4/repo/.git")).unwrap();
        // The old bound (3) could not reach a depth-5 checkout — documents the limit.
        assert!(
            !find_git_folders(root, 3).iter().any(|p| p.ends_with("repo")),
            "depth-3 bound cannot reach a depth-5 checkout"
        );
        // The lifted bound reaches it.
        assert!(
            find_git_folders(root, MAX_SCAN_DEPTH).iter().any(|p| p.ends_with("repo")),
            "MAX_SCAN_DEPTH reaches the deep checkout"
        );
    }

    #[test]
    fn detect_before_prune_git_inside_ignored_name() {
        // A checkout whose OWN directory name collides with an ignored
        // build-output name (a repo literally named `build`) or is dotfile-
        // prefixed must still be detected — `is_checkout` runs BEFORE the
        // IGNORED_DIRS / dotfile skip (detect-before-prune, D15d).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("build/.git")).unwrap(); // ignored NAME, real repo
        std::fs::create_dir_all(root.join(".hidden_repo/.git")).unwrap(); // dotfile-prefixed repo
        let gits = find_git_folders(root, MAX_SCAN_DEPTH);
        assert!(
            gits.iter().any(|p| p.ends_with("build")),
            "checkout named `build` detected: {gits:?}"
        );
        assert!(
            gits.iter().any(|p| p.ends_with(".hidden_repo")),
            "dotfile-prefixed checkout detected: {gits:?}"
        );
    }

    #[test]
    fn all_directories_does_not_descend_into_git_file_checkout() {
        // `walk_dirs` must treat a `.git`-FILE checkout as a git boundary just
        // like a `.git`-DIR one — otherwise a worktree's internal dirs leak into
        // the quasi-repo candidate set.
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let wt = root.join("worktree");
        std::fs::create_dir_all(wt.join("src")).unwrap();
        std::fs::write(wt.join(".git"), "gitdir: /main/.git/worktrees/wt\n").unwrap();
        let dirs = all_directories(root, MAX_SCAN_DEPTH);
        assert!(dirs.iter().any(|d| d.ends_with("worktree")), "the checkout dir itself is listed");
        assert!(
            !dirs.iter().any(|d| d.ends_with("worktree/src")),
            "must NOT descend into a .git-FILE checkout (its src is repo content, not a candidate root)"
        );
    }

    #[test]
    fn is_inside_git_repo_detects_enclosing_git_file_worktree() {
        // An enclosing repo whose `.git` is a FILE (worktree/submodule) must
        // count as a git ancestor, so a manifest sub-dir inside it is never
        // promoted to its own standalone project.
        let tmp = tempfile::tempdir().unwrap();
        let wt = tmp.path().join("worktree");
        std::fs::create_dir_all(wt.join("crates/x")).unwrap();
        std::fs::write(wt.join(".git"), "gitdir: /main/.git/worktrees/wt\n").unwrap();
        assert!(
            is_inside_git_repo(&wt.join("crates/x")),
            "a dir inside a .git-FILE worktree is inside a git repo"
        );
    }

    /// Both exclusion FORMS, because the two callers supply different ones.
    ///
    /// `scan.rs` passes prefixes already resolved to absolute by
    /// `root_exclusion_prefixes` (`root + entry`), while `workspace.rs` hands the
    /// watcher the RAW relative entries straight off the request. That is why
    /// `RootWatcher::should_watch_path` had grown its own two-form matcher — and
    /// why it and this function could disagree. One matcher, both forms, so they
    /// cannot drift apart again.
    ///
    /// NOTE ON PROVENANCE: an earlier version of this comment blamed the missing
    /// bare-name arm for 289k vendored OpenSSL nodes being indexed. That was
    /// wrong — those got in because the stored exclusion entry was missing a
    /// `pre-sales/` path segment, so the RESOLVED absolute prefix pointed at a
    /// directory that does not exist. This function is hardening; it was not the
    /// bug. Kept because the two callers genuinely do supply different forms.
    ///
    /// Breaking mutation: drop the bare-name arm — the segment cases below fail.
    #[test]
    fn a_bare_name_exclusion_matches_a_path_segment_like_the_watcher_does() {
        let ex = vec!["Code".to_string()];
        assert!(is_excluded(Path::new("/Users/dev/Developer/Code/repo"), &ex));
        assert!(is_excluded(Path::new("/Users/dev/Developer/Code"), &ex), "the segment itself");
        // Boundary-safe: a bare name must not match a longer sibling segment.
        assert!(!is_excluded(Path::new("/Users/dev/Developer/Coder/repo"), &ex));
        assert!(!is_excluded(Path::new("/Users/dev/Codebase/repo"), &ex));

        // A multi-segment relative run, the second live exclusion's shape.
        let ex = vec!["find-me-board/docs/proposal/deck-node".to_string()];
        assert!(is_excluded(
            Path::new(
                "/Users/dev/Work/pre-sales/find-me-board/docs/proposal/deck-node/include/a.h"
            ),
            &ex,
        ));
        assert!(!is_excluded(Path::new("/Users/dev/Work/pre-sales/find-me-board/src/a.ts"), &ex));
    }

    #[test]
    fn is_excluded_matches_prefix_and_self_but_not_siblings() {
        let ex = vec!["/Users/dev/Developer/Code".to_string(), "/tmp/junk/".to_string()];
        // The prefix itself and anything under it are excluded.
        assert!(is_excluded(Path::new("/Users/dev/Developer/Code"), &ex));
        assert!(is_excluded(Path::new("/Users/dev/Developer/Code/archive/repo"), &ex));
        // Trailing slash in the exclusion is normalized.
        assert!(is_excluded(Path::new("/tmp/junk/repo"), &ex));
        // Boundary-safe: a sibling that merely shares the prefix string is NOT excluded.
        assert!(!is_excluded(Path::new("/Users/dev/Developer/Coder"), &ex));
        assert!(!is_excluded(Path::new("/Users/dev/Developer/Other"), &ex));
        // Empty exclusion list excludes nothing.
        assert!(!is_excluded(Path::new("/anything"), &[]));
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

        let git_names: Vec<&str> = classified
            .iter()
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

        let quasi: Vec<&str> = classified
            .iter()
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
    fn is_inside_git_repo_detects_enclosing_repo_at_or_above() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        let crate_dir = repo.join("crates/mycrate");
        std::fs::create_dir_all(&crate_dir).unwrap();
        // A dir inside the repo (repo's .git is an ancestor) → inside a git repo.
        assert!(is_inside_git_repo(&crate_dir));
        assert!(is_inside_git_repo(&repo.join("crates")));
        // A sibling of the repo (no .git ancestor) → not inside a git repo.
        let outside = tmp.path().join("loose");
        std::fs::create_dir_all(&outside).unwrap();
        assert!(!is_inside_git_repo(&outside));
    }

    #[test]
    fn classify_does_not_promote_manifest_subdir_inside_a_git_repo() {
        // Bug 3: when the scan is rooted AT a git repo (its own `.git` sits at the
        // scan root, so `find_git_folders` — which starts at children — never
        // discovers it), a manifest-bearing sub-crate must NOT be promoted to its
        // own standalone project. It belongs to the enclosing repo's project.
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("crates/mycrate/src")).unwrap();
        std::fs::write(repo.join("crates/mycrate/Cargo.toml"), "[package]\nname=\"mycrate\"")
            .unwrap();
        std::fs::write(repo.join("crates/mycrate/src/lib.rs"), "pub fn a() {}").unwrap();

        // Scan rooted at the repo itself → its own `.git` is not among git_folders.
        let gits = find_git_folders(&repo, 3);
        assert!(
            gits.is_empty(),
            "repo's own .git at the scan root is not discovered as a child git folder"
        );
        let dirs = all_directories(&repo, 3);
        let classified = classify_folders(&repo, &gits, &dirs, has_indexable_code);

        // The manifest-bearing sub-crate must NOT become a standalone project root.
        assert!(
            !classified.iter().any(|f| f.kind == FolderKind::Standalone),
            "a Cargo.toml sub-dir inside a git repo must not be promoted to standalone, got {classified:?}",
        );
    }

    #[test]
    fn classify_does_not_promote_member_when_git_repo_is_a_child_of_scan_root() {
        // #101 regression (the LIVE shape): scan rooted ABOVE the repo (e.g.
        // ~/Developer), the git repo is a child (~/Developer/repo/.git), and a
        // workspace member (crates/mycrate) sits inside it. `find_git_folders`
        // DOES discover the child repo, so the member must be excluded as content
        // of that repo — never promoted to its own Standalone root (which is what
        // produced the 2026-07-13 double-owner residue). Genuine sibling repos and
        // standalone projects OUTSIDE the git repo are still discovered.
        let tmp = tempfile::tempdir().unwrap();
        let scan_root = tmp.path();
        let repo = scan_root.join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("crates/mycrate/src")).unwrap();
        std::fs::write(repo.join("crates/mycrate/Cargo.toml"), "[package]\nname=\"mycrate\"")
            .unwrap();
        std::fs::write(repo.join("crates/mycrate/src/lib.rs"), "pub fn a() {}").unwrap();
        // A real standalone project OUTSIDE the repo — must still be discovered.
        // Manifest sits directly in `loose/` (has_indexable_code checks direct files).
        std::fs::create_dir_all(scan_root.join("loose")).unwrap();
        std::fs::write(scan_root.join("loose/go.mod"), "module loose").unwrap();

        let gits = find_git_folders(scan_root, 3);
        assert!(gits.iter().any(|g| g == &repo), "the child git repo is discovered");
        let dirs = all_directories(scan_root, 3);
        let classified = classify_folders(scan_root, &gits, &dirs, has_indexable_code);

        // The member inside the git repo is NOT a project root of any kind.
        assert!(
            !classified.iter().any(|f| f.path == repo.join("crates/mycrate")),
            "a workspace member inside a git repo must not be classified as a root; got {classified:?}",
        );
        // The enclosing repo IS a Git root; the outside loose project IS Standalone.
        assert!(classified.iter().any(|f| f.path == repo && f.kind == FolderKind::Git));
        assert!(
            classified
                .iter()
                .any(|f| f.path == scan_root.join("loose") && f.kind == FolderKind::Standalone)
        );
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
        let names: Vec<String> =
            tree.iter().map(|(d, _)| d.to_string_lossy().to_string()).collect();

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
        std::fs::write(tmp.path().join("package.json"), r#"{"dependencies":{"svelte":"^5"}}"#)
            .unwrap();
        assert_eq!(detect_stack(tmp.path()), vec!["svelte"]);
    }

    /// Helper: build a prior map of `rel -> (mtime, hash)`.
    fn prior_of(entries: &[(&str, i64, &str)]) -> std::collections::HashMap<String, (i64, String)> {
        entries.iter().map(|(p, m, h)| (p.to_string(), (*m, h.to_string()))).collect()
    }

    /// The cheap mtime gate must skip an unchanged file WITHOUT ever hashing it
    /// (proven by the spy counter staying 0) — this is what keeps a no-op scan
    /// stats-only.
    #[test]
    fn plan_reindex_mtime_gate_skips_unchanged_without_hashing() {
        let prior = prior_of(&[("a.rs", 100, "hash_a")]);
        let current = vec![("a.rs".to_string(), 100i64)];
        let mut hash_calls = 0usize;
        let plan = plan_reindex(&current, &prior, |_p| {
            hash_calls += 1;
            Some("x".into())
        });
        assert_eq!(hash_calls, 0, "an unchanged-mtime file must never be hashed");
        assert_eq!(plan.unchanged, 1);
        assert!(plan.changed.is_empty());
        assert!(plan.touched.is_empty());
        assert!(plan.removed.is_empty());
    }

    /// mtime drifted but content is byte-identical (a `touch`): the file is
    /// hashed exactly once, lands in `touched` (so its mtime is refreshed), and
    /// is NOT reindexed — no duplicate work.
    #[test]
    fn plan_reindex_touched_rehashes_but_does_not_reindex() {
        let prior = prior_of(&[("a.rs", 100, "same_hash")]);
        let current = vec![("a.rs".to_string(), 999i64)]; // mtime changed
        let mut hash_calls = 0usize;
        let plan = plan_reindex(&current, &prior, |_p| {
            hash_calls += 1;
            Some("same_hash".into())
        });
        assert_eq!(hash_calls, 1, "a touched candidate is hashed once to confirm identity");
        assert!(plan.changed.is_empty(), "identical content must NOT reindex");
        assert_eq!(
            plan.touched,
            vec![("a.rs".to_string(), 999i64, "same_hash".to_string())],
            "touched file carries its NEW mtime so the gate hits next pass"
        );
    }

    /// mtime drifted AND content changed: hashed once, lands in `changed`.
    #[test]
    fn plan_reindex_reindexes_genuine_change() {
        let prior = prior_of(&[("a.rs", 100, "old_hash")]);
        let current = vec![("a.rs".to_string(), 200i64)];
        let mut hash_calls = 0usize;
        let plan = plan_reindex(&current, &prior, |_p| {
            hash_calls += 1;
            Some("new_hash".into())
        });
        assert_eq!(hash_calls, 1);
        assert!(plan.changed.contains("a.rs"), "changed content → reindex");
        assert!(plan.touched.is_empty());
    }

    /// A brand-new file (no prior fingerprint) is reindexed WITHOUT hashing —
    /// there is nothing to compare it against.
    #[test]
    fn plan_reindex_new_file_reindexed_without_hashing() {
        let prior = prior_of(&[]);
        let current = vec![("new.rs".to_string(), 400i64)];
        let mut hash_calls = 0usize;
        let plan = plan_reindex(&current, &prior, |_p| {
            hash_calls += 1;
            Some("x".into())
        });
        assert_eq!(hash_calls, 0, "a new file needs no hash comparison");
        assert!(plan.changed.contains("new.rs"));
    }

    /// A file that vanished on disk is `removed`; an unreadable candidate
    /// (hash_file → None) falls through to `changed` rather than being dropped.
    #[test]
    fn plan_reindex_removed_and_unreadable_candidate() {
        let prior = prior_of(&[("gone.rs", 300, "h"), ("bad.rs", 100, "h")]);
        let current = vec![("bad.rs".to_string(), 200i64)]; // mtime drifted, unreadable
        let plan = plan_reindex(&current, &prior, |_p| None);
        assert_eq!(plan.removed, vec!["gone.rs".to_string()], "vanished file → removed");
        assert!(
            plan.changed.contains("bad.rs"),
            "unreadable candidate must not be silently dropped"
        );
    }

    /// Full mixed working tree exercised end-to-end through one call.
    #[test]
    fn plan_reindex_classifies_new_changed_touched_unchanged_removed() {
        let prior = prior_of(&[
            ("src/a.rs", 100, "ha"),    // unchanged
            ("src/b.rs", 200, "hb"),    // will reindex
            ("src/t.rs", 300, "ht"),    // will be touched (same content)
            ("src/gone.rs", 400, "hg"), // removed
        ]);
        let current = vec![
            ("src/a.rs".to_string(), 100),   // mtime unchanged
            ("src/b.rs".to_string(), 250),   // mtime + content changed
            ("src/t.rs".to_string(), 350),   // mtime changed, content same
            ("src/new.rs".to_string(), 500), // new
        ];
        let plan = plan_reindex(&current, &prior, |p| match p {
            "src/b.rs" => Some("hb_new".into()),
            "src/t.rs" => Some("ht".into()), // identical → touched
            other => panic!("unexpected hash of {other} (a.rs/new.rs must not be hashed)"),
        });
        assert_eq!(plan.unchanged, 1);
        assert!(plan.changed.contains("src/b.rs"));
        assert!(plan.changed.contains("src/new.rs"));
        assert!(!plan.changed.contains("src/a.rs"));
        assert_eq!(plan.touched.len(), 1);
        assert_eq!(plan.touched[0].0, "src/t.rs");
        assert_eq!(plan.removed, vec!["src/gone.rs".to_string()]);
    }

    #[test]
    fn detect_stack_dotnet() {
        // Globbed project file
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("WebApi.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\" />")
            .unwrap();
        assert_eq!(detect_stack(tmp.path()), vec!["dotnet"]);
        // Solution file
        let tmp2 = tempfile::tempdir().unwrap();
        std::fs::write(tmp2.path().join("App.sln"), "Microsoft Visual Studio Solution File")
            .unwrap();
        assert_eq!(detect_stack(tmp2.path()), vec!["dotnet"]);
        // global.json SDK pin
        let tmp3 = tempfile::tempdir().unwrap();
        std::fs::write(tmp3.path().join("global.json"), "{\"sdk\":{\"version\":\"8.0.0\"}}")
            .unwrap();
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
    fn detect_stack_python_from_requirements_txt_only() {
        // requirements.txt (no pyproject.toml) still marks python via the
        // filesystem-signal fallback.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("requirements.txt"), "flask\n").unwrap();
        assert_eq!(detect_stack(tmp.path()), vec!["python"]);
    }

    #[test]
    fn detect_stack_does_not_duplicate_python_when_pyproject_and_requirements_both_present() {
        // pyproject adapter already contributes "python"; requirements.txt
        // fallback must not double it.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("pyproject.toml"), "[project]").unwrap();
        std::fs::write(tmp.path().join("requirements.txt"), "flask\n").unwrap();
        let stack = detect_stack(tmp.path());
        assert_eq!(stack.iter().filter(|s| *s == "python").count(), 1);
    }

    #[test]
    fn is_monorepo_detects_pnpm_workspace_yaml_alone() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("pnpm-workspace.yaml"), "packages:\n  - 'apps/*'\n")
            .unwrap();
        assert!(is_monorepo(tmp.path()));
    }

    #[test]
    fn is_monorepo_detects_go_work_alone() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("go.work"), "go 1.21\n\nuse ( ./sub )\n").unwrap();
        assert!(is_monorepo(tmp.path()));
    }

    #[test]
    fn is_monorepo_detects_npm_workspaces_from_package_json() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"mono","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        assert!(is_monorepo(tmp.path()));
    }

    #[test]
    fn find_subprojects_finds_go_modules() {
        // Adapter-aware child manifest lookup: a go.mod in a nested folder
        // must now count as a sub-project boundary (previously only Cargo.toml
        // and package.json did).
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("go.work"), "go 1.21\nuse ( ./svc )\n").unwrap();
        std::fs::create_dir_all(root.join("svc")).unwrap();
        std::fs::write(root.join("svc/go.mod"), "module x\n").unwrap();
        let subs = find_subprojects(root, 3);
        assert!(
            subs.iter().any(|p| p.ends_with("svc")),
            "go.mod folder should be a sub-project: {subs:?}"
        );
    }

    #[test]
    fn find_subprojects_finds_pyproject_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("py_pkg")).unwrap();
        std::fs::write(root.join("py_pkg/pyproject.toml"), "[project]\nname=\"p\"\n").unwrap();
        let subs = find_subprojects(root, 3);
        assert!(
            subs.iter().any(|p| p.ends_with("py_pkg")),
            "pyproject.toml folder should be a sub-project: {subs:?}"
        );
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
        assert_eq!(classify_stale_root(Path::new("/dev/a"), &roots, true, true), StaleAction::Keep);
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

    // ── decide_stale_root (deletion-avoidance policy) ────────────────────
    #[test]
    fn decide_remaps_a_gone_root_whose_remote_lives_elsewhere() {
        // Remove base + a live root with the same git remote = a rename/move.
        let to = uuid::Uuid::from_u128(0x42);
        assert_eq!(
            decide_stale_root(StaleAction::Remove, Some(to), false),
            StaleDisposition::Remap(to),
            "a moved repo re-points its history, never deletes"
        );
        // remote match wins even when it ALSO has history (remap subsumes archive).
        assert_eq!(
            decide_stale_root(StaleAction::Remove, Some(to), true),
            StaleDisposition::Remap(to)
        );
    }

    #[test]
    fn decide_archives_a_gone_history_bearing_root_with_no_twin() {
        assert_eq!(
            decide_stale_root(StaleAction::Remove, None, true),
            StaleDisposition::Archive,
            "gone but carries sessions/transcripts → retain as archived, don't hard-delete"
        );
    }

    #[test]
    fn decide_removes_a_gone_history_free_root() {
        assert_eq!(
            decide_stale_root(StaleAction::Remove, None, false),
            StaleDisposition::Remove,
            "gone with nothing worth keeping → the existing hard-delete path"
        );
    }

    #[test]
    fn decide_never_overrides_keep_or_markstale() {
        let to = uuid::Uuid::from_u128(0x7);
        // Keep = still live; a remote match cannot demote it.
        assert_eq!(decide_stale_root(StaleAction::Keep, Some(to), true), StaleDisposition::Keep);
        // MarkStale = old path still exists with content; a same-remote clone there
        // is a duplicate to triage, not a rename to absorb.
        assert_eq!(
            decide_stale_root(StaleAction::MarkStale, Some(to), true),
            StaleDisposition::MarkStale
        );
        assert_eq!(
            decide_stale_root(StaleAction::MarkStale, None, false),
            StaleDisposition::MarkStale
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
