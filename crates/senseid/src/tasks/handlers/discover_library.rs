//! A dependency's library, discovered from its repository (#341).
//!
//! A scanned repo's dependencies lead to their repositories
//! ([`dependency_repos`]); a repository that ships a `sensei.library.json` is a
//! library, and its docs, skills and agents are registered here — read at the
//! ref the dependent is pinned to. The scan only queues the work: the network
//! read happens in this task, never in the scan.

use std::path::{Path, PathBuf};

use crate::libraries::dependencies::{DependencyRepo, dependency_repos, read_remote_manifest};
use crate::tasks::executor::TaskContext;
use crate::tasks::{Task, TaskKind};

/// How long a repository's answer stands before a scan asks it again.
pub const RECHECK_HOURS: i32 = 24;

/// Read the dependency's manifest from its repository and register the library.
pub async fn discover_library(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    discover_with(ctx, task, fetch_text).await
}

/// A URL's body; `None` for a 404 — a repository without a manifest.
async fn fetch_text(url: String) -> Result<Option<String>, String> {
    let client = crate::http::bounded_builder()
        .user_agent("sensei-daemon")
        .build()
        .map_err(|e| format!("HTTP client: {e}"))?;
    let resp = client.get(&url).send().await.map_err(|e| format!("{url}: {e}"))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let resp = resp.error_for_status().map_err(|e| format!("{url}: {e}"))?;
    resp.text().await.map(Some).map_err(|e| format!("{url}: {e}"))
}

pub(crate) async fn discover_with<F, Fut>(
    ctx: &TaskContext,
    task: &Task,
    fetch: F,
) -> Result<u32, String>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Result<Option<String>, String>>,
{
    let payload =
        task.url.as_deref().ok_or("discover_library: the dependency is missing (task.url)")?;
    let dep: DependencyRepo = serde_json::from_str(payload)
        .map_err(|e| format!("discover_library: unreadable dependency {payload}: {e}"))?;
    let Some(m) = read_remote_manifest(&dep, fetch).await? else {
        return Ok(0); // not a library — the common case
    };
    let id = crate::libraries::register_dependency_library(ctx.pg(), &dep, &m).await?;
    // Its docs, through the one docs route: index_library reads a GitHub tree.
    if let Some(path) = m.llms_path.as_deref().filter(|p| !p.contains("..")) {
        let url = format!(
            "https://github.com/{}/{}/tree/{}/{}",
            dep.owner,
            dep.repo,
            dep.git_ref.as_deref().unwrap_or("HEAD"),
            path.trim_matches('/')
        );
        ctx.queue
            .enqueue_unique(
                Task::new(TaskKind::IndexLibrary, &id.to_string(), &m.library).with_url(&url),
            )
            .await;
    }
    tracing::info!(library = %m.library, repository = %dep.url(), "discover_library: registered");
    Ok(1)
}

