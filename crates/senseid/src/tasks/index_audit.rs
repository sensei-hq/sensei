//! Index integrity self-audit (P2 — the "rock-solid indexing" assurance layer).
//!
//! The reconcile scheduler ([`crate::tasks::reconcile_scheduler`]) keeps the
//! index CONVERGING to the filesystem cheaply and frequently. This audit is the
//! next layer up: it GENERALIZES the point-fix self-heals that were added for the
//! 2026-07-13 drift incident into one continuous invariant checker, so drift
//! classes we *haven't* hit yet get caught (and, in repair mode, fixed)
//! automatically instead of lingering until a human notices.
//!
//! It is a pure ORCHESTRATOR — it reuses the existing detection + repair
//! primitives, never re-implementing deletion logic:
//!
//! | invariant violated                                   | detect / repair reuse |
//! |------------------------------------------------------|-----------------------|
//! | a node whose `file_path` no longer exists on disk    | [`scan::prune_vanished`] |
//! | a `kind='folder'` row whose directory vanished       | [`scan::detect_vanished_folders`] + [`scan::apply_folder_prune`] |
//! | a `standalone` project nested inside another repo    | [`PgStore::detect_nested_standalone_roots`] / [`PgStore::heal_nested_standalone_roots`] |
//! | duplicate-name phantom projects                      | [`PgStore::detect_duplicate_name_phantoms`] / [`PgStore::heal_duplicate_name_projects`] |
//!
//! Two entry points share [`audit_index_integrity`]:
//! - [`run_audit`]`(pg, true)` — the periodic REPAIR pass, wired into the daemon
//!   at a conservative (daily) cadence gated by the `audit.last_run` watermark. It
//!   stats every indexed file/folder, so it is heavier than the 300s reconcile and
//!   must NOT ride every reconcile tick.
//! - [`run_doctor`] = `run_audit(pg, false)` — the READ-ONLY diagnostic behind
//!   `GET /api/index/doctor` / `sensei index doctor`. It reports the SAME drift it
//!   would repair, mutating nothing.
//!
//! SAFETY: the deletion classes only ever operate under roots confirmed PRESENT on
//! disk (mirroring [`scan::detect_vanished_folders`]'s absent-root guard) — a
//! temporarily-unmounted volume must never cause a live index to be pruned. When
//! NO watch root is present on disk the audit does nothing at all.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;

use crate::api::util::json_uuid;
use crate::db::pg_store::PgStore;
use crate::tasks::handlers::scan;
use crate::tasks::ticker;

/// `sensei.config` key holding the last integrity-audit run (epoch millis). The
/// watermark GATES the pass (unlike the boot-always reconcile) so frequent
/// restarts can't re-run the heavier sweep more than once per interval.
const LAST_RUN_KEY: &str = "audit.last_run";

/// At most this many sample paths/ids per drift class in the report — enough to
/// make a doctor report actionable without dumping a whole tree.
const SAMPLE_CAP: usize = 5;

/// A watch root confirmed PRESENT on disk. The audit's deletion classes operate
/// only under these (unmounted-volume safety).
#[derive(Debug, Clone)]
pub struct WatchRootRef {
    pub id: uuid::Uuid,
    pub path: String,
}

/// A few example paths/ids per drift class, for the doctor report.
#[derive(Debug, Default, Clone, Serialize)]
pub struct AuditSamples {
    /// Absolute paths of indexed files that vanished on disk.
    pub orphan_files: Vec<String>,
    /// Absolute paths of `kind='folder'` rows whose directory vanished.
    pub ghost_folders: Vec<String>,
    /// Absolute paths of standalone roots mis-scoped inside a git repo.
    pub nested_standalone: Vec<String>,
    /// Ids of duplicate-name phantom projects.
    pub duplicate_name_projects: Vec<String>,
}

impl AuditSamples {
    /// Push into a per-class sample bucket up to [`SAMPLE_CAP`], ignoring the rest.
    fn extend_capped<I: IntoIterator<Item = String>>(dst: &mut Vec<String>, items: I) {
        for it in items {
            if dst.len() >= SAMPLE_CAP {
                break;
            }
            dst.push(it);
        }
    }
}

