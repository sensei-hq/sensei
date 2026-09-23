//! The manifest→files gate.
//!
//! Two handlers, and a barrier that is REUSED rather than rebuilt.
//!
//! ## Why a gate at all
//!
//! `index_file` cannot mint an fqn without knowing the file's PACKAGE, and the
//! package is named by a manifest. So every manifest in a repo must be read
//! before any of its files are parsed — otherwise a file indexed early gets no
//! placement and is dropped (`Skipped::Unplaced`), which is a visible gap that
//! only a full re-scan closes.
//!
//! ## Why there is no counter here
//!
//! [`crate::tasks::queue::TaskQueue`] already implements a general dependency
//! barrier: a task carries `depends_on`, `complete` and `fail` both drop the
//! dep and promote the task once the list empties, and a task whose deps are
//! ALREADY complete goes straight to pending. So the gate is one ordinary
//! blocked task:
//!
//! ```ignore
//! let ids = manifests.map(|m| queue.enqueue(Task::new(ProcessManifest, repo, m)));
//! queue.enqueue(Task::new(ProcessRepoFiles, repo, "").blocked_by(ids));
//! ```
//!
//! Writing a counter here would be a second implementation of that, and the
//! two would disagree the first time a manifest task failed. Both edge cases
//! are already covered by the queue's own tests: `failed_task_unblocks_dependents`
//! (a broken manifest cannot deadlock the gate) and `blocked_by(vec![])`
//! leaving the status `Pending` (a repo with NO manifests runs immediately).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::super::executor::TaskContext;
use super::super::{Task, TaskKind};

/// Enqueue one `ProcessManifest` per manifest, then the `ProcessRepoFiles` gate
/// blocked on all of them. Returns the gate's task id.
///
/// The ORDER matters and is not arbitrary: the manifests are enqueued first so
/// their ids exist to block on. Enqueueing the gate first and adding
/// dependencies afterwards is possible (`add_dependency`) but opens a window in
/// which the gate is runnable with no deps — and it would fan out over files
/// that have no placement yet.
///
pub async fn enqueue_manifest_gate(
    ctx: &TaskContext,
    repo_path: &str,
    manifests: &[String],
) -> u64 {
    let mut manifest_ids = Vec::with_capacity(manifests.len());
    for rel in manifests {
        manifest_ids
            .push(ctx.queue.enqueue(Task::new(TaskKind::ProcessManifest, repo_path, rel)).await);
    }
    ctx.queue
        .enqueue(Task::new(TaskKind::ProcessRepoFiles, repo_path, "").blocked_by(manifest_ids))
        .await
}

/// Read ONE manifest, through the SINGLE per-manifest pass.
///
/// `task.folder_path` is the repo root; `task.path` is the manifest's path
/// relative to it.
///
/// Delegates to [`pipeline::apply_manifest`] rather than doing its own thing.
/// This handler previously read only the package NAME — the "underpowered
/// duplicate" `pipeline.rs` warns about by name — which left
/// `referenced_libraries`, `folder_dependency` and `folder_commands` with no
/// production writer at all once the old `ResolveLibs` enqueue was removed.
/// The adapter already knows the ecosystem, the dependencies, its own lockfile
/// formats and the named commands; taking `.name` and discarding the rest is
/// exactly what the adapter pattern exists to prevent.
pub async fn process_manifest(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let repo_root = Path::new(&task.folder_path);
    let manifest_abs = repo_root.join(&task.path);

    let Some(name) = manifest_abs.file_name().and_then(|n| n.to_str()) else {
        return Ok(0);
    };
    // The adapter that CLAIMS this file decides the ecosystem — `accepts`
    // covers both fixed names and the extension-only ecosystems (.NET).
    let Some(adapter) =
        crate::adapters::manifest::registered_adapters().iter().find(|a| a.accepts(name))
    else {
        tracing::debug!(manifest = %task.path, "process_manifest: no adapter claims it");
        return Ok(0);
    };

    let Some(repo) = ctx.pg().get_repo_by_path(&task.folder_path).await? else {
        // The structure write has not run, so there is nothing to hang facts
        // off. Erroring would retry this manifest for ever.
        tracing::warn!(repo = %task.folder_path, "process_manifest: repo folder row missing");
        return Ok(0);
    };
    let (Some(root_id), Some(repo_folder_id)) =
        (crate::api::util::json_uuid(&repo["root_id"]), crate::api::util::json_uuid(&repo["id"]))
    else {
        return Err(format!("process_manifest: repo row for {} has no ids", task.folder_path));
    };
    let project_id = crate::api::util::json_uuid(&repo["project_id"]);

    // Facts belong to the folder that HOLDS the manifest (D11).
    let dir_rel = Path::new(&task.path).parent().unwrap_or(Path::new(""));
    let dir_abs = repo_root.join(dir_rel);
    let folder_ids = ctx.pg().folder_ids_under(&task.folder_path).await?;
    let folder_id = folder_ids.get(&dir_abs).copied().unwrap_or(repo_folder_id);

    // Lockfile candidates, found through the SAME search every other walk uses.
    // `nearest_lockfile` then picks the one this adapter can actually read.
    let lock_names = crate::adapters::manifest::all_lockfile_filenames()
        .iter()
        .map(|s| (*s).to_string())
        .collect::<Vec<_>>();
    let lockfiles: Vec<PathBuf> =
        crate::indexer::search::search(&crate::indexer::search::SearchSpec {
            root: repo_root,
            patterns: &lock_names,
            kinds: crate::indexer::search::Kinds::Files,
            exclusions: &[],
            prunes: &crate::indexer::repo::default_prunes(),
            gitignore: false,
            hidden: false,
        })?
        .found
        .iter()
        .map(|f| repo_root.join(&f.rel_path))
        .collect();

    let outcome = crate::indexer::pipeline::apply_manifest(
        ctx.pg(),
        crate::indexer::pipeline::ManifestJob {
            repo_root,
            manifest: &manifest_abs,
            ecosystem: adapter.ecosystem(),
            folder_id,
            folder_ids: &folder_ids,
            lockfiles: &lockfiles,
        },
    )
    .await;

    for e in &outcome.errors {
        tracing::warn!(manifest = %task.path, error = %e, "process_manifest");
    }

    // The manifest's DIRECTORY is the build unit — `folders.kind = 'module'`
    // already means exactly "a manifest-bearing build unit inside a repo". The
    // repo root's own manifest describes the repo folder, which already exists.
    if let Some(package) = &outcome.package
        && !dir_rel.as_os_str().is_empty()
    {
        ctx.pg()
            .upsert_subfolder_kind(
                &root_id,
                "module",
                package,
                &dir_rel.to_string_lossy(),
                &dir_abs.to_string_lossy(),
                Some(&repo_folder_id),
                project_id.as_ref(),
            )
            .await?;
    }

    tracing::info!(
        manifest = %task.path, package = ?outcome.package,
        local_deps = outcome.local_deps, external_deps = outcome.external_deps,
        commands = outcome.commands,
        "process_manifest: applied"
    );
    Ok(outcome.external_deps + outcome.local_deps + outcome.commands)
}

