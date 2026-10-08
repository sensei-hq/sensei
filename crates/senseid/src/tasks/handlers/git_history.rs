//! `ScanGitHistory` — walk one checkout's git history into repository facts.
//!
//! Feeds three diagram views that have no other source: Hidden coupling
//! (files changed in the same commits), Ownership (authorship), and
//! Complexity's churn axis.
//!
//! ## What this handler is careful about
//!
//! **It walks the TIP SET**, `--branches --remotes --tags`, never `HEAD` alone
//! and never `--all`. HEAD is one branch, not the repository. `--all` sweeps
//! `refs/stash` and `refs/original` — local debris two clones will never agree
//! on, and in this very repository `refs/original/refs/heads/develop` is a
//! leftover from a history rewrite. On the tip set the checkouts largely AGREE:
//! of four co-keyed pairs measured here, three produce byte-identical sets.
//!
//! **Facts are repository-grain; the cursor is per checkout.** Two checkouts of
//! one repository contribute to one commit set rather than fighting over it,
//! and each remembers its own position.
//!
//! **A repository with no remote is skipped.** `upsert_repository` mints a
//! FRESH row for every keyless repo, so `repository_id` is not stable there and
//! a cursor keyed on it would re-ingest the whole history on every scan. That
//! is an identity bug of its own; this handler declines rather than papering
//! over it, and says so in the log.
//!
//! **A git failure is never a successful scan of zero commits.** That is why
//! `crate::git` returns a `Result` carrying the exit code instead of the
//! `Option` the older helpers return.

use super::super::executor::TaskContext;
use crate::tasks::Task;