/// The outcome of one integrity audit — per-class drift counts (what was, or in
/// read-only mode WOULD be, repaired) plus a few samples. Serialized as-is by the
/// `/api/index/doctor` endpoint.
#[derive(Debug, Default, Clone, Serialize)]
pub struct AuditReport {
    /// True when this was a repair pass; false for the read-only doctor.
    pub repair: bool,
    /// Watch roots examined in total (present + absent).
    pub roots_checked: u32,
    /// Watch roots confirmed present on disk (the only ones operated under).
    pub roots_present: u32,
    /// Watch roots skipped because their directory is absent (e.g. unmounted).
    pub roots_absent: u32,
    /// Indexed files whose file no longer exists on disk (nodes pruned / to prune).
    pub orphan_files: u64,
    /// `kind='folder'` rows whose directory vanished (subtrees pruned / to prune).
    pub ghost_folders: u64,
    /// Standalone roots mis-scoped inside a git repo (re-absorbed / to re-absorb).
    pub nested_standalone: u64,
    /// Duplicate-name phantom projects (merged / to merge).
    pub duplicate_name_projects: u64,
    /// A few example paths/ids per class.
    pub samples: AuditSamples,
}

impl AuditReport {
    /// True when any drift class has a non-zero count — the index is not clean.
    pub fn has_drift(&self) -> bool {
        self.orphan_files > 0
            || self.ghost_folders > 0
            || self.nested_standalone > 0
            || self.duplicate_name_projects > 0
    }
}