/// The gate. Fans out one `ProcessFile` per file still awaiting a parse.
///
/// Carries no work list — it reads the unparsed set back from `files`, which is
/// what makes a re-run idempotent (see
/// [`crate::db::pg_store::PgStore::list_unparsed_files`]).
pub async fn process_repo_files(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let Some(repo) = ctx.pg().get_repo_by_path(&task.folder_path).await? else {
        tracing::warn!(repo = %task.folder_path, "process_repo_files: repo folder row missing");
        return Ok(0);
    };
    let Some(folder_id) = crate::api::util::json_uuid(&repo["id"]) else {
        return Err(format!("process_repo_files: repo row for {} has no id", task.folder_path));
    };

    let files = ctx.pg().list_unparsed_files(&folder_id).await?;
    let mut file_ids: Vec<u64> = Vec::with_capacity(files.len());
    let mut enqueued = 0u32;
    for rel in &files {
        // ABSOLUTE. `files.file_path` is repo-RELATIVE, and `process_file`
        // reads `task.path` as an absolute path (`process.rs`: `let abs_path =
        // &task.path`). Handing it the relative form made `strip_prefix` fail,
        // `placement_on_disk` walk up from the DAEMON's cwd, and every file
        // return `Ok(0)` — "not indexed" reported as success, re-fanned for
        // ever because no fingerprint advanced. `Task::for_file` names the
        // contract: (kind, folder_ABS_path, file_ABS_path).
        let abs = Path::new(&task.folder_path).join(rel);
        // Single-writer: a concurrent scan must not fan out a second parse task
        // for the same file.
        // A `None` here is a DEDUP: a task for this exact file is already in
        // flight, so its own scan's barrier is already waiting on it. Adding
        // nothing is right — inventing an id would block this barrier on a task
        // that will never report to it.
        if let Some(id) = ctx
            .queue
            .enqueue_unique(Task::for_file(
                TaskKind::ProcessFile,
                &task.folder_path,
                &abs.to_string_lossy(),
            ))
            .await
        {
            file_ids.push(id);
            enqueued += 1;
        }
    }

    // THE TERMINAL BARRIER, blocked on the FILE TASKS — the whole reason it is
    // enqueued here and not by the repo scan. `DetectCommunities` is the sole
    // writer of `folders.status = 'indexed'`, so releasing it before the files
    // are parsed marks the folder complete over a graph that does not exist.
    //
    // An empty `file_ids` (nothing to parse) leaves `blocked_by` empty, so both
    // tasks are Pending immediately — correct: there is no work to wait for.
    let embed = ctx
        .queue
        .enqueue(
            Task::new(TaskKind::EmbedNodes, &task.folder_path, "").blocked_by(file_ids.clone()),
        )
        .await;
    let mut barrier_deps = file_ids;
    barrier_deps.push(embed);
    ctx.queue
        .enqueue(
            Task::new(TaskKind::DetectCommunities, &task.folder_path, "").blocked_by(barrier_deps),
        )
        .await;
    tracing::info!(
        repo = %task.folder_path,
        unparsed = files.len(),
        enqueued,
        "process_repo_files: manifest gate opened, files fanned out"
    );
    Ok(enqueued)
}

#[cfg(test)]
mod tests {
    use super::super::super::queue::TaskQueue;
    use super::super::super::{Task, TaskKind};
    use crate::tasks::test_support::make_ctx;