/// Walk the tip set of one checkout and store what it finds.
///
/// Returns the number of commit rows newly inserted — not the number walked, so
/// a caller can tell new history from a re-read.
pub async fn scan_git_history(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let repo_path = std::path::Path::new(&task.folder_path);
    if !repo_path.exists() {
        return Err(format!("scan_git_history: path does not exist: {}", task.folder_path));
    }

    let Some(row) = ctx.pg().get_repo_by_path(&task.folder_path).await? else {
        return Err(format!("scan_git_history: no folder row for {}", task.folder_path));
    };
    let Some(folder_id) = crate::api::util::json_uuid(&row["id"]) else {
        return Err(format!("scan_git_history: folder row for {} has no id", task.folder_path));
    };

    // The facts hang off the repository, so without one there is nowhere to put
    // them. This is a real state — the walk runs before the reconcile assigns
    // repositories on a fresh install — and it is reported, not silently zero.
    let Some(repository_id) = ctx.pg().repository_id_for_folder(&folder_id).await? else {
        tracing::info!(
            folder = %folder_id, path = %task.folder_path,
            "scan_git_history: folder carries no repository yet — nothing to key history on"
        );
        return Ok(0);
    };

    // Remote-less repositories get a fresh id per upsert; see the module doc.
    let remote = crate::indexer::pipeline::origin_remote(&task.folder_path);
    if remote.is_none() {
        tracing::info!(
            folder = %folder_id, path = %task.folder_path,
            "scan_git_history: repository has no remote, so its id is not stable — skipping"
        );
        return Ok(0);
    }

    // Where the last walk of THIS checkout stopped. Absent = walk everything.
    let stop_at = ctx.pg().get_commit_scan_tips(&folder_id).await?.unwrap_or_default();
    let args = crate::indexer::git_history::log_args(&stop_at)
        .map_err(|e| format!("scan_git_history: {e}"))?;
    let argv: Vec<&str> = args.iter().map(String::as_str).collect();

    let stdout = crate::git::run_bytes(repo_path, &argv)
        .map_err(|e| format!("scan_git_history: git log failed for {}: {e}", task.folder_path))?;
    let commits = crate::indexer::git_history::parse_log(&stdout)
        .map_err(|e| format!("scan_git_history: parsing {}: {e}", task.folder_path))?;

    let (n_commits, n_touches) = ctx.pg().insert_commits(&repository_id, &commits).await?;

    // The cursor advances only after the facts are committed. The other order
    // would record "I have walked to here" over history that failed to land.
    let tips_args = crate::indexer::git_history::tips_args();
    let tips_argv: Vec<&str> = tips_args.iter().map(String::as_str).collect();
    let tips = match crate::git::run_text(repo_path, &tips_argv) {
        Ok(out) => out.split_whitespace().map(str::to_string).collect::<Vec<_>>(),
        Err(e) => {
            // The facts are already stored and correct; only the optimisation
            // is lost, so the next walk redoes work rather than losing any.
            tracing::warn!(
                folder = %folder_id, error = %e,
                "scan_git_history: could not read tips — cursor not advanced, next walk re-reads"
            );
            return Ok(n_commits as u32);
        }
    };
    ctx.pg().upsert_commit_scan(&folder_id, &repository_id, &tips).await?;

    tracing::info!(
        repository = %repository_id, path = %task.folder_path,
        walked = commits.len(), commits = n_commits, touches = n_touches,
        "scan_git_history: history walked"
    );
    Ok(n_commits as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskKind;
    use crate::tasks::test_support::make_ctx;

    /// A real repository's history lands as repository facts.
    ///
    /// Mutation that must break this test: drop the `insert_commits` call.
    #[tokio::test]
    async fn a_git_history_walk_stores_commits_and_their_touches() {
        let ctx = make_ctx().await;
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("hist");
        std::fs::create_dir_all(&repo).unwrap();
        crate::tasks::test_support::git_init_repo(&repo);
        std::fs::write(repo.join("a.rs"), "fn a() {}\n").unwrap();
        run_git(&repo, &["add", "-A"]);
        run_git(&repo, &["commit", "-m", "first"]);
        std::fs::write(repo.join("b.rs"), "fn b() {}\n").unwrap();
        run_git(&repo, &["add", "-A"]);
        run_git(&repo, &["commit", "-m", "second"]);
        // A remote, because a keyless repository is deliberately skipped.
        run_git(&repo, &["remote", "add", "origin", "https://example.invalid/hist-test.git"]);

        let root_id = ctx
            .pg()
            .add_watch_root(&t.path().to_string_lossy(), "hist-wt", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx
            .pg()
            .upsert_repo_kind(&root_id, "git", "hist", &repo.to_string_lossy())
            .await
            .unwrap();
        let rid = crate::tasks::test_support::give_folder_a_repository(ctx.pg(), &fid, "hist")
            .await
            .unwrap();

        let task = Task::new(TaskKind::ScanGitHistory, &repo.to_string_lossy(), "");
        let inserted = scan_git_history(&ctx, &task).await.unwrap();
        assert_eq!(inserted, 2, "both commits stored");

        // Scoped to THIS repository — the test database is shared (#183).
        let (commits, touches): (i64, i64) = sqlx_core::query_as::query_as(
            "SELECT (SELECT count(*) FROM sensei.commits WHERE repository_id = $1), \
                    (SELECT count(*) FROM sensei.commit_files WHERE repository_id = $1)",
        )
        .bind(rid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(commits, 2, "one row per commit");
        assert_eq!(touches, 2, "a.rs and b.rs");

        // Re-walking is a no-op, not a duplicate: the cursor plus ON CONFLICT.
        let again = scan_git_history(&ctx, &task).await.unwrap();
        assert_eq!(again, 0, "a second walk inserts nothing");
    }

    /// A repository with NO remote is skipped, loudly and without error.
    ///
    /// `upsert_repository` mints a fresh id for every keyless repo, so a cursor
    /// keyed on it would re-ingest everything forever.
    ///
    /// Mutation that must break this test: remove the `remote.is_none()` guard.
    #[tokio::test]
    async fn a_repository_with_no_remote_is_skipped_rather_than_re_ingested() {
        let ctx = make_ctx().await;
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("noremote");
        std::fs::create_dir_all(&repo).unwrap();
        crate::tasks::test_support::git_init_repo(&repo);
        std::fs::write(repo.join("a.rs"), "fn a() {}\n").unwrap();
        run_git(&repo, &["add", "-A"]);
        run_git(&repo, &["commit", "-m", "only"]);

        let root_id = ctx
            .pg()
            .add_watch_root(&t.path().to_string_lossy(), "nr-wt", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx
            .pg()
            .upsert_repo_kind(&root_id, "git", "noremote", &repo.to_string_lossy())
            .await
            .unwrap();
        let rid = crate::tasks::test_support::give_folder_a_repository(ctx.pg(), &fid, "noremote")
            .await
            .unwrap();

        let task = Task::new(TaskKind::ScanGitHistory, &repo.to_string_lossy(), "");
        assert_eq!(scan_git_history(&ctx, &task).await.unwrap(), 0);

        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.commits WHERE repository_id = $1",
        )
        .bind(rid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(n.0, 0, "nothing stored for a repository with no stable identity");
    }

    /// A folder that is not a git repository stores NOTHING, and says why.
    ///
    /// It is refused at the identity guard rather than at the git call: a
    /// non-repository has no remote, so there is no stable `repository_id` to
    /// key history on and the walk never runs. Either way the property under
    /// test is the same one — a refusal is reported as zero stored history,
    /// never as a successful scan that happened to find nothing.
    ///
    /// Mutation that must break this test: return `Ok(1)` from either guard.
    #[tokio::test]
    async fn a_folder_that_is_not_a_git_repository_fails_loudly() {
        let ctx = make_ctx().await;
        let t = tempfile::tempdir().unwrap();
        let plain = t.path().join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(plain.join("a.rs"), "fn a() {}\n").unwrap();

        let root_id = ctx
            .pg()
            .add_watch_root(&t.path().to_string_lossy(), "plain-wt", &serde_json::json!([]))
            .await
            .unwrap();
        let fid = ctx
            .pg()
            .upsert_repo_kind(&root_id, "git", "plain", &plain.to_string_lossy())
            .await
            .unwrap();
        crate::tasks::test_support::give_folder_a_repository(ctx.pg(), &fid, "plain")
            .await
            .unwrap();
        let task = Task::new(TaskKind::ScanGitHistory, &plain.to_string_lossy(), "");
        let stored = scan_git_history(&ctx, &task).await.expect("a refusal is not an error");
        assert_eq!(stored, 0, "a non-repository never reports stored history");

        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.commits c JOIN sensei.folders f \
               ON f.repository_id = c.repository_id WHERE f.id = $1",
        )
        .bind(fid)
        .fetch_one(ctx.pg().pool())
        .await
        .unwrap();
        assert_eq!(n.0, 0, "and writes nothing");
    }

    fn run_git(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_NAME", "T")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "T")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .output()
            .expect("git");
        assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }
}