/// Check (and, when `repair`, fix) the index invariants under the given
/// confirmed-present watch roots. READ-ONLY when `repair` is false — it returns
/// the same per-class report of what it WOULD repair without mutating anything.
///
/// Orchestrator only: every mutation delegates to an existing primitive (see the
/// module docs). When `present_roots` is empty it returns an all-zero report and
/// touches nothing (nothing is confirmed on disk to operate under).
pub async fn audit_index_integrity(
    pg: &PgStore,
    present_roots: &[WatchRootRef],
    repair: bool,
) -> AuditReport {
    let mut report =
        AuditReport { repair, roots_present: present_roots.len() as u32, ..Default::default() };
    if present_roots.is_empty() {
        // Nothing confirmed present on disk — never operate (unmounted-volume
        // safety). The global structural heals are gated on this too.
        return report;
    }

    // ── Per-root, disk-touching classes ──────────────────────────────────────
    for root in present_roots {
        let folders = pg.list_folders_by_root(&root.id).await.unwrap_or_else(|e| {
            tracing::warn!(root = %root.path, error = %e, "index_audit: list_folders_by_root failed; skipping root");
            Vec::new()
        });

        // Class 1 — orphan nodes: an indexed file that vanished on disk. We stat
        // each indexed file (the invariant check) and delegate the actual node
        // deletion to `prune_vanished`, which drops any indexed path NOT in the
        // live set — so the live set is every indexed file NOT confirmed absent.
        for f in &folders {
            // Only project roots own file nodes (repo-relative to their abs_path),
            // exactly as `process_git_folder` calls `prune_vanished`.
            if !matches!(f["kind"].as_str().unwrap_or(""), "git" | "standalone" | "subtree") {
                continue;
            }
            let Some(abs) = f["abs_path"].as_str() else { continue };
            // A project-root folder whose own dir is absent (moved repo, unmounted)
            // is left entirely alone — statting its files would mark them all
            // absent and mass-prune a live index. `reconcile_roots` owns absent roots.
            if !scan::dir_present(Path::new(abs)) {
                continue;
            }
            let Some(fid) = json_uuid(&f["id"]) else { continue };
            let indexed = pg.list_indexed_files(&fid).await.unwrap_or_else(|e| {
                tracing::warn!(folder = %fid, error = %e, "index_audit: list_indexed_files failed");
                Vec::new()
            });
            let mut live: HashSet<String> = HashSet::with_capacity(indexed.len());
            let mut absent: Vec<String> = Vec::new();
            for rel in indexed {
                if scan::dir_present(&Path::new(abs).join(&rel)) {
                    live.insert(rel);
                } else {
                    absent.push(format!("{abs}/{rel}"));
                }
            }
            if !absent.is_empty() {
                report.orphan_files += absent.len() as u64;
                AuditSamples::extend_capped(&mut report.samples.orphan_files, absent);
                if repair {
                    let pruned = scan::prune_vanished(pg, &fid, &live).await;
                    tracing::info!(folder = %fid, pruned, "index_audit: pruned orphan nodes for vanished files");
                }
            }
        }

        // Class 2 — ghost folders: a `kind='folder'` row whose directory vanished.
        // Reuse the exact scan-reconcile detection (already absent-root-guarded),
        // and repair via the shared apply half.
        let ghosts = scan::detect_vanished_folders(pg, &root.id).await;
        if !ghosts.is_empty() {
            report.ghost_folders += ghosts.len() as u64;
            AuditSamples::extend_capped(
                &mut report.samples.ghost_folders,
                ghosts.iter().map(|(_, abs)| abs.clone()),
            );
            if repair {
                let pruned = scan::apply_folder_prune(pg, &ghosts).await;
                tracing::info!(root = %root.path, pruned, "index_audit: pruned ghost folder subtrees");
            }
        }
    }

    // ── Global, structural classes (gated on ≥1 present root above) ───────────
    // These are pure DB-structural re-attributions; they run once, not per root.

    // Class 3 — a standalone project root nested inside another repo.
    match pg.detect_nested_standalone_roots().await {
        Ok(paths) => {
            report.nested_standalone = paths.len() as u64;
            AuditSamples::extend_capped(&mut report.samples.nested_standalone, paths);
        }
        Err(e) => tracing::warn!(error = %e, "index_audit: detect_nested_standalone_roots failed"),
    }
    if repair && report.nested_standalone > 0 {
        match pg.heal_nested_standalone_roots().await {
            Ok(n) => tracing::info!(healed = n, "index_audit: re-absorbed nested standalone roots"),
            Err(e) => {
                tracing::warn!(error = %e, "index_audit: heal_nested_standalone_roots failed")
            }
        }
    }

    // Class 4 — duplicate-name phantom projects. Detected after the nested-heal so
    // the count reflects the phantoms that heal itself did not already fold away.
    match pg.detect_duplicate_name_phantoms().await {
        Ok(ids) => {
            report.duplicate_name_projects = ids.len() as u64;
            AuditSamples::extend_capped(
                &mut report.samples.duplicate_name_projects,
                ids.iter().map(|id| id.to_string()),
            );
        }
        Err(e) => tracing::warn!(error = %e, "index_audit: detect_duplicate_name_phantoms failed"),
    }
    if repair && report.duplicate_name_projects > 0 {
        match pg.heal_duplicate_name_projects().await {
            Ok(n) => {
                tracing::info!(healed = n, "index_audit: merged duplicate-name phantom projects")
            }
            Err(e) => {
                tracing::warn!(error = %e, "index_audit: heal_duplicate_name_projects failed")
            }
        }
    }

    report
}

/// List the watch roots and partition them by disk presence, returning only the
/// present ones. The absent ones are counted (never operated under).
async fn partition_present_roots(pg: &PgStore) -> (Vec<WatchRootRef>, u32, u32) {
    let all = pg.list_watch_roots().await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "index_audit: list_watch_roots failed; auditing no roots");
        Vec::new()
    });
    let total = all.len() as u32;
    let mut present = Vec::new();
    let mut absent = 0u32;
    for r in &all {
        let Some(path) = r["path"].as_str() else { continue };
        let Some(id) = json_uuid(&r["id"]) else { continue };
        if scan::dir_present(Path::new(path)) {
            present.push(WatchRootRef { id, path: path.to_string() });
        } else {
            absent += 1;
        }
    }
    (present, total, absent)
}

/// Run a full integrity audit over every watch root present on disk. `repair`
/// selects the mutating pass (periodic scheduler) vs the read-only doctor.
pub async fn run_audit(pg: &PgStore, repair: bool) -> AuditReport {
    let (present, total, absent) = partition_present_roots(pg).await;
    let mut report = audit_index_integrity(pg, &present, repair).await;
    report.roots_checked = total;
    report.roots_absent = absent;
    report
}

/// Read-only diagnostic: report drift without repairing. Backs
/// `GET /api/index/doctor` and `sensei index doctor`.
pub async fn run_doctor(pg: &PgStore) -> AuditReport {
    run_audit(pg, false).await
}