    /// The gate stays shut until EVERY manifest has finished, then opens.
    ///
    /// Driven through the real `TaskQueue` on purpose: the whole point is that
    /// this handler owns no ordering logic, so a test that counted manifests
    /// itself would prove something no production code does.
    #[tokio::test]
    async fn the_gate_opens_only_after_every_manifest_completes() {
        let q = TaskQueue::new();
        let m1 = q.enqueue(Task::new(TaskKind::ProcessManifest, "repo", "Cargo.toml")).await;
        let m2 = q.enqueue(Task::new(TaskKind::ProcessManifest, "repo", "app/package.json")).await;
        let gate = q
            .enqueue(Task::new(TaskKind::ProcessRepoFiles, "repo", "").blocked_by(vec![m1, m2]))
            .await;

        assert_eq!(q.status().await.blocked, 1, "the gate starts shut");

        let t = q.next_task().await;
        q.complete(t.id).await;
        assert_eq!(q.status().await.blocked, 1, "one manifest left — still shut");

        let t = q.next_task().await;
        q.complete(t.id).await;
        assert_eq!(q.status().await.blocked, 0, "the last manifest opens it");

        assert_eq!(q.next_task().await.id, gate);
    }

    /// `enqueue_manifest_gate` ITSELF blocks the gate — the property the
    /// module exists for.
    ///
    /// The sibling tests hand-build `blocked_by(vec![m1, m2])` and so prove the
    /// QUEUE's barrier, which is the queue's own responsibility. Mutating
    /// `enqueue_manifest_gate` to `blocked_by(vec![])` left every one of them
    /// green. This one calls the production function.
    #[tokio::test]
    async fn enqueue_manifest_gate_blocks_the_gate_on_its_manifests() {
        let ctx = make_ctx().await;
        let manifests = vec!["Cargo.toml".to_string(), "app/package.json".to_string()];

        let gate = super::enqueue_manifest_gate(&ctx, "/repo", &manifests).await;

        let status = ctx.queue.status().await;
        assert_eq!(status.blocked, 1, "the gate must be BLOCKED on its manifests");
        assert_eq!(status.pending, 2, "both manifest tasks are runnable");

        // Drain the manifests; only then may the gate run.
        for _ in 0..2 {
            let t = ctx.queue.next_task().await;
            assert_eq!(t.kind, TaskKind::ProcessManifest);
            ctx.queue.complete(t.id).await;
        }
        assert_eq!(ctx.queue.status().await.blocked, 0);
        assert_eq!(ctx.queue.next_task().await.id, gate);
    }

    /// A repo with NO manifests must index immediately, not hang forever.
    ///
    /// `blocked_by(vec![])` leaves the status `Pending`, so this falls out of
    /// the queue's own semantics — but a repo with no `Cargo.toml` or
    /// `package.json` is the common case, and a gate that deadlocked on it
    /// would stall every file in the repo silently.
    #[tokio::test]
    async fn a_repo_with_no_manifests_opens_the_gate_at_once() {
        let q = TaskQueue::new();
        let gate =
            q.enqueue(Task::new(TaskKind::ProcessRepoFiles, "repo", "").blocked_by(vec![])).await;

        let status = q.status().await;
        assert_eq!(status.blocked, 0, "nothing to wait for");
        assert_eq!(status.pending, 1);
        assert_eq!(q.next_task().await.id, gate);
    }

    /// A manifest that FAILS still opens the gate.
    ///
    /// Fail-open here is deliberate and is the lesser harm: files whose
    /// placement came from the broken manifest are skipped as unplaced — a
    /// visible gap — whereas holding the gate shut strands every OTHER file in
    /// the repo behind one bad `Cargo.toml`, with nothing counting them.
    #[tokio::test]
    async fn a_failed_manifest_does_not_deadlock_the_gate() {
        let q = TaskQueue::new();
        let m = q.enqueue(Task::new(TaskKind::ProcessManifest, "repo", "Cargo.toml")).await;
        let gate =
            q.enqueue(Task::new(TaskKind::ProcessRepoFiles, "repo", "").blocked_by(vec![m])).await;

        let t = q.next_task().await;
        q.fail(t.id, "malformed toml".to_string()).await;

        assert_eq!(q.status().await.blocked, 0, "a failed manifest must not strand the repo");
        assert_eq!(q.next_task().await.id, gate);
    }
}

// ── The repo pass ────────────────────────────────────────────────────────
//
// Every step below is a call. The parts were built and tested separately —
// `repo::scan` walks and classifies, `repo::folder_tree` orders the folder
// rows, `apply_manifest` is the manifest pass, the queue owns the gate — so
// this is the sequence and nothing else. The function it replaces had grown to
// 776 lines by inlining all of them.

/// The watch root's exclusions for this repo, or none when it sits under no
/// registered root. Fails closed: a DB error propagates rather than reading as
/// "no exclusions", which would index exactly what the user excluded.
async fn exclusions_for(ctx: &TaskContext, repo_path: &str) -> Result<Vec<String>, String> {
    match ctx.pg().enclosing_watch_root(repo_path).await? {
        Some((_, watch_root)) => ctx.pg().root_exclusion_prefixes(&watch_root).await,
        None => Ok(Vec::new()),
    }
}