/// Queue a discovery for each repository a repo's dependencies come from —
/// except one whose library is held locally (its own scan registers it, from
/// the working copy) and one already asked within [`RECHECK_HOURS`].
pub(crate) async fn enqueue_dependency_discovery(
    ctx: &TaskContext,
    repo_root: &Path,
    package_jsons: &[PathBuf],
) -> Result<u32, String> {
    let deps = dependency_repos(repo_root, package_jsons);
    if deps.is_empty() {
        return Ok(0);
    }
    let keys: Vec<(String, String)> = deps.iter().map(|d| (d.url(), d.key())).collect();
    let settled = ctx.pg().dependency_repos_settled(&keys, RECHECK_HOURS).await?;
    let mut queued = 0;
    for dep in deps.iter().filter(|d| !settled.contains(&d.key())) {
        let payload = serde_json::to_string(dep).map_err(|e| format!("discover_library: {e}"))?;
        let task = Task::new(TaskKind::DiscoverLibrary, &dep.url(), &dep.key()).with_url(&payload);
        if ctx.queue.enqueue_unique(task).await.is_some() {
            queued += 1;
        }
    }
    Ok(queued)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::make_ctx;

    fn dep(owner: &str, repo: &str, packages: &[&str]) -> DependencyRepo {
        DependencyRepo {
            owner: owner.into(),
            repo: repo.into(),
            git_ref: Some("v2.0.0".into()),
            ecosystem: "npm".into(),
            packages: packages.iter().map(|p| p.to_string()).collect(),
        }
    }

    fn task_for(d: &DependencyRepo) -> Task {
        Task::new(TaskKind::DiscoverLibrary, &d.url(), &d.key())
            .with_url(&serde_json::to_string(d).unwrap())
    }

    /// A dependency whose repository ships a manifest becomes a library with
    /// its skills, its packages — the ones the manifest names and the ones the
    /// dependent uses — its repository, and its docs queued from the pinned ref.
    #[tokio::test]
    async fn a_dependencys_repository_manifest_registers_its_library() {
        let ctx = make_ctx().await;
        let uniq = uuid::Uuid::new_v4().simple().to_string();
        let lib = format!("dlib{}", &uniq[..12]);
        let d = dep("acme", &lib, &[&format!("@{lib}/ui")]);
        let base = format!("https://raw.githubusercontent.com/acme/{lib}/v2.0.0");
        let files: std::collections::HashMap<String, String> = [
            (
                format!("{base}/sensei.library.json"),
                format!(
                    r#"{{"library":"{lib}","version":">=2","documents":"2.0.0",
                        "packages":["@{lib}/core"],"llms":{{"path":"docs/llms"}},
                        "skills":[{{"name":"{lib}","focus":"use","path":"skills/a/SKILL.md"}}]}}"#
                ),
            ),
            (format!("{base}/skills/a/SKILL.md"), "# use it".to_string()),
        ]
        .into_iter()
        .collect();
        let fetch = |url: String| {
            let hit = files.get(&url).cloned();
            async move { Ok(hit) }
        };

        assert_eq!(discover_with(&ctx, &task_for(&d), fetch).await.unwrap(), 1);

        let skills = ctx.pg().list_library_skills(&lib).await.unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0]["body"], "# use it", "the body is read from the same ref");
        let id = ctx.pg().library_id_for(&lib).await.unwrap().expect("registered");
        for pkg in [format!("@{lib}/core"), format!("@{lib}/ui")] {
            assert_eq!(ctx.pg().library_id_for(&pkg).await.unwrap(), Some(id), "{pkg}");
        }
        assert_eq!(
            ctx.pg().library_repository_url(&id).await.unwrap().as_deref(),
            Some(d.url().as_str())
        );
        assert!(
            ctx.queue.has_pending_kind_folder(TaskKind::IndexLibrary, &id.to_string()).await,
            "its docs are indexed from the repository"
        );
    }

    /// No manifest in the repository: nothing is registered, and it is not an error.
    #[tokio::test]
    async fn a_dependency_without_a_manifest_registers_nothing() {
        let ctx = make_ctx().await;
        let uniq = uuid::Uuid::new_v4().simple().to_string();
        let d = dep("acme", &format!("plain{}", &uniq[..12]), &["plain"]);
        let got = discover_with(&ctx, &task_for(&d), |_u: String| async { Ok(None) }).await;
        assert_eq!(got.unwrap(), 0);
        assert_eq!(ctx.pg().library_id_for(&d.repo).await.unwrap(), None);
    }

    /// A scan queues one discovery per dependency repository; a repository
    /// asked today, or whose library is held locally, is not asked again.
    #[tokio::test]
    async fn a_scan_queues_discovery_once_a_day_and_never_for_a_local_library() {
        let ctx = make_ctx().await;
        let uniq = uuid::Uuid::new_v4().simple().to_string();
        let (fresh, asked, local) = (
            format!("fresh{}", &uniq[..10]),
            format!("asked{}", &uniq[..10]),
            format!("local{}", &uniq[..10]),
        );
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let names = [&fresh, &asked, &local];
        let deps: serde_json::Map<String, serde_json::Value> =
            names.iter().map(|n| (n.to_string(), serde_json::json!("^1"))).collect();
        std::fs::write(
            root.join("package.json"),
            serde_json::json!({"dependencies": deps}).to_string(),
        )
        .unwrap();
        for n in names {
            let dir = root.join("node_modules").join(n);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("package.json"),
                format!(r#"{{"repository": "github:acme/{n}"}}"#),
            )
            .unwrap();
        }
        // `asked`: discovered an hour ago.
        let pg = ctx.pg();
        let run = pg
            .start_task_execution(&crate::db::pg_store::TaskExecutionStart {
                task_id: 1,
                parent_task_id: None,
                task_kind: "discover_library",
                folder_path: &format!("https://github.com/acme/{asked}"),
                path: &format!("acme/{asked}"),
                retry_number: 0,
            })
            .await
            .unwrap();
        pg.complete_task_execution(&run, 0, 10).await.unwrap();
        // `local`: a library this machine scans from its own working copy.
        let id = pg
            .upsert_library(&crate::db::pg_store::LibraryRow {
                name: &local,
                ecosystem: "npm",
                version: None,
                description: None,
                source_type: None,
                base_url: None,
            })
            .await
            .unwrap();
        pg.set_library_local_path(&id, "/tmp/local-checkout").await.unwrap();
        pg.set_library_repository_url(&id, &format!("https://github.com/acme/{local}"))
            .await
            .unwrap();

        let queued =
            enqueue_dependency_discovery(&ctx, root, &[root.join("package.json")]).await.unwrap();

        assert_eq!(queued, 1, "only the repository nobody has asked");
        let fresh_url = format!("https://github.com/acme/{fresh}");
        assert!(ctx.queue.has_pending_kind_folder(TaskKind::DiscoverLibrary, &fresh_url).await);
    }
}