/// Spawn the periodic repair audit for the daemon's lifetime.
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(run(pg));
}

async fn run(pg: Arc<PgStore>) {
    // Cadence lives in `sensei.schedules` (name `index_audit`).
    //
    // The old loop GATED itself on a `LAST_RUN_KEY` watermark to avoid
    // re-auditing after a quick restart. `should_run` now answers exactly that
    // question from `schedules.last_run_at`, so the hand-rolled gate is gone
    // rather than kept as a second source of truth.
    //
    // The watermark is still WRITTEN, but nothing reads it any more:
    // `GET /api/tasks/scheduled` now shows `schedules.last_run_at` (step 4 of
    // the schedules spec). The write is a no-op cost kept out of this slice —
    // retiring it is a separate change, along with the key itself.
    let store = pg.clone();
    ticker::run_scheduled(pg, "index_audit", move || {
        let pg = store.clone();
        async move {
            audit_pass(&pg).await;
            Ok(())
        }
    })
    .await;
}

/// One audit + repair pass.
async fn audit_pass(pg: &PgStore) {
    let report = run_audit(pg, true).await;
    // Non-fatal: the audit happened; only the visibility watermark failed.
    let now_ms = Utc::now().timestamp_millis();
    if let Err(e) = pg.set_config(LAST_RUN_KEY, &now_ms.to_string()).await {
        tracing::warn!(error = %e, "index_audit: persisting last_run watermark failed");
    }
    if report.has_drift() {
        tracing::info!(
            orphan_files = report.orphan_files,
            ghost_folders = report.ghost_folders,
            nested_standalone = report.nested_standalone,
            duplicate_name_projects = report.duplicate_name_projects,
            roots_present = report.roots_present,
            roots_absent = report.roots_absent,
            "index_audit: repaired index drift",
        );
    } else {
        tracing::debug!("index_audit: no drift");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_capped() {
        let mut v = Vec::new();
        AuditSamples::extend_capped(&mut v, (0..100).map(|i| i.to_string()));
        assert_eq!(v.len(), SAMPLE_CAP, "sample bucket must cap at SAMPLE_CAP");
    }

    // ── DB-gated integrity tests (sensei_test — never prod) ───────────────────
    //
    // Each seeds a specific drift class under a UNIQUE temp watch root and runs
    // `audit_index_integrity` with an explicit `present_roots` = just that root,
    // isolating the per-root classes from the shared test DB's other rows.

    /// Build a `WatchRootRef` for an on-disk temp dir + register it as a watch root.
    async fn present_root(pg: &PgStore, dir: &std::path::Path) -> (WatchRootRef, uuid::Uuid) {
        let path = dir.to_string_lossy().to_string();
        let id = pg.add_watch_root(&path, "audit_test", &serde_json::json!([])).await.unwrap();
        (WatchRootRef { id, path }, id)
    }

    #[tokio::test]
    async fn audit_repairs_orphan_nodes() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        std::fs::write(repo.join("live.rs"), "fn a() {}\n").unwrap();

        let (root, root_id) = present_root(&pg, repo).await;
        let fid =
            pg.upsert_repo_kind(&root_id, "git", "repo", &repo.to_string_lossy()).await.unwrap();
        // live.rs exists on disk; gone.rs does not.
        pg.upsert_node(&fid, "function", "a", "live.rs", None, None, None, None).await.unwrap();
        pg.upsert_node(&fid, "struct", "Gone", "gone.rs", None, None, None, None).await.unwrap();

        // READ-ONLY first: detects the orphan without mutating.
        let doctor = audit_index_integrity(&pg, std::slice::from_ref(&root), false).await;
        assert_eq!(doctor.orphan_files, 1, "doctor detects the vanished file");
        assert!(
            pg.list_indexed_files(&fid).await.unwrap().iter().any(|p| p == "gone.rs"),
            "read-only doctor must NOT prune"
        );

        // REPAIR: prunes the orphan, keeps the live node.
        let rep = audit_index_integrity(&pg, std::slice::from_ref(&root), true).await;
        assert_eq!(rep.orphan_files, 1);
        let files = pg.list_indexed_files(&fid).await.unwrap();
        assert!(files.contains(&"live.rs".to_string()), "live file kept");
        assert!(!files.contains(&"gone.rs".to_string()), "vanished file pruned");

        // IDEMPOTENT: re-run finds nothing.
        assert_eq!(
            audit_index_integrity(&pg, std::slice::from_ref(&root), true).await.orphan_files,
            0
        );

        pg.remove_watch_root(&root_id).await.unwrap();
    }

    #[tokio::test]
    async fn audit_repairs_ghost_folders() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        let (root, root_id) = present_root(&pg, tmp.path()).await;
        let repo_fid =
            pg.upsert_repo_kind(&root_id, "git", "repo", &repo.to_string_lossy()).await.unwrap();
        // Live subfolder (dir present) vs ghost subfolder (dir absent, has a node).
        let live_dir = repo.join("live");
        std::fs::create_dir_all(&live_dir).unwrap();
        pg.upsert_subfolder(
            &root_id,
            "live",
            "live",
            &live_dir.to_string_lossy(),
            Some(&repo_fid),
            None,
        )
        .await
        .unwrap();
        let gone = repo.join("gone"); // never created on disk
        let gone_fid = pg
            .upsert_subfolder(
                &root_id,
                "gone",
                "gone",
                &gone.to_string_lossy(),
                Some(&repo_fid),
                None,
            )
            .await
            .unwrap();
        pg.upsert_node(&gone_fid, "struct", "Ghost", "gone/x.rs", None, None, None, None)
            .await
            .unwrap();

        // READ-ONLY: detects the ghost, leaves the row.
        let doctor = audit_index_integrity(&pg, std::slice::from_ref(&root), false).await;
        assert_eq!(doctor.ghost_folders, 1, "doctor detects the ghost folder");
        assert!(
            pg.list_folders_by_root(&root_id)
                .await
                .unwrap()
                .iter()
                .any(|r| r["abs_path"].as_str() == Some(gone.to_string_lossy().as_ref())),
            "read-only doctor must NOT delete the ghost row"
        );

        // REPAIR: prunes the ghost, keeps the live subfolder.
        assert_eq!(
            audit_index_integrity(&pg, std::slice::from_ref(&root), true).await.ghost_folders,
            1
        );
        let abs: HashSet<String> = pg
            .list_folders_by_root(&root_id)
            .await
            .unwrap()
            .iter()
            .filter_map(|r| r["abs_path"].as_str().map(String::from))
            .collect();
        assert!(!abs.contains(&gone.to_string_lossy().to_string()), "ghost folder pruned");
        assert!(abs.contains(&live_dir.to_string_lossy().to_string()), "live subfolder kept");

        // IDEMPOTENT.
        assert_eq!(
            audit_index_integrity(&pg, std::slice::from_ref(&root), true).await.ghost_folders,
            0
        );

        pg.remove_watch_root(&root_id).await.unwrap();
    }

    #[tokio::test]
    async fn audit_repairs_nested_standalone() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("monorepo");
        let nested = repo.join("crates").join("sub");
        std::fs::create_dir_all(&nested).unwrap();

        let (root, root_id) = present_root(&pg, tmp.path()).await;
        // A git repo with its own project.
        let gpid = pg
            .create_project(&format!("mono-{}", uuid::Uuid::new_v4().simple()), None, None)
            .await
            .unwrap();
        pg.upsert_folder(
            &root_id,
            "git",
            "monorepo",
            "monorepo",
            &repo.to_string_lossy(),
            None,
            Some(&gpid),
        )
        .await
        .unwrap();
        // A standalone root mis-scoped INSIDE the repo, attributed to a DIFFERENT project.
        let spid = pg
            .create_project(&format!("sub-{}", uuid::Uuid::new_v4().simple()), None, None)
            .await
            .unwrap();
        let s_fid = pg
            .upsert_folder(
                &root_id,
                "standalone",
                "sub",
                "sub",
                &nested.to_string_lossy(),
                None,
                Some(&spid),
            )
            .await
            .unwrap();

        // Detection is confirmed via the deterministic doctor test on the
        // root-scoped classes; here we assert the REPAIR OUTCOME on our own unique
        // rows. `heal_nested_standalone_roots` is GLOBAL (every `scan_root`
        // reconcile runs it too), so a concurrent pass may re-absorb our row first
        // — but after a repair pass MY nested standalone is guaranteed re-absorbed.
        audit_index_integrity(&pg, std::slice::from_ref(&root), true).await;
        let (kind, pid): (String, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
            "SELECT kind::text, project_id FROM sensei.folders WHERE id = $1",
        )
        .bind(s_fid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            kind, "folder",
            "nested standalone re-absorbed as a folder of the enclosing repo"
        );
        assert_eq!(pid, Some(gpid), "re-absorbed folder re-pointed to the repo's project");

        pg.remove_watch_root(&root_id).await.unwrap();
    }

    #[tokio::test]
    async fn audit_repairs_duplicate_name_projects() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();

        let (root, root_id) = present_root(&pg, tmp.path()).await;
        let name = format!("dupname-{}", uuid::Uuid::new_v4().simple());
        // Survivor: a folder-bearing project.
        let survivor = pg.create_project(&name, None, None).await.unwrap();
        pg.upsert_folder(
            &root_id,
            "git",
            "repo",
            "repo",
            &repo.to_string_lossy(),
            None,
            Some(&survivor),
        )
        .await
        .unwrap();
        // Phantom: a same-name, 0-folder, discovery project.
        let phantom = pg.create_project(&name, None, None).await.unwrap();

        // Assert the REPAIR OUTCOME on our own uniquely-named rows.
        // `heal_duplicate_name_projects` is GLOBAL, but the name is unique to this
        // test, so after a repair pass MY phantom is guaranteed merged away (by
        // this pass or a concurrent one) while the folder-bearing survivor stays.
        audit_index_integrity(&pg, std::slice::from_ref(&root), true).await;
        assert!(pg.get_project(&phantom).await.unwrap().is_none(), "phantom merged away");
        assert!(pg.get_project(&survivor).await.unwrap().is_some(), "survivor kept");

        pg.remove_watch_root(&root_id).await.unwrap();
    }

    #[tokio::test]
    async fn empty_present_roots_is_a_no_op() {
        let pg = PgStore::connect_test().await.unwrap();
        // No present roots → nothing operated under (unmounted-volume safety),
        // even if the DB elsewhere has structural drift.
        let report = audit_index_integrity(&pg, &[], true).await;
        assert_eq!(report.roots_present, 0);
        assert!(!report.has_drift(), "an empty present-root set repairs nothing");
    }

    /// Convergence / chaos stand-in: real FSEvents-drop + mid-scan-restart chaos
    /// isn't unit-testable, so we SIMULATE the DB state such chaos leaves — MULTIPLE
    /// drift classes at once under one root — then assert the repair audit drives
    /// the index invariant-clean, and a second pass is a no-op (idempotent).
    #[tokio::test]
    async fn convergence_repairs_multiple_drift_classes_at_once() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("monorepo");
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("live.rs"), "fn a() {}\n").unwrap();

        let (root, root_id) = present_root(&pg, tmp.path()).await;

        // (a) git repo with a project + a present + vanished FILE node (orphan class).
        let gpid = pg
            .create_project(&format!("mono-{}", uuid::Uuid::new_v4().simple()), None, None)
            .await
            .unwrap();
        let repo_fid = pg
            .upsert_folder(
                &root_id,
                "git",
                "monorepo",
                "monorepo",
                &repo.to_string_lossy(),
                None,
                Some(&gpid),
            )
            .await
            .unwrap();
        pg.upsert_node(&repo_fid, "function", "a", "live.rs", None, None, None, None)
            .await
            .unwrap();
        pg.upsert_node(&repo_fid, "struct", "Gone", "gone.rs", None, None, None, None)
            .await
            .unwrap();

        // (b) ghost subfolder (dir absent) with a node (ghost-folder class).
        let ghost_dir = repo.join("ghost"); // never created on disk
        let ghost_fid = pg
            .upsert_subfolder(
                &root_id,
                "ghost",
                "ghost",
                &ghost_dir.to_string_lossy(),
                Some(&repo_fid),
                None,
            )
            .await
            .unwrap();
        pg.upsert_node(&ghost_fid, "struct", "Ghost", "ghost/x.rs", None, None, None, None)
            .await
            .unwrap();

        // (c) standalone root mis-scoped inside the repo (nested-standalone class).
        let nested = repo.join("crates").join("sub");
        std::fs::create_dir_all(&nested).unwrap();
        let spid = pg
            .create_project(&format!("sub-{}", uuid::Uuid::new_v4().simple()), None, None)
            .await
            .unwrap();
        pg.upsert_folder(
            &root_id,
            "standalone",
            "sub",
            "sub",
            &nested.to_string_lossy(),
            None,
            Some(&spid),
        )
        .await
        .unwrap();

        // (d) duplicate-name phantom project (duplicate-name class).
        let dupname = format!("dupname-{}", uuid::Uuid::new_v4().simple());
        let survivor = pg.create_project(&dupname, None, None).await.unwrap();
        let extra_repo = tmp.path().join("dup");
        std::fs::create_dir_all(&extra_repo).unwrap();
        pg.upsert_folder(
            &root_id,
            "git",
            "dup",
            "dup",
            &extra_repo.to_string_lossy(),
            None,
            Some(&survivor),
        )
        .await
        .unwrap();
        let phantom = pg.create_project(&dupname, None, None).await.unwrap();

        // One repair pass fixes every class it can see. The per-root classes
        // (orphan/ghost) are scoped to THIS unique root, so their counts are
        // deterministic; the global classes (nested/duplicate) are asserted by
        // outcome below (their counts race with concurrent global heals).
        let rep = audit_index_integrity(&pg, std::slice::from_ref(&root), true).await;
        assert_eq!(rep.orphan_files, 1, "orphan file detected+pruned");
        assert_eq!(rep.ghost_folders, 1, "ghost folder detected+pruned");

        // Invariant-clean afterwards — assert on THIS root's own rows (the shared
        // DB may hold unrelated structural rows from concurrent tests, so we check
        // our constructed drift converged rather than global zero).
        let files = pg.list_indexed_files(&repo_fid).await.unwrap();
        assert!(
            files.contains(&"live.rs".to_string()) && !files.contains(&"gone.rs".to_string()),
            "orphan gone, live kept"
        );
        let abs: HashSet<String> = pg
            .list_folders_by_root(&root_id)
            .await
            .unwrap()
            .iter()
            .filter_map(|r| r["abs_path"].as_str().map(String::from))
            .collect();
        assert!(!abs.contains(&ghost_dir.to_string_lossy().to_string()), "ghost folder gone");
        assert_eq!(
            pg.get_repo_by_path(&nested.to_string_lossy())
                .await
                .unwrap()
                .map(|r| r["kind"].as_str().unwrap_or("").to_string()),
            Some("folder".to_string()),
            "nested standalone now a folder of the repo"
        );
        assert!(pg.get_project(&phantom).await.unwrap().is_none(), "phantom merged");

        // A second full audit over this root is a no-op for the per-root classes.
        let again = audit_index_integrity(&pg, std::slice::from_ref(&root), true).await;
        assert_eq!(again.orphan_files, 0, "no orphan files remain");
        assert_eq!(again.ghost_folders, 0, "no ghost folders remain");

        pg.remove_watch_root(&root_id).await.unwrap();
    }

    #[tokio::test]
    async fn run_audit_counts_present_and_absent_roots() {
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let present = tmp.path().join("present");
        std::fs::create_dir_all(&present).unwrap();
        let absent = tmp.path().join("absent"); // never created on disk

        let pid = pg
            .add_watch_root(&present.to_string_lossy(), "present", &serde_json::json!([]))
            .await
            .unwrap();
        let aid = pg
            .add_watch_root(&absent.to_string_lossy(), "absent", &serde_json::json!([]))
            .await
            .unwrap();

        // run_audit sweeps ALL watch roots (shared DB), so assert on the totals it
        // must reflect our two contribute to, not an exact global count.
        let report = run_audit(&pg, false).await;
        assert!(!report.repair, "doctor pass is read-only");
        assert!(report.roots_checked >= 2, "both our roots are checked");
        assert!(report.roots_present >= 1, "the present root is counted present");
        assert!(report.roots_absent >= 1, "the absent root is counted absent");

        pg.remove_watch_root(&pid).await.unwrap();
        pg.remove_watch_root(&aid).await.unwrap();
    }
}