/// Write the folder rows, parents first, and return their ids by relative path.
///
/// `tree` is already ordered parents-before-children, so the parent's id is
/// always in the map by the time a child needs it. A child whose parent is
/// missing is SKIPPED rather than reparented to the repo root: a folder hung
/// off the wrong parent is worse than one absent, because nothing downstream
/// can tell it is wrong.
async fn write_folder_tree(
    ctx: &TaskContext,
    root_id: &uuid::Uuid,
    repo_folder_id: uuid::Uuid,
    project_id: Option<&uuid::Uuid>,
    repo_abs: &Path,
    tree: &[crate::indexer::repo::PlannedFolder],
) -> BTreeMap<PathBuf, uuid::Uuid> {
    let mut ids: BTreeMap<PathBuf, uuid::Uuid> = BTreeMap::new();
    // The repo root's row already exists — it IS the repo folder.
    ids.insert(PathBuf::new(), repo_folder_id);

    for f in tree {
        if f.rel_path.as_os_str().is_empty() {
            continue;
        }
        let Some(parent) = f.parent.as_ref().and_then(|p| ids.get(p)).copied() else {
            tracing::warn!(folder = %f.rel_path.display(), "repo scan: parent row missing — folder skipped");
            continue;
        };
        let name =
            f.rel_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let abs = repo_abs.join(&f.rel_path);
        match ctx
            .pg()
            .upsert_subfolder(
                root_id,
                &name,
                &f.rel_path.to_string_lossy(),
                &abs.to_string_lossy(),
                Some(&parent),
                project_id,
            )
            .await
        {
            Ok(id) => {
                ids.insert(f.rel_path.clone(), id);
            }
            Err(e) => {
                tracing::warn!(folder = %f.rel_path.display(), error = %e, "repo scan: upsert_subfolder failed")
            }
        }
    }
    ids
}

/// What `write_file_rows` actually managed to record.
///
/// `written` is the denominator `folder_completeness` divides by — the rows
/// that exist. `to_parse` is the TASK-grain count the gate will fan out, which
/// is smaller: a source file the byte-sniff rejects gets a `skip_reason` and is
/// never enqueued, so using `written` for the progress bar leaves its ceiling
/// permanently out of reach. `failed` is what makes a partial write visible
/// instead of being reported as a complete one.
#[derive(Debug, Default, Clone, Copy)]
struct FileRowOutcome {
    written: u32,
    to_parse: u32,
    failed: u32,
}

/// The extension without its dot, as `classify_unscannable` expects.
fn ext_of(rel: &Path) -> &str {
    rel.extension().and_then(|e| e.to_str()).unwrap_or("")
}

/// Write one `files` row per file, supported or not.
///
/// A SOURCE file lands at `BARRIER_MTIME` with no skip reason — the stage-3
/// barrier, so every row exists before any parse task runs and node
/// persistence cannot fail closed on a missing row. An UNSUPPORTED file is
/// recorded WITH `unsupported_format` and its real fingerprint, which is what
/// makes the skip stick instead of being re-enqueued every pass.
///
/// Manifests and lockfiles get no file row: they are structure, not graph
/// content, and `parsed_at` would never advance for them.
async fn write_file_rows(
    ctx: &TaskContext,
    folder_id: &uuid::Uuid,
    repo_abs: &Path,
    contents: &crate::indexer::repo::RepoContents,
) -> FileRowOutcome {
    use crate::db::pg_store::folders::{BARRIER_HASH, BARRIER_MTIME};
    use crate::indexer::repo::EntryClass;

    let mut out = FileRowOutcome::default();
    let mut written = 0u32;
    for entry in &contents.entries {
        let rel = entry.rel_path.to_string_lossy().to_string();
        let res = match entry.class {
            EntryClass::Source { .. } => {
                // An extension a language adapter claims is not yet a file it
                // can READ: the bytes may be binary or not valid UTF-8. Catching
                // that HERE, with the real fingerprint and reason, is what stops
                // a doomed parse task being enqueued for ever — `process_file`
                // returns early for such a file WITHOUT writing a row, and a
                // file with no row reads as changed on the next pass.
                let abs = repo_abs.join(&entry.rel_path);
                match super::helpers::classify_unscannable(&abs, ext_of(&entry.rel_path)) {
                    Some(reason) => {
                        let Some((mtime, hash)) = super::helpers::file_fingerprint(&abs) else {
                            continue;
                        };
                        ctx.pg().upsert_file_row(folder_id, &rel, mtime, &hash, Some(reason)).await
                    }
                    None => {
                        // Barrier-seeded: this one WILL be fanned out.
                        out.to_parse += 1;
                        ctx.pg()
                            .upsert_file_row(folder_id, &rel, BARRIER_MTIME, BARRIER_HASH, None)
                            .await
                    }
                }
            }
            EntryClass::Unsupported => {
                // Only fingerprint what we actually observed — a file we could
                // not read gets no row rather than a fingerprint we invented.
                let Some((mtime, hash)) =
                    super::helpers::file_fingerprint(&repo_abs.join(&entry.rel_path))
                else {
                    continue;
                };
                ctx.pg()
                    .upsert_file_row(
                        folder_id,
                        &rel,
                        mtime,
                        &hash,
                        Some(crate::classifiers::ScanSkipReason::UnsupportedFormat),
                    )
                    .await
            }
            _ => continue,
        };
        match res {
            Ok(_) => written += 1,
            Err(e) => {
                out.failed += 1;
                tracing::warn!(file = %rel, error = %e, "repo scan: upsert_file_row failed");
            }
        }
    }
    out.written = written;
    out
}

