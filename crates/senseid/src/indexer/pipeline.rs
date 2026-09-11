//! Stages 1-3 wired end to end: scan_root -> scan_repo -> structure write.
//!
//! Specs: `docs/spec/indexer/01-scan-root.md`, `02-scan-repo.md`,
//! `03-structure-write.md`.
//!
//! This is the IO half. The decisions all live in the pure functions those
//! stages already provide — [`super::scan_root::find_git_roots`],
//! [`super::scan_repo::scan_repo_files`], [`super::structure::plan_structure`]
//! — and this module only executes a plan it does not think about. That split
//! is why the interesting behaviour is testable without a database.
//!
//! It writes repositories, folders and files. It writes NO nodes and NO
//! edges, and enqueues no parse task: stage 3's barrier ends here (R14).

use std::collections::{BTreeMap, BTreeSet};

use super::scan_repo::{self, RepoScan};
use super::scan_root::{self, RepoRoot, RootExclusions};
use super::structure::{self, ChangeKind, FileFacts};
use crate::db::pg_store::PgStore;

/// What one repo's structure write produced. Counts only — the rows are in
/// the database, which is where they are inspected.
#[derive(Debug, Clone, Default)]
pub struct RepoResult {
    pub abs_path: String,
    pub repository_id: Option<uuid::Uuid>,
    pub folder_id: Option<uuid::Uuid>,
    pub folders: usize,
    pub files: usize,
    pub manifests: usize,
    pub lockfiles: usize,
    pub submodules: usize,
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
    pub removed: usize,
    /// The library this repo declares itself to be, if it ships a
    /// `sensei.library.json` at its root.
    pub library: Option<String>,
    /// Package names grouped under that library (02b S2).
    pub library_packages: u64,
    pub errors: Vec<String>,
}

/// The whole scan.
#[derive(Debug, Clone, Default)]
pub struct ScanSummary {
    pub repos: Vec<RepoResult>,
    pub roots_found: usize,
    pub excluded_dirs: usize,
    pub unreadable: Vec<String>,
}

impl ScanSummary {
    pub fn total_files(&self) -> usize {
        self.repos.iter().map(|r| r.files).sum()
    }
    pub fn total_folders(&self) -> usize {
        self.repos.iter().map(|r| r.folders).sum()
    }
    pub fn total_manifests(&self) -> usize {
        self.repos.iter().map(|r| r.manifests).sum()
    }
    /// Every repo-level failure, prefixed with the repo it came from. A caller
    /// that only looks at the counts cannot tell a repo that failed from one
    /// that legitimately had nothing — this is how it tells.
    pub fn errors(&self) -> Vec<String> {
        self.repos
            .iter()
            .flat_map(|r| r.errors.iter().map(move |e| format!("{}: {e}", r.abs_path)))
            .collect()
    }
}

/// Read `origin`'s URL, or the first remote if there is no `origin`.
///
/// Reuses the walk-level reader in `tasks::handlers::scan` rather than
/// shelling out a second time — a second copy would be a second place for
/// "what counts as this repo's remote" to be answered differently.
fn origin_remote(repo_path: &str) -> Option<String> {
    let remotes = crate::tasks::handlers::scan::read_git_remotes(repo_path);
    let pick = remotes.iter().find(|r| r["name"] == "origin").or_else(|| remotes.first())?;
    pick["url"].as_str().map(str::to_string)
}

/// This file's `(mtime, sha256)`, reusing the previous scan's hash when the
/// timestamp has not moved.
///
/// The hash is what decides re-parsing, and computing it means READING the
/// file — 48,665 reads on a repo this size. The mtime is a stat. So the mtime
/// gates the read: if the file was seen before at this exact timestamp its
/// bytes cannot have changed, and last scan's hash is still its hash.
///
/// This is the two-tier gate v1 already used (`scan_logic::plan_reindex`),
/// kept OUT of [`structure::plan_structure`] so that function stays pure and
/// hash-only. It is a caching decision, not a classification one.
fn file_facts(path: &std::path::Path, previous: Option<&FileFacts>) -> Option<FileFacts> {
    let mtime = crate::tasks::handlers::helpers::file_mtime_ms(path)?;
    if let Some(prev) = previous
        && prev.mtime == mtime
    {
        return Some(FileFacts { mtime, hash: prev.hash.clone() });
    }
    let hash = crate::tasks::handlers::helpers::hash_file(path)?;
    Some(FileFacts { mtime, hash })
}

