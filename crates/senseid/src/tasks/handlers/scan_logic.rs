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
    let p = fold_case(&path.to_string_lossy());
    exclusions.iter().any(|ex| {
        let ex = fold_case(ex.trim_start_matches('/').trim_end_matches('/'));
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

/// Normalise a path for comparison on a filesystem where case does not
/// distinguish two paths.
///
/// AN EXCLUSION IS A PRIVACY BOUNDARY, so this compares the way the FILESYSTEM
/// does. macOS APFS and Windows NTFS are case-insensitive by default —
/// `~/dev/Archive` and `~/dev/archive` are one directory — so a byte-exact
/// comparison fails OPEN there: the user stores `archive`, the walker reports
/// `Archive`, nothing matches, and the subtree they asked us never to read is
/// read into the database anyway.
///
/// Left exact on Linux, where the two really are different directories and
/// folding would exclude a subtree the user never named.
fn fold_case(s: &str) -> String {
    if cfg!(any(target_os = "macos", target_os = "windows")) {
        s.to_lowercase()
    } else {
        s.to_string()
    }
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

/// One registered folder's inputs for symlink de-duplication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderPathIdentity {
    pub id: uuid::Uuid,
    pub abs_path: String,
    /// `abs_path` with symlinks resolved. Equals `abs_path` when already real.
    pub real_path: String,
    pub repository_id: Option<uuid::Uuid>,
}

/// Folders that are THE SAME DIRECTORY reached by two paths share one repository.
///
/// Returns the `(folder_id, repository_id)` links to apply.
///
/// This is NOT the abs_path fallback `assign_repositories` deliberately refuses.
/// That refusal is about INVENTING an identity for a repo that has none — a
/// local-only checkout gets `NULL`, never a path-derived key. Here the identity
/// already exists and belongs to a folder that is provably the same directory:
/// `~/Developer/sensei-hq/gateway` is a symlink to `~/Developer/gateway`, one
/// inode tree reached by two paths. Recognising that is not a guess.
///
/// Measured: that pair shared 4,543 fqns — the largest duplicate pair in the
/// graph and the whole of the rust duplicate population. `assign_repositories`
/// could not link them because the symlinked path is classified `standalone`
/// with no remote, so it was skipped entirely.
///
/// A group whose members disagree about their repository is left ALONE rather
/// than resolved arbitrarily — two identities for one directory is a different
/// defect, and picking a winner here would hide it.
pub fn symlink_repository_links(folders: &[FolderPathIdentity]) -> Vec<(uuid::Uuid, uuid::Uuid)> {
    let mut by_real: std::collections::HashMap<&str, Vec<&FolderPathIdentity>> =
        std::collections::HashMap::new();
    for f in folders {
        by_real.entry(f.real_path.as_str()).or_default().push(f);
    }

    let mut links = Vec::new();
    for (_, group) in by_real {
        if group.len() < 2 {
            continue;
        }
        // Exactly ONE identity may be present, or this is the two-repositories
        // -one-directory defect and nothing here can choose correctly.
        let mut claimed: Vec<uuid::Uuid> =
            group.iter().filter_map(|f| f.repository_id).collect::<Vec<_>>();
        claimed.sort();
        claimed.dedup();
        let [repo] = claimed.as_slice() else { continue };

        for f in group.iter().filter(|f| f.repository_id.is_none()) {
            links.push((f.id, *repo));
        }
    }
    // Deterministic, so a re-run writes the same rows in the same order.
    links.sort();
    links
}

/// Does this FILE pass the scan's file-level filters?
///
/// ONE definition, because there were three. `count_indexable_files`,
/// `dir_has_indexable_content` and the `process_git_folder` walk each re-derived
/// it, and the walk's copy differed in a way nobody could see from reading it:
/// it applied the exclude glob only to decide whether the file's PARENT
/// DIRECTORY was discoverable, never to the file itself. So an excluded file
/// sitting beside an included one was indexed anyway, and the count and the
/// index disagreed about the same tree — 13,523 `.spec.ts`/`.test.ts` files in
/// the graph against an exclude list that named them.
///
/// Same shape as every other defect in this area: two paths answering one
/// question, drifting silently.
pub fn file_passes_scan_filters(rel: &str, ext: &str, exclude: &globset::GlobSet) -> bool {
    !ext.is_empty() && !super::helpers::is_binary_ext(ext) && !exclude.is_match(rel)
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

    /// A SYMLINKED checkout adopts the repository of the directory it points at.
    ///
    /// `~/Developer/sensei-hq/gateway` is a symlink to `~/Developer/gateway` —
    /// one inode tree reached by two paths, both registered, both indexed. They
    /// shared 4,543 fqns, the largest duplicate pair in the graph and the entire
    /// rust duplicate population. `assign_repositories` skipped the symlinked
    /// path because it is classified `standalone` with no git remote, so nothing
    /// linked the two and the UI had no way to show them as one repository.
    ///
    /// Breaking mutation: return `Vec::new()` — the symlink keeps
    /// `repository_id = NULL` and stays an invisible second copy.
    #[test]
    fn a_symlinked_checkout_adopts_the_repository_it_points_at() {
        let repo = uuid::Uuid::new_v4();
        let real = uuid::Uuid::new_v4();
        let link = uuid::Uuid::new_v4();

        let links = symlink_repository_links(&[
            FolderPathIdentity {
                id: real,
                abs_path: "/w/gateway".into(),
                real_path: "/w/gateway".into(),
                repository_id: Some(repo),
            },
            FolderPathIdentity {
                id: link,
                abs_path: "/w/org/gateway".into(),
                real_path: "/w/gateway".into(),
                repository_id: None,
            },
        ]);

        assert_eq!(links, vec![(link, repo)], "the symlink adopts the real dir's repository");
    }

    /// A folder that is genuinely its own directory is left alone, and a group
    /// that already agrees needs no write.
    #[test]
    fn distinct_directories_and_settled_groups_produce_no_links() {
        let repo_a = uuid::Uuid::new_v4();
        let repo_b = uuid::Uuid::new_v4();

        // Two real, different directories.
        let links = symlink_repository_links(&[
            FolderPathIdentity {
                id: uuid::Uuid::new_v4(),
                abs_path: "/w/a".into(),
                real_path: "/w/a".into(),
                repository_id: Some(repo_a),
            },
            FolderPathIdentity {
                id: uuid::Uuid::new_v4(),
                abs_path: "/w/b".into(),
                real_path: "/w/b".into(),
                repository_id: Some(repo_b),
            },
        ]);
        assert!(links.is_empty(), "different directories are different repositories");

        // Same directory, already linked — nothing to write.
        let settled = symlink_repository_links(&[
            FolderPathIdentity {
                id: uuid::Uuid::new_v4(),
                abs_path: "/w/gateway".into(),
                real_path: "/w/gateway".into(),
                repository_id: Some(repo_a),
            },
            FolderPathIdentity {
                id: uuid::Uuid::new_v4(),
                abs_path: "/w/org/gateway".into(),
                real_path: "/w/gateway".into(),
                repository_id: Some(repo_a),
            },
        ]);
        assert!(settled.is_empty(), "an already-linked group is idempotent");
    }

    /// Two DIFFERENT repositories claiming one directory is a separate defect.
    /// Picking a winner here would hide it, so the group is left untouched.
    #[test]
    fn a_directory_claimed_by_two_repositories_is_left_alone() {
        let repo_a = uuid::Uuid::new_v4();
        let repo_b = uuid::Uuid::new_v4();
        let orphan = uuid::Uuid::new_v4();

        let links = symlink_repository_links(&[
            FolderPathIdentity {
                id: uuid::Uuid::new_v4(),
                abs_path: "/w/one".into(),
                real_path: "/w/real".into(),
                repository_id: Some(repo_a),
            },
            FolderPathIdentity {
                id: uuid::Uuid::new_v4(),
                abs_path: "/w/two".into(),
                real_path: "/w/real".into(),
                repository_id: Some(repo_b),
            },
            FolderPathIdentity {
                id: orphan,
                abs_path: "/w/three".into(),
                real_path: "/w/real".into(),
                repository_id: None,
            },
        ]);
        assert!(links.is_empty(), "conflicting identities must not be resolved by guessing");
    }

    /// The graph is for CODE, TESTS and DOCS — not build output.
    ///
    /// Two halves, both of which were wrong:
    ///
    /// 1. TESTS BELONG. `*.spec.ts`, `*.test.ts`, `*_test.py`, `*_test.go` and
    ///    `*_test.rs` were on the exclude list, which contradicts what the graph
    ///    is for. They were also INERT on the indexing path (see
    ///    `file_passes_scan_filters`), so 13,523 spec/test files were indexed
    ///    while `count_indexable_files` reported them excluded — the count and
    ///    the index disagreed about the same tree.
    ///
    /// 2. BUILD OUTPUT DOES NOT BELONG, and the list never named it. Measured in
    ///    the live graph: `documentation/artifacts/erd/assets/index-Cgv8QKbu.js`,
    ///    a Vite bundle, produced 540 call references to single-letter minified
    ///    identifiers (`t`, `u`, `a`), and the same file vendored at a second
    ///    path produced 540 more. It escaped every glob because it lives under
    ///    `artifacts/`, not `dist/` or `build/`.
    ///
    /// Breaking mutation: put `**/*.spec.ts` back, or drop `**/*.min.js` — one
    /// half of this test fails immediately.
    #[test]
    fn tests_are_indexable_and_build_output_is_not() {
        let gs = super::super::helpers::build_globset();
        // Mirror `Path::extension()` — a name with no dot has NO extension, and
        // a plain `rsplit('.')` would hand back the whole file name instead.
        let idx = |rel: &str| {
            let ext = Path::new(rel).extension().and_then(|e| e.to_str()).unwrap_or("");
            file_passes_scan_filters(rel, ext, gs)
        };

        // CODE and TESTS are part of the structure being described.
        assert!(idx("src/lib/widget.ts"), "source is indexable");
        assert!(idx("src/lib/widget.spec.ts"), "a spec IS part of the codebase");
        assert!(idx("src/lib/widget.test.ts"), "a test IS part of the codebase");
        assert!(idx("app/e2e/flow.spec.ts"), "an e2e spec too");
        assert!(idx("pkg/thing_test.py"), "a python test too");
        assert!(idx("crates/x/src/thing.rs"), "rust source");

        // BUILD OUTPUT is not.
        assert!(!idx("documentation/artifacts/erd/assets/index-Cgv8QKbu.js"), "a Vite bundle");
        assert!(!idx("web/static/app.min.js"), "minified js");
        assert!(!idx("web/static/app.min.css"), "minified css");
        assert!(!idx("web/static/app.bundle.js"), "an explicit bundle");
        assert!(!idx("web/static/app.js.map"), "a source map");
        assert!(!idx("node_modules/left-pad/index.js"), "dependencies stay excluded");
        assert!(!idx("target/debug/thing.rs"), "build trees stay excluded");

        // Binary and extension-less files are still rejected.
        assert!(!idx("docs/diagram.png"), "binary");
        assert!(!idx("Makefile"), "no extension");
    }

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

    /// AN EXCLUSION IS A PRIVACY BOUNDARY, and on a case-insensitive
    /// filesystem a case mismatch must not silently defeat it.
    ///
    /// macOS APFS and Windows NTFS are case-insensitive by default, so
    /// `~/dev/Archive` and `~/dev/archive` are the SAME directory. A byte-exact
    /// comparison between the stored exclusion and the path the walker reports
    /// therefore fails open: the user believes the subtree is excluded, and its
    /// file contents are read into the database anyway.
    #[test]
    fn an_exclusion_matches_regardless_of_case_on_a_case_insensitive_filesystem() {
        if !cfg!(any(target_os = "macos", target_os = "windows")) {
            return; // case-sensitive filesystem: the two really are different dirs
        }
        let excl = vec!["archive".to_string()];
        assert!(
            is_excluded(Path::new("/Users/dev/Archive/secret.rs"), &excl),
            "a stored `archive` must exclude on-disk `Archive` — they are one directory here"
        );
        let excl = vec!["/Users/dev/Archive".to_string()];
        assert!(
            is_excluded(Path::new("/Users/dev/archive/secret.rs"), &excl),
            "and the absolute form too"
        );
        // Still boundary-anchored: folding case must not make it a substring match.
        assert!(
            !is_excluded(Path::new("/Users/dev/Archiver/x.rs"), &["archive".to_string()]),
            "`archive` must not exclude `Archiver`"
        );
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
    fn is_project_source_ext_covers_code_and_md_not_data() {
        // parser languages + common unparsed source + markdown count
        // `cpp`, `cc` and `hpp` are the C++ trio: SOURCE, but parsed by
        // nothing here since v1's C adapter was deleted. A folder holding them
        // is still a project.
        for e in [
            "py", "rs", "ts", "cpp", "cc", "hpp", "h", "go", "rb", "sh", "pl", "php", "lua", "md",
            "mdx",
        ] {
            assert!(is_project_source_ext(e), "{e} should count as project source");
            assert!(is_project_source_ext(&format!(".{e}")), "leading-dot {e} should count too");
        }
        // data / config / binaries are NOT a project signal on their own
        for e in ["csv", "txt", "json", "yaml", "toml", "png", "lock", "log", "pdf"] {
            assert!(!is_project_source_ext(e), "{e} should NOT count as project source");
        }
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
}