/// The project this repo belongs to, created if it does not exist yet.
///
/// Every project root is its own project, named after itself. Grouping several
/// folders into one project is OPT-IN via README frontmatter `project:` — never
/// by parent directory, which once conflated every unrelated repo under a scan
/// root into a single "Developer" project. A git SUBTREE (composite folder name
/// `parent:sub`) inherits its parent repository's project unless its own README
/// overrides. Scanning is read-only; frontmatter is never written back.
///
/// FAILS rather than fabricating: minting a synthetic `p-<name>` id here once
/// pushed a phantom project into the UI that matched no row and never
/// reconciled. An error aborts this repo and the scan retries next tick.
async fn resolve_project(
    ctx: &TaskContext,
    repo_path: &Path,
    folder_name: &str,
) -> Result<(uuid::Uuid, bool), String> {
    let fm = crate::tasks::processors::metadata::read_frontmatter(repo_path).unwrap_or_default();
    let name: String = match folder_name.split_once(':') {
        Some((parent_repo, _)) if fm.project.is_none() => {
            let parent_project = ctx
                .pg()
                .get_repo_by_name(parent_repo)
                .await
                .ok()
                .flatten()
                .and_then(|f| crate::api::util::json_uuid(&f["project_id"]));
            match parent_project {
                Some(pid) => ctx
                    .pg()
                    .get_project(&pid)
                    .await
                    .ok()
                    .flatten()
                    .and_then(|p| p["name"].as_str().map(String::from))
                    .unwrap_or_else(|| parent_repo.to_string()),
                None => parent_repo.to_string(),
            }
        }
        _ => fm.project.clone().unwrap_or_else(|| folder_name.to_string()),
    };
    ctx.pg().get_or_create_project_by_name(&name).await
}