/// Every workspace member this repo DECLARES, as repo-relative paths.
///
/// Asks every registered adapter, because one repo can declare members in more
/// than one ecosystem — this one has a Cargo workspace AND npm workspaces, and
/// taking only the first adapter's answer would label the other's members
/// `package`.
///
/// Reads the filesystem (each adapter opens the root manifest and resolves its
/// globs), which is why it lives here in the IO half rather than inside
/// [`structure::plan_folders`], which stays pure and is handed the result.
fn declared_workspace_members(repo_root: &std::path::Path) -> BTreeSet<String> {
    crate::adapters::manifest::registered_adapters()
        .iter()
        .flat_map(|a| a.detect_workspace_members(repo_root))
        .map(|m| m.path)
        .collect()
}

/// Run stages 1-3 over `scan_dir` and write the structure.
///
/// `root_id` is the `folders_to_watch` row every folder hangs off.
pub async fn scan_and_write_structure(
    pg: &PgStore,
    scan_dir: &std::path::Path,
    root_id: &uuid::Uuid,
) -> ScanSummary {
    let mut summary = ScanSummary::default();

    // ── Stage 1 ──────────────────────────────────────────────────────────
    let roots = scan_root::find_git_roots(scan_dir, &RootExclusions::defaults());
    summary.roots_found = roots.roots.len();
    summary.excluded_dirs = roots.excluded;
    summary.unreadable = roots.unreadable.iter().map(|u| u.path.display().to_string()).collect();

    for root in &roots.roots {
        summary.repos.push(write_one_repo(pg, root, root_id).await);
    }
    summary
}

async fn write_one_repo(pg: &PgStore, root: &RepoRoot, root_id: &uuid::Uuid) -> RepoResult {
    let abs = root.abs_path.to_string_lossy().to_string();
    let mut out = RepoResult { abs_path: abs.clone(), ..Default::default() };
    let name =
        root.abs_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

    // ── Stage 1 S4/S5: repository identity, then the root folder ────────
    match pg.upsert_repository(&name, origin_remote(&abs).as_deref()).await {
        Ok(id) => out.repository_id = Some(id),
        Err(e) => out.errors.push(format!("upsert_repository: {e}")),
    }

    let folder_id =
        match pg.upsert_folder(root_id, "git", &name, &abs, &abs, None, None, None).await {
            Ok(id) => id,
            Err(e) => {
                out.errors.push(format!("upsert_folder(root): {e}"));
                return out;
            }
        };
    out.folder_id = Some(folder_id);
    out.folders += 1;

    // S4/S6: the link, both directions asserted by the caller.
    if let Some(rid) = out.repository_id
        && let Err(e) = pg.link_folder_to_repository(&folder_id, &rid).await
    {
        out.errors.push(format!("link_folder_to_repository: {e}"));
    }

    // ── Stage 2 ──────────────────────────────────────────────────────────
    if let Ok(text) = std::fs::read_to_string(root.abs_path.join(".gitmodules")) {
        out.submodules = scan_repo::find_submodules(&text).len();
    }
    let scan: RepoScan = scan_repo::scan_repo_files(&root.abs_path);
    out.manifests = scan.manifests.len();
    out.lockfiles = scan.lockfiles.len();

    // ── Stage 3 ──────────────────────────────────────────────────────────
    // One repo's barrier failing is recorded against THAT repo and does not
    // stop the scan: the other roots are independent and their structure is
    // still correct. `out.errors` non-empty is how a caller tells the
    // difference between "this repo has no files" and "this repo failed".
    if let Err(e) = write_structure(pg, root_id, &root.abs_path, &folder_id, &scan, &mut out).await
    {
        out.errors.push(e);
    }

    // ── Stage 2b: this repo may BE a library ─────────────────────────────
    ingest_library(pg, &root.abs_path, &scan, &mut out).await;
    out
}

