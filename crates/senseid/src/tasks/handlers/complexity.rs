//! Measure a repository's function complexity with local qlty (Code map,
//! Complexity lens; user, 2026-10-27: rely on local qlty). At HEAD, in a
//! temporary worktree against the pinned ruler; again only when HEAD moved.

use crate::tasks::Task;
use crate::tasks::executor::TaskContext;

/// `folder_path` is the repository's root on disk; `path` its repository id.
pub async fn measure_complexity(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let repo = uuid::Uuid::parse_str(&task.path)
        .map_err(|_| format!("measure_complexity: not a repository id: {}", task.path))?;
    let root = task.folder_path.clone();
    measure_with(
        ctx.pg(),
        &repo,
        &root,
        |root| {
            crate::tasks::handlers::metrics::churn::run_git(root, &["rev-parse", "HEAD"])
                .map(|s| s.trim().to_string())
        },
        |root| {
            let root = root.to_string();
            async move {
                tokio::task::spawn_blocking(move || crate::qlty::measure_head(&root))
                    .await
                    .map_err(|e| format!("measure_complexity: {e}"))?
            }
        },
    )
    .await
}

pub(crate) async fn measure_with<H, M, Fut>(
    pg: &crate::db::pg_store::PgStore,
    repo: &uuid::Uuid,
    root: &str,
    head: H,
    measure: M,
) -> Result<u32, String>
where
    H: Fn(&str) -> Option<String>,
    M: Fn(&str) -> Fut,
    Fut: std::future::Future<
            Output = Result<
                Option<std::collections::HashMap<String, crate::qlty::FileComplexity>>,
                String,
            >,
        >,
{
    let sha = head(root).ok_or_else(|| format!("measure_complexity: no HEAD at {root}"))?;
    if pg.complexity_sha(repo).await?.as_deref() == Some(sha.as_str()) {
        return Ok(0); // HEAD has not moved since the last measurement
    }
    let Some(files) = measure(root).await? else {
        tracing::info!(root, "measure_complexity: qlty not available — nothing measured");
        return Ok(0);
    };
    pg.replace_file_complexity(repo, &sha, &files).await?;
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qlty::FileComplexity;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn complexity_is_measured_again_only_when_head_moved() {
        let pg = crate::db::pg_store::PgStore::connect_test().await.unwrap();
        let repo = pg
            .upsert_repository(&format!("cx-{}", uuid::Uuid::new_v4().simple()), None)
            .await
            .unwrap();
        let runs = AtomicU32::new(0);
        let measure = |_root: &str| {
            runs.fetch_add(1, Ordering::SeqCst);
            async {
                Ok(Some(
                    [(
                        "src/a.rs".to_string(),
                        FileComplexity { functions: 3, complex: 1, max_cyclo: 12 },
                    )]
                    .into_iter()
                    .collect(),
                ))
            }
        };
        let at = |sha: &'static str| move |_root: &str| Some(sha.to_string());

        assert_eq!(measure_with(&pg, &repo, "/tmp/r", at("aaa"), measure).await.unwrap(), 1);
        assert_eq!(measure_with(&pg, &repo, "/tmp/r", at("aaa"), measure).await.unwrap(), 0);
        assert_eq!(runs.load(Ordering::SeqCst), 1, "HEAD did not move: not measured again");
        assert_eq!(measure_with(&pg, &repo, "/tmp/r", at("bbb"), measure).await.unwrap(), 1);
        assert_eq!(pg.complexity_sha(&repo).await.unwrap().as_deref(), Some("bbb"));

        // qlty absent: nothing recorded, and not an error.
        let absent = |_root: &str| async { Ok(None) };
        assert_eq!(measure_with(&pg, &repo, "/tmp/r", at("ccc"), absent).await.unwrap(), 0);
        assert_eq!(pg.complexity_sha(&repo).await.unwrap().as_deref(), Some("bbb"));
        // No HEAD is a failure, said.
        assert!(measure_with(&pg, &repo, "/tmp/r", |_: &str| None, absent).await.is_err());
    }
}