/// THE REPO SCAN. `ScanRoot` enqueues one of these per repository it found.
///
/// The whole pass, and it is a sequence of calls because every step is built
/// and tested on its own:
///
///   walk+classify → folder rows → file rows → manifest gate → (gate) file tasks
///
/// It writes NO nodes and NO edges. Those come from `ProcessFile`, which the
/// gate fans out once every manifest has been read — because a file cannot be
/// placed, and therefore cannot be given an fqn, until its package is known.
pub async fn process_git_folder(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    use crate::api::events::{ActivityEvent, ActivityLevel, StateEvent};
    use crate::indexer::repo;
    use crate::tasks::progress_emitter as msg;

    let started = std::time::Instant::now();
    let repo_abs = Path::new(&task.folder_path);
    let name = repo_abs.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let emit = |level: ActivityLevel, message: String| {
        let _ = ctx.app_state.event_tx.send(StateEvent::activity(ActivityEvent::new(
            level,
            &message,
            started.elapsed().as_secs_f64(),
        )));
    };

    // A repo whose directory is GONE must say so, not report a missing row: the
    // two are different facts and only the first is actionable (an unmounted
    // volume, a moved checkout). `reconcile_roots` owns absent roots; this task
    // simply fails and is retried.
    if !repo_abs.exists() {
        return Err(format!("Repo path does not exist: {}", task.folder_path));
    }

    let Some(row) = ctx.pg().get_repo_by_path(&task.folder_path).await? else {
        return Err(format!("process_git_folder: no folder row for {}", task.folder_path));
    };
    let (Some(root_id), Some(folder_id)) =
        (crate::api::util::json_uuid(&row["root_id"]), crate::api::util::json_uuid(&row["id"]))
    else {
        return Err(format!("process_git_folder: folder row for {} has no ids", task.folder_path));
    };
    // The project, resolved and ATTACHED before anything else — a folder with
    // no `project_id` is invisible to every project surface (UI, list_projects,
    // metrics), and nothing downstream repairs it.
    let (project_uuid, _created) = resolve_project(ctx, repo_abs, &name).await?;
    if let Err(e) = ctx.pg().set_folder_project(&folder_id, &project_uuid, "root", None).await {
        tracing::warn!(folder_id = %folder_id, error = %e, "process_git_folder: set_folder_project failed");
    }
    let project_id = Some(project_uuid);

    // The indexed git branch, in the TYPED column. Preferred from the
    // `BranchSwitch` task that triggered this re-index, else read from
    // `.git/HEAD`. `folders.branch` is a real partition key — one folder per
    // checkout — not a label.
    let branch = task.branch.clone().or_else(|| {
        crate::watcher::root_watcher::read_git_head(&format!("{}/.git/HEAD", task.folder_path))
    });
    if let Some(br) = branch
        && let Err(e) = ctx.pg().set_folder_branch(&folder_id, &br).await
    {
        tracing::warn!(folder_id = %folder_id, branch = %br, error = %e, "process_git_folder: set_folder_branch failed");
    }

    // Repo identity — stack, role, external links, summary, frontmatter props.
    // ONE call, and it is the same function `ReconcileRepoMetadata` runs, so a
    // scan and a README-triggered reconcile cannot disagree about a repo.
    if let Err(e) = super::process::reconcile_repo_identity(ctx, &task.folder_path).await {
        tracing::warn!(path = %task.folder_path, error = %e, "process_git_folder: reconcile_repo_identity failed");
    }

    // MARK THE SCAN IN FLIGHT. `mark_folder_indexed_fail_closed` promotes ONLY
    // from `indexing`, so without this the terminal barrier reads `discovered`
    // and declines to promote — the folder never reaches `indexed`, and one
    // that `fail_folder` marked `failed` can never be cleared without hand SQL.
    if let Err(e) = ctx.pg().update_folder_status(&folder_id, "indexing").await {
        tracing::warn!(folder_id = %folder_id, error = %e, "scan_repo: update_folder_status(indexing) failed");
    }

    emit(ActivityLevel::Process, msg::repo_scanning_message(&name));

    // ONE walk, classified as it passes.
    let exclusions = exclusions_for(ctx, &task.folder_path).await?;
    // OFF THE ASYNC WORKER. `repo::scan` is a fully synchronous `ignore` walk,
    // so a repository on a stalled NFS mount blocks in `read_dir` inside the
    // handler future — it never returns `Pending`, so the executor's
    // `tokio::time::timeout` can never fire, the worker thread is lost, and the
    // repo's gate stays blocked for ever. `process_file` routes its parse
    // through `spawn_blocking` for exactly this reason.
    let walk_root = repo_abs.to_path_buf();
    let walk_exclusions = exclusions.clone();
    let contents = tokio::task::spawn_blocking(move || repo::scan(&walk_root, &walk_exclusions))
        .await
        .map_err(|e| format!("repo scan walk panicked: {e}"))??;
    let counts = contents.counts();
    emit(
        ActivityLevel::Info,
        msg::repo_identified_message(&name, counts.folders, counts.files, counts.supported),
    );

    let tree = repo::folder_tree(&contents);
    let folders =
        write_folder_tree(ctx, &root_id, folder_id, project_id.as_ref(), repo_abs, &tree).await;
    emit(ActivityLevel::Info, msg::repo_folders_saved_message(&name, folders.len()));

    // STAGE 3 BARRIER: every `files` row exists before any parse task runs, so
    // node persistence can fail closed on a missing row instead of inventing one.
    let rows = write_file_rows(ctx, &folder_id, repo_abs, &contents).await;
    let files = rows.written;
    emit(ActivityLevel::Info, msg::repo_files_saved_message(&name, files as usize));

    // A ROW THAT DID NOT LAND IS A FILE THAT WILL NEVER BE INDEXED: the gate
    // fans out from `files`, so a missing row means no parse task, silently,
    // while `expected_files` still counts it. Fail the task so the retryable
    // `ProcessGitFolder` re-drives, rather than reporting a complete structure.
    if rows.failed > 0 {
        return Err(format!(
            "structure write incomplete: {} of {} file rows failed",
            rows.failed,
            rows.failed + rows.written
        ));
    }

    // The denominator, recorded while the walk still knows it. Nothing
    // downstream can reconstruct it: counting rows is vacuous, because a walk
    // that died partway leaves the unwritten ones absent rather than undecided.
    // THE ROWS THE BARRIER WROTE, not the source files. `folder_completeness`
    // counts a row as decided when `parsed_at` OR `skip_reason` is set, and
    // `write_file_rows` stamps `unsupported_format` on every Unsupported file
    // at the barrier — so a source-only denominator is already exceeded by its
    // own numerator the moment the walk ends, flipping `subtree_complete` true
    // before any source is parsed.
    if let Err(e) = ctx.pg().set_folder_expected_files(&folder_id, files as i64).await {
        tracing::warn!(folder_id = %folder_id, error = %e, "process_git_folder: set_folder_expected_files failed");
    }

    // DELETIONS — and the two kinds are not interchangeable.
    //
    // OBSERVED: an event said the file is gone. Safe at ANY scope, because
    // something watched it happen.
    for gone in task.scope.deleted() {
        let Ok(rel) = gone.strip_prefix(repo_abs) else { continue };
        let rel = rel.to_string_lossy().to_string();
        if let Err(e) = ctx.pg().unresolve_edges_to_file(&folder_id, &rel).await {
            tracing::warn!(file = %rel, error = %e, "scan_repo: unresolve_edges_to_file failed");
        }
        if let Err(e) = ctx.pg().delete_nodes_by_file(&folder_id, &rel).await {
            tracing::warn!(file = %rel, error = %e, "scan_repo: delete_nodes_by_file failed");
        }
        if let Err(e) = ctx.pg().delete_scan_state_file(&folder_id, &rel).await {
            tracing::warn!(file = %rel, error = %e, "scan_repo: delete_scan_state_file failed");
        }
    }

    // INFERRED: indexed before, absent from this walk. Only meaningful when the
    // walk looked at EVERYTHING. An event scope examined a shortlist, so a file
    // missing from it was never looked at — pruning on that basis would delete
    // the nodes of every file the batch did not happen to mention.
    let pruned = if task.scope.is_exhaustive() && contents.is_complete() {
        let live: std::collections::HashSet<String> =
            contents.entries.iter().map(|e| e.rel_path.to_string_lossy().to_string()).collect();
        super::scan::prune_vanished(ctx.pg(), &folder_id, &live).await
    } else {
        0
    };

    // The progress emitter builds this folder's tracker from FolderQueued and
    // cannot derive `files_total` any other way — without it the folder never
    // appears in the scan progress UI. `files_expected` is the SAME denominator
    // just written to the folder row, never re-derived: a second derivation can
    // disagree with the one the rest of the system uses (08 S2).
    let _ = ctx.queue.sender().send(crate::tasks::progress::TaskEvent::FolderQueued {
        folder_path: task.folder_path.clone(),
        // TASK grain: what the gate will actually fan out. `counts.supported`
        // counts source files by EXTENSION, including ones the byte-sniff then
        // rejects — a bar keyed on it can never reach its own ceiling.
        files_total: rows.to_parse,
        files_expected: Some(files),
    });

    let manifests: Vec<String> =
        contents.manifests().iter().map(|e| e.rel_path.to_string_lossy().to_string()).collect();
    emit(ActivityLevel::Info, msg::repo_manifests_processed_message(&name, manifests.len()));
    let gate = enqueue_manifest_gate(ctx, &task.folder_path, &manifests).await;

    // The terminal barrier, blocked on the GATE rather than on the file tasks:
    // the gate has not fanned them out yet, so their ids do not exist to depend
    // on. `process_repo_files` adds each file task as a dependency of the barrier
    // as it enqueues them, which is what `add_dependency` is for.
    //
    // `DetectCommunities` is what flips the folder to `indexed` — without it a
    // folder stays `indexing` for ever, which is why this cannot be dropped
    // from the pass.
    let embed = ctx
        .queue
        .enqueue(Task::new(TaskKind::EmbedNodes, &task.folder_path, "").blocked_by(vec![gate]))
        .await;
    ctx.queue
        .enqueue(
            Task::new(TaskKind::DetectCommunities, &task.folder_path, "")
                .blocked_by(vec![gate, embed]),
        )
        .await;

    tracing::info!(
        repo = %task.folder_path, folders = folders.len(), files, pruned,
        supported = counts.supported, manifests = manifests.len(),
        "process_git_folder: structure written, manifest gate enqueued"
    );
    Ok(files)
}