/// If this repo ships a `sensei.library.json` at its root, register the
/// library and group the packages it publishes (02b S1, S2).
///
/// THE MISSING TRIGGER. Every piece of this was already built —
/// `read_manifest`, `ingest_manifest_at`, `replace_library_packages` — and
/// `library_packages` still had zero rows, because the only two callers were a
/// manual API endpoint and a task that requires a URL. Nothing ran it during a
/// scan, so a library sitting on disk was never noticed.
async fn ingest_library(
    pg: &PgStore,
    repo_root: &std::path::Path,
    scan: &RepoScan,
    out: &mut RepoResult,
) {
    let Some(m) = crate::libraries::read_manifest(repo_root) else {
        return; // not a library. The common case, and not an error.
    };
    out.library = Some(m.library.clone());

    // The ecosystem comes from the manifest AT THE REPO ROOT — rokkit's
    // package.json makes it npm, dbd's Cargo.toml makes it cargo. It is never
    // guessed from the name: `libraries` is keyed `(ecosystem, name)`, so a
    // wrong ecosystem mints a second identity for one library and the grouping
    // attaches to the wrong row. No root manifest means no answer, and this
    // says so rather than picking one.
    let Some(ecosystem) =
        scan.manifests.iter().find(|(p, _)| p.parent() == Some(repo_root)).map(|(_, eco)| *eco)
    else {
        out.errors.push(format!(
            "library {}: no manifest at the repo root, so its ecosystem is unknown — not registered",
            m.library
        ));
        return;
    };

    let library_id =
        match pg.upsert_library(&m.library, ecosystem, Some(&m.version), None, None, None).await {
            Ok(id) => id,
            Err(e) => {
                out.errors.push(format!("upsert_library({}): {e}", m.library));
                return;
            }
        };

    match crate::libraries::ingest_manifest_at(pg, &library_id, repo_root).await {
        Some((_, _, np)) => out.library_packages = np,
        None => out.errors.push(format!("ingest_manifest_at({}): returned nothing", m.library)),
    }
}

/// The barrier (S1): every folder row, then every file row, then stop.
///
/// Returns `Err` rather than a half-written structure. A partial structure
/// means the barrier never happened, and half a denominator is worse than
/// none (03 §4) — so the folder loop aborts on the first failure and the
/// previous-state read is propagated, never defaulted.
async fn write_structure(
    pg: &PgStore,
    root_id: &uuid::Uuid,
    repo_root: &std::path::Path,
    root_folder_id: &uuid::Uuid,
    scan: &RepoScan,
    out: &mut RepoResult,
) -> Result<(), String> {
    let declared = declared_workspace_members(repo_root);
    let fplan = structure::plan_folders(repo_root, &scan.manifest_dirs(), &declared, &scan.files);

    // Folders first, parents before children — `plan_folders` guarantees that
    // ordering, and it is what lets `parent_id` be resolved from the map
    // rather than created on the fly, which would hide an ordering bug.
    let mut ids: BTreeMap<&std::path::Path, uuid::Uuid> = BTreeMap::new();
    ids.insert(repo_root, *root_folder_id);
    for f in &fplan.folders {
        if f.abs_path == repo_root {
            continue; // written by `write_one_repo` as kind `git`
        }
        let parent = f.parent.as_ref().and_then(|p| ids.get(p.as_path())).copied();
        let parent = parent.ok_or_else(|| {
            format!("folder {} has no parent row — plan_folders ordering bug", f.abs_path.display())
        })?;
        let rel = f.abs_path.strip_prefix(repo_root).map_err(|e| e.to_string())?;
        let name =
            f.abs_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        // The workspace that DECLARES this module, resolved from its path to
        // the folder id. `plan_folders` names WHICH root rather than a bare
        // flag, so this is a map lookup — and it stays correct if a nested
        // workspace root is ever detected.
        let ws_root = f.workspace_root.as_ref().and_then(|w| ids.get(w.as_path())).copied();
        let id = pg
            .upsert_folder(
                root_id,
                f.kind,
                &name,
                &rel.to_string_lossy(),
                &f.abs_path.to_string_lossy(),
                Some(&parent),
                None,
                ws_root.as_ref(),
            )
            .await
            .map_err(|e| format!("upsert_folder({}): {e}", rel.display()))?;
        ids.insert(f.abs_path.as_path(), id);
        out.folders += 1;
    }

    // Then the files, one folder at a time. The plan is per FOLDER because
    // `files` is keyed `(folder_id, file_path)` and `expected_files` is a
    // per-folder denominator — a repo-wide plan would give every workspace
    // member a denominator of zero and make the recursive rollup meaningless.
    for (folder_abs, planned) in fplan.by_folder() {
        let folder_id =
            ids.get(folder_abs).copied().ok_or_else(|| format!("no id for {folder_abs:?}"))?;

        // What the LAST scan recorded. Read before the walk's facts, because
        // it supplies the hashes `file_facts` reuses for files whose mtime has
        // not moved.
        //
        // The error is PROPAGATED, never defaulted to empty. An empty previous
        // state is a real thing — a folder scanned for the first time — and a
        // failed read that returned one would be indistinguishable from it:
        // every file would classify as Added and re-parse, and `plan.removed`
        // would be empty so nothing already-deleted would ever reconcile.
        let previous: BTreeMap<String, FileFacts> = pg
            .list_scan_state_full(&folder_id)
            .await
            .map_err(|e| format!("list_scan_state_full({folder_abs:?}): {e}"))?
            .into_iter()
            .map(|(path, mtime, hash)| (path, FileFacts { mtime, hash }))
            .collect();

        let current: BTreeMap<String, FileFacts> = planned
            .iter()
            .filter_map(|pf| {
                let facts = file_facts(&pf.abs_path, previous.get(&pf.rel_path))?;
                Some((pf.rel_path.clone(), facts))
            })
            .collect();

        let plan = structure::plan_structure(&current, &previous);
        out.added += plan.count(ChangeKind::Added);
        out.changed += plan.count(ChangeKind::ContentChanged);
        out.unchanged += plan.count(ChangeKind::Unchanged) + plan.count(ChangeKind::TouchedOnly);
        out.removed += plan.removed.len();

        for (rel, _) in &plan.files {
            let Some(f) = current.get(rel) else { continue };
            pg.upsert_file_row(&folder_id, rel, f.mtime, &f.hash, None)
                .await
                .map_err(|e| format!("upsert_file_row({rel}): {e}"))?;
            out.files += 1;
        }

        // S3: the denominator, recorded AT the barrier from the plan — never
        // recomputed later by a second count.
        pg.set_folder_expected_files(&folder_id, plan.expected_files() as i64)
            .await
            .map_err(|e| format!("set_folder_expected_files({folder_abs:?}): {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod corpus {
    use super::*;

    /// What to scan: `SENSEI_SCAN_DIR` if set, else the repo this crate lives
    /// in (`crates/senseid` up two levels). The override is how the same
    /// runner does the real rebuild over `~/Developer` instead of one repo.
    fn scan_dir() -> std::path::PathBuf {
        match std::env::var("SENSEI_SCAN_DIR") {
            Ok(d) => std::path::PathBuf::from(d).canonicalize().expect("SENSEI_SCAN_DIR"),
            Err(_) => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .canonicalize()
                .unwrap(),
        }
    }

    /// The `folders_to_watch` row `dir` belongs under.
    ///
    /// Longest existing prefix wins. Creating a root nested inside another is
    /// the failure this avoids: the same directory would then be described by
    /// two trees with different `root_id`s, and `folder_ids_for_root` — which
    /// scopes reconcile — would return a different set depending on which one
    /// the caller started from.
    async fn resolve_watch_root(pg: &PgStore, dir: &std::path::Path) -> uuid::Uuid {
        let existing: Vec<(uuid::Uuid, String)> =
            sqlx_core::query_as::query_as("SELECT id, path FROM sensei.folders_to_watch")
                .fetch_all(pg.pool())
                .await
                .expect("list watch roots");

        let best = existing
            .iter()
            .filter(|(_, p)| dir.starts_with(p))
            .max_by_key(|(_, p)| p.len())
            .map(|(id, _)| *id);
        if let Some(id) = best {
            return id;
        }

        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.folders_to_watch(path, name, status)
             VALUES($1, $2, 'watching'::sensei.watch_status)
             ON CONFLICT (path) DO UPDATE SET name = EXCLUDED.name
             RETURNING id",
        )
        .bind(dir.to_string_lossy().to_string())
        .bind(dir.file_name().unwrap().to_string_lossy().to_string())
        .fetch_one(pg.pool())
        .await
        .expect("create watch root");
        id
    }

    /// Run stages 1-3 against THIS repository and write the structure.
    ///
    /// `#[ignore]` because it needs a database and writes real rows — the same
    /// reason stage 2's corpus check is ignored. Run it deliberately:
    ///
    /// ```text
    /// cargo test -p senseid --lib indexer::pipeline::corpus -- --ignored --nocapture
    /// ```
    ///
    /// Defaults to `sensei_test`. Point it at another database with
    /// `TEST_DATABASE_URL`.
    #[tokio::test]
    #[ignore]
    async fn scan_this_repo_and_write_structure() {
        let pg = PgStore::connect_test().await.expect("connect");
        let root = scan_dir();

        // The watch root every folder hangs off. Resolve to the LONGEST
        // EXISTING root that contains the scan dir before creating one:
        // `~/Developer` is already a watch root here, and adding a
        // second one nested inside it would give this repo's folders a
        // different root_id from every sibling repo's — two trees describing
        // one directory. Only a scan dir under no existing root gets a new one.
        let root_id = resolve_watch_root(&pg, &root).await;

        let t = std::time::Instant::now();
        let summary = scan_and_write_structure(&pg, &root, &root_id).await;
        let elapsed = t.elapsed();

        println!("\n── stages 1-3 over {} ──", root.display());
        println!("roots found     {}", summary.roots_found);
        println!("dirs excluded   {}", summary.excluded_dirs);
        println!("folders written {}", summary.total_folders());
        println!("files written   {}", summary.total_files());
        println!("manifests       {}", summary.total_manifests());
        let libs: Vec<&RepoResult> = summary.repos.iter().filter(|r| r.library.is_some()).collect();
        println!("libraries       {}", libs.len());
        for l in &libs {
            println!(
                "  library {:14} packages={}",
                l.library.as_deref().unwrap_or(""),
                l.library_packages
            );
        }
        for r in &summary.repos {
            println!(
                "  {} -> repo={:?} folders={} files={} manifests={} lockfiles={} submodules={} \
                 added={} changed={} unchanged={} removed={}",
                r.abs_path,
                r.repository_id.is_some(),
                r.folders,
                r.files,
                r.manifests,
                r.lockfiles,
                r.submodules,
                r.added,
                r.changed,
                r.unchanged,
                r.removed
            );
        }
        let errors = summary.errors();
        for e in &errors {
            println!("  ERROR {e}");
        }
        println!("elapsed         {elapsed:?}\n");

        assert!(errors.is_empty(), "structure write reported failures");
        assert!(summary.roots_found >= 1, "at least one repo root");
        assert!(summary.total_files() > 1_000, "this repo has thousands of files");
        assert!(
            summary.total_manifests() >= 18,
            "stage 2 measured 18 manifests in this repo alone"
        );
    }
}