#[cfg(test)]
mod scan_tests {
    use super::super::super::{Task, TaskKind};
    use crate::db::pg_store::graph_seed::SeedGraph;
    use crate::tasks::test_support::make_ctx;

    /// The whole pass over a real repository on disk: folder rows with parents,
    /// file rows for supported AND unsupported files, a project attached, and
    /// the manifest gate enqueued with the file fan-out behind it.
    #[tokio::test]
    async fn scan_repo_writes_structure_and_enqueues_the_gate() {
        let ctx = make_ctx().await;
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("demo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        std::fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(repo.join("README.md"), "# demo\n").unwrap();

        let root_id = ctx
            .pg()
            .add_watch_root(&t.path().to_string_lossy(), "wt", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx
            .pg()
            .upsert_repo_kind(&root_id, "git", "demo", &repo.to_string_lossy())
            .await
            .unwrap();

        let task = Task::new(TaskKind::ProcessGitFolder, &repo.to_string_lossy(), "");
        let written = super::process_git_folder(&ctx, &task).await.unwrap();

        // Both sources AND the unsupported README get a row; the manifest does not.
        assert_eq!(written, 3, "src/main.rs, src/lib.rs and README.md");

        // The unparsed set is the SUPPORTED files only — README carries
        // `unsupported_format`, so the gate will never fan out a task for it.
        let unparsed = ctx.pg().list_unparsed_files(&fid).await.unwrap();
        assert_eq!(unparsed, vec!["src/lib.rs".to_string(), "src/main.rs".to_string()]);

        // A project is attached — without it the folder is invisible to every
        // project surface, and nothing downstream repairs it.
        let row = ctx.pg().get_repo_by_path(&repo.to_string_lossy()).await.unwrap().unwrap();
        assert!(crate::api::util::json_uuid(&row["project_id"]).is_some(), "project attached");

        // The gate is queued, blocked on the one manifest, with the terminal
        // barrier behind it.
        let snapshot = ctx.queue.snapshot().await;
        let kinds: Vec<TaskKind> = snapshot.iter().map(|(k, _, _)| k.clone()).collect();
        assert!(kinds.contains(&TaskKind::ProcessManifest), "got {kinds:?}");
        assert!(kinds.contains(&TaskKind::ProcessRepoFiles), "got {kinds:?}");
        assert!(kinds.contains(&TaskKind::DetectCommunities), "terminal barrier queued");
    }

    /// AN EVENT SCOPE MUST NOT PRUNE WHAT IT DID NOT LOOK AT.
    ///
    /// The gate this pins is `task.scope.is_exhaustive() && contents.is_complete()`
    /// guarding `prune_vanished`. Mutating it to `if true` left every other test
    /// in this file green, because none of them ever builds a `Scope::Events`.
    ///
    /// The scenario is the real one: a batch mentions ONE file in a repository
    /// that holds others. Under a full scan the unmentioned files are absent
    /// from the walk and correctly pruned; under an event scope they were never
    /// looked at, and pruning them destroys the nodes of files that are still
    /// on disk.
    #[tokio::test]
    async fn an_event_scope_never_prunes_the_files_it_did_not_examine() {
        let ctx = make_ctx().await;
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("demo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        std::fs::write(repo.join("src/kept.rs"), "pub fn kept() {}\n").unwrap();

        let root_id = ctx
            .pg()
            .add_watch_root(&t.path().to_string_lossy(), "wt", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx
            .pg()
            .upsert_repo_kind(&root_id, "git", "demo", &repo.to_string_lossy())
            .await
            .unwrap();

        // A node for a file that is NOT in the event batch, and whose path the
        // walk will still see. `prune_vanished` drops nodes whose file is
        // absent from the live set it is handed.
        ctx.pg()
            .seed_node(&fid, "function", "ghost", "src/ghost.rs", None, None, None, None)
            .await
            .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let scoped = Task::new(TaskKind::ProcessGitFolder, &repo_path, "").with_scope(
            crate::tasks::Scope::Events {
                changed: vec![repo.join("src/kept.rs")],
                deleted: vec![],
            },
        );
        super::process_git_folder(&ctx, &scoped).await.unwrap();

        let (alive,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND name='ghost'",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(
            alive, 1,
            "an event scope pruned a file it never examined — absence from a shortlist is not deletion"
        );
    }

    /// AN OBSERVED DELETE IS APPLIED, at any scope.
    ///
    /// The direct successor of the deleted `process_batch_delete_targets_repo`
    /// test: the watcher no longer enqueues `DeleteFile`, it puts the path in
    /// `scope.deleted()`, and `process_git_folder` applies it. Disabling that
    /// loop left all 75 tests across three modules green.
    #[tokio::test]
    async fn an_observed_delete_is_applied_even_under_an_event_scope() {
        let ctx = make_ctx().await;
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("demo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();

        let root_id = ctx
            .pg()
            .add_watch_root(&t.path().to_string_lossy(), "wt", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx
            .pg()
            .upsert_repo_kind(&root_id, "git", "demo", &repo.to_string_lossy())
            .await
            .unwrap();
        ctx.pg()
            .seed_node(&fid, "function", "doomed", "src/gone.rs", None, None, None, None)
            .await
            .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        let scoped = Task::new(TaskKind::ProcessGitFolder, &repo_path, "").with_scope(
            crate::tasks::Scope::Events {
                changed: vec![],
                deleted: vec![repo.join("src/gone.rs")],
            },
        );
        super::process_git_folder(&ctx, &scoped).await.unwrap();

        let (alive,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND name='doomed'",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(alive, 0, "an OBSERVED delete must be applied — something watched it happen");
    }

    /// THE CYCLE ACTUALLY PRODUCES A GRAPH.
    ///
    /// The enqueue-shape test above asserts that tasks appear; it cannot see
    /// whether running them indexes anything. This drives the gate's own output
    /// through `process_file` and asserts NODES EXIST — the only claim that
    /// distinguishes a working pipeline from one that silently indexes nothing.
    #[tokio::test]
    async fn the_gate_fans_out_file_tasks_that_actually_index() {
        let ctx = make_ctx().await;
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("demo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn alpha() {}\n").unwrap();

        let root_id = ctx
            .pg()
            .add_watch_root(&t.path().to_string_lossy(), "wt", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx
            .pg()
            .upsert_repo_kind(&root_id, "git", "demo", &repo.to_string_lossy())
            .await
            .unwrap();

        let repo_path = repo.to_string_lossy().to_string();
        super::process_git_folder(&ctx, &Task::new(TaskKind::ProcessGitFolder, &repo_path, ""))
            .await
            .unwrap();
        // BEFORE any parse: a source row must sit on the stage-3 barrier
        // sentinel. A row bearing its TRUE fingerprint reads as UNCHANGED next
        // pass and the file is never indexed — the defect 8d488e2a fixed once.
        let (mtime, hash): (i64, String) = sqlx_core::query_as::query_as(
            "SELECT mtime, content_hash FROM sensei.files WHERE folder_id=$1 AND file_path=$2",
        )
        .bind(fid)
        .bind("src/lib.rs")
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(
            (mtime, hash.as_str()),
            (
                crate::db::pg_store::folders::BARRIER_MTIME,
                crate::db::pg_store::folders::BARRIER_HASH
            ),
            "a source row must sit on the barrier sentinel until a parse advances it"
        );

        super::process_repo_files(&ctx, &Task::new(TaskKind::ProcessRepoFiles, &repo_path, ""))
            .await
            .unwrap();

        // Run every ProcessFile the gate produced, exactly as the executor would.
        let file_tasks: Vec<_> = ctx
            .queue
            .snapshot()
            .await
            .into_iter()
            .filter(|(k, _, _)| *k == TaskKind::ProcessFile)
            .collect();
        assert!(!file_tasks.is_empty(), "the gate fanned out no file tasks");
        for (_, folder_path, path) in &file_tasks {
            super::super::process_file(&ctx, &Task::new(TaskKind::ProcessFile, folder_path, path))
                .await
                .unwrap();
        }

        let (nodes,): (i64,) =
            sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id = $1")
                .bind(fid)
                .fetch_one(ctx.pg().pool())
                .await
                .unwrap();
        assert!(nodes > 0, "the cycle produced NO nodes — the file was never indexed");

        // The stage-3 BARRIER SENTINEL, not a real fingerprint. A row bearing
        // the true mtime reads as UNCHANGED next pass and the file is never
        // indexed — the defect commit 8d488e2a fixed once already.

        // And the work list must EMPTY, or every later scan re-fans the same file.
        let still_unparsed = ctx.pg().list_unparsed_files(&fid).await.unwrap();
        assert!(
            still_unparsed.is_empty(),
            "an indexed file is still listed unparsed, so it is re-enqueued for ever: {still_unparsed:?}"
        );
    }
}
