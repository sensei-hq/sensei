//! The seeded catalogue of supported tools and this machine's installs (#455).
use super::{AssistantInstallRow, PgStore};
use crate::assistants::catalogue::CatalogueWordRow;

impl PgStore {
    /// Every spelling of every supported tool (`sensei.assistant_catalogue_words`),
    /// one query. A failed read is an error, never an empty catalogue.
    pub async fn catalogue_words(&self) -> Result<Vec<CatalogueWordRow>, String> {
        let rows: Vec<(String, String, String, String, bool, Option<String>)> =
            sqlx_core::query_as::query_as(
                "select word, id, name, kind::text, joins_crew, launch_template \
                   from sensei.assistant_catalogue_words",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("read the assistant catalogue: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(word, id, name, kind, joins_crew, launch_template)| CatalogueWordRow {
                word,
                id,
                name,
                kind,
                joins_crew,
                launch_template,
            })
            .collect())
    }

    /// The catalogue's words as [`crate::assistants::catalogue::CatalogueNames`].
    pub async fn catalogue_names(
        &self,
    ) -> Result<crate::assistants::catalogue::CatalogueNames, String> {
        Ok(crate::assistants::catalogue::CatalogueNames::from_rows(self.catalogue_words().await?))
    }

    /// Upsert one install row per probed tool, resolving each registry id through
    /// the catalogue's words. `configured_at` / `configured_version` move only for
    /// a row with `configured_now` (this sensei just configured it); detection
    /// alone observes `configured` and keeps any earlier configuration's stamp.
    /// Returns the registry ids the catalogue does not know; those are not stored.
    pub async fn upsert_assistant_installs(
        &self,
        rows: &[AssistantInstallRow<'_>],
        sensei_version: &str,
    ) -> Result<Vec<String>, String> {
        let mut unknown = Vec::new();
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for r in rows {
            let &AssistantInstallRow {
                registry_id,
                installed,
                config_path,
                configured,
                configured_now,
            } = r;
            let stored: Option<String> = sqlx_core::query_scalar::query_scalar(
                "insert into sensei.assistant_installs as i
                        (catalogue_id, installed, config_path, configured,
                         configured_at, configured_version, detected_at, modified_at)
                 select w.id, $2, $3, $4,
                        case when $6 then now() end, case when $6 then $5 end, now(), now()
                   from sensei.assistant_catalogue_words w
                  where w.word = $1
                 on conflict (catalogue_id) do update set
                        installed   = excluded.installed,
                        config_path = excluded.config_path,
                        configured  = excluded.configured,
                        configured_at = case when $6 then now() else i.configured_at end,
                        configured_version = case when $6 then $5 else i.configured_version end,
                        detected_at = now(),
                        modified_at = now()
                 returning catalogue_id",
            )
            .bind(registry_id)
            .bind(installed)
            .bind(config_path)
            .bind(configured)
            .bind(sensei_version)
            .bind(configured_now)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("store the install of {registry_id}: {e}"))?;
            if stored.is_none() {
                unknown.push(registry_id.to_string());
            }
        }
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(unknown)
    }

    /// This machine's install rows joined to the catalogue, assistants then
    /// editors, each in catalogue order. A failed read is an error.
    pub async fn list_assistant_installs(&self) -> Result<Vec<serde_json::Value>, String> {
        sqlx_core::query_scalar::query_scalar(
            "select jsonb_build_object(
                    'id', c.id, 'name', c.name, 'kind', c.kind,
                    'installed', i.installed, 'version', i.version, 'path', i.path,
                    'config_path', i.config_path, 'configured', i.configured,
                    'configured_at', i.configured_at, 'configured_version', i.configured_version,
                    'detected_at', i.detected_at)
               from sensei.assistant_installs i
               join sensei.assistant_catalogue c on c.id = i.catalogue_id
              order by c.kind, c.sequence, c.id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("read the assistant installs: {e}"))
    }

    /// Whether no install row exists yet (first run).
    pub async fn assistant_installs_empty(&self) -> Result<bool, String> {
        sqlx_core::query_scalar::query_scalar(
            "select not exists (select 1 from sensei.assistant_installs)",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("read the assistant installs: {e}"))
    }

    /// `GET /relay/catalogue` (#455): the catalogue, this machine's installs and
    /// what sessions named, shaped by [`crate::assistants::catalogue::assemble_catalogue`].
    /// Reads only the database, never the filesystem; any failed read is an error.
    /// `repo` is the local path launch templates resolve against (#513).
    pub async fn relay_catalogue(&self, repo: Option<&str>) -> Result<serde_json::Value, String> {
        let names = self.catalogue_names().await?;
        let installs: Vec<(String, bool, bool, Option<String>)> = sqlx_core::query_as::query_as(
            "select catalogue_id, installed, configured, config_path from sensei.assistant_installs",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("read the assistant installs: {e}"))?;
        let installs: Vec<crate::assistants::catalogue::ToolInstall> = installs
            .into_iter()
            .map(|(id, installed, configured, config_path)| {
                crate::assistants::catalogue::ToolInstall { id, installed, configured, config_path }
            })
            .collect();
        // A session names its assistant (acp_id) and, when it ran in one, its
        // editor (host); a host equal to the acp_id is the same tool, counted once.
        let usage: Vec<(String, Option<chrono::DateTime<chrono::Utc>>, i64)> =
            sqlx_core::query_as::query_as(
                "select word, max(started_at),
                        count(*) filter (where started_at > now() - interval '7 days')
                   from (select acp_id as word, started_at from activity.sessions where acp_id is not null
                         union all
                         select host, started_at from activity.sessions
                          where host is not null and host is distinct from acp_id) named
                  group by word",
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("read session usage: {e}"))?;
        let usage: Vec<crate::assistants::catalogue::WordUsage> =
            usage
                .into_iter()
                .map(|(word, last_used_at, sessions_week)| {
                    crate::assistants::catalogue::WordUsage { word, last_used_at, sessions_week }
                })
                .collect();
        let models: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "select word, model
               from (select acp_id as word, model, started_at from activity.sessions
                      where acp_id is not null and model is not null
                     union all
                     select host, model, started_at from activity.sessions
                      where host is not null and host is distinct from acp_id and model is not null) named
              group by word, model
              order by max(started_at) desc, word, model",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("read session models: {e}"))?;
        Ok(crate::assistants::catalogue::assemble_catalogue(
            &names, &installs, &usage, &models, repo,
        ))
    }

    /// The local path launch templates resolve against (#513): the crew's
    /// repository checkout when `crew_id` names a crew with one, else the
    /// workspace's first repository checkout. `None` when neither resolves —
    /// every `launch` stays null. First checkout wins (by path); the
    /// workspace's first repo wins by membership, matched by id so a
    /// remote-less repository (null key) still resolves.
    pub async fn launch_repo_path(
        &self,
        user_id: uuid::Uuid,
        crew_id: Option<uuid::Uuid>,
        workspace_id: Option<uuid::Uuid>,
    ) -> Result<Option<String>, String> {
        if let Some(crew) = crew_id {
            let key: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
                "select repo_key from relay.crews where user_id = $1 and id = $2",
            )
            .bind(user_id)
            .bind(crew)
            .fetch_optional(self.pool())
            .await
            .map_err(|e| format!("read the crew repo: {e}"))?;
            if let Some((Some(key),)) = key
                && let Some(path) = self.first_checkout_path(&key).await?
            {
                return Ok(Some(path));
            }
        }
        let Some(workspace) = workspace_id else { return Ok(None) };
        // By id, not by key: a remote-less repository has a null key and
        // would never match, but its checkout still starts a tool.
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "select f.abs_path from sensei.repositories_in_workspaces j \
               join sensei.folders f on f.repository_id = j.repository_id and f.kind = 'git' \
              where j.workspace_id = $1 \
              order by j.created_at, f.abs_path limit 1",
        )
        .bind(workspace)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| format!("read the workspace checkouts: {e}"))?;
        Ok(row.map(|r| r.0))
    }

    /// The first local checkout of the repository `repo_key`, by path.
    async fn first_checkout_path(&self, repo_key: &str) -> Result<Option<String>, String> {
        let row: Option<(String,)> = sqlx_core::query_as::query_as(
            "select f.abs_path from sensei.folders f \
               join sensei.repositories r on r.id = f.repository_id \
              where r.repo_key = $1 and f.kind = 'git' \
              order by f.abs_path limit 1",
        )
        .bind(repo_key)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| format!("read the repo checkouts: {e}"))?;
        Ok(row.map(|r| r.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seed repository + checkout + workspace junction under unique keys; the
    /// caller deletes the same keys. Returns (repo_key, abs_path).
    async fn fx_repo(
        pg: &PgStore,
        tag: &str,
        workspace_id: Option<uuid::Uuid>,
    ) -> (String, String) {
        let key = format!("launch-probe-{tag}");
        let path = format!("/tmp/launch-probe-{tag}");
        let repo: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "insert into sensei.repositories (repo_key, name) values ($1, $2) \
             on conflict (repo_key) do update set repo_key = excluded.repo_key returning id",
        )
        .bind(&key)
        .bind(format!("probe-{tag}"))
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let root: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "insert into sensei.folders_to_watch (path, name) values ($1, $2) \
             on conflict (path) do update set path = excluded.path returning id",
        )
        .bind(&path)
        .bind(format!("probe-{tag}"))
        .fetch_one(pg.pool())
        .await
        .unwrap();
        sqlx_core::query::query(
            "insert into sensei.folders (root_id, repository_id, name, path, abs_path, kind) \
             values ($1, $2, $3, $4, $5, 'git') on conflict (abs_path) do nothing",
        )
        .bind(root.0)
        .bind(repo.0)
        .bind(format!("probe-{tag}"))
        .bind(&path)
        .bind(&path)
        .execute(pg.pool())
        .await
        .unwrap();
        if let Some(ws) = workspace_id {
            sqlx_core::query::query(
                "insert into sensei.repositories_in_workspaces (workspace_id, repository_id) \
                 values ($1, $2) on conflict do nothing",
            )
            .bind(ws)
            .bind(repo.0)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        (key, path)
    }

    async fn fx_crew(pg: &PgStore, user: uuid::Uuid, repo_key: Option<&str>) -> uuid::Uuid {
        let id = uuid::Uuid::now_v7();
        sqlx_core::query::query(
            "insert into relay.crews (user_id, id, repo_key, status) values ($1, $2, $3, 'active')",
        )
        .bind(user)
        .bind(id)
        .bind(repo_key)
        .execute(pg.pool())
        .await
        .unwrap();
        id
    }

    async fn un_fx_repo(pg: &PgStore, tag: &str) {
        let key = format!("launch-probe-{tag}");
        let path = format!("/tmp/launch-probe-{tag}");
        for sql in [
            "delete from sensei.folders where abs_path = $1",
            "delete from sensei.folders_to_watch where path = $1",
        ] {
            sqlx_core::query::query(sql).bind(&path).execute(pg.pool()).await.unwrap();
        }
        sqlx_core::query::query("delete from sensei.repositories where repo_key = $1")
            .bind(&key)
            .execute(pg.pool())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn launch_repo_prefers_the_crew_checkout_then_the_workspace_first() {
        let pg = PgStore::test_store().await;
        let user = uuid::Uuid::now_v7();
        let ws: (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "insert into sensei.workspaces (name) values ('launch-probe-ws') returning id",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let (crew_key, crew_path) = fx_repo(&pg, "crew", None).await;
        let (ws_key, ws_path) = fx_repo(&pg, "ws", Some(ws.0)).await;
        let crew = fx_crew(&pg, user, Some(&crew_key)).await;
        let crewless = fx_crew(&pg, user, None).await;

        // The crew's own checkout wins over the workspace's first.
        let got = pg.launch_repo_path(user, Some(crew), Some(ws.0)).await.unwrap();
        assert_eq!(got.as_deref(), Some(crew_path.as_str()));
        // No crew repo: the workspace's first checkout.
        let got = pg.launch_repo_path(user, Some(crewless), Some(ws.0)).await.unwrap();
        assert_eq!(got.as_deref(), Some(ws_path.as_str()));
        // A remote-less repository (null key) still resolves by id.
        sqlx_core::query::query(
            "update sensei.repositories set repo_key = null where repo_key = $1",
        )
        .bind(&ws_key)
        .execute(pg.pool())
        .await
        .unwrap();
        let got = pg.launch_repo_path(user, Some(crewless), Some(ws.0)).await.unwrap();
        assert_eq!(got.as_deref(), Some(ws_path.as_str()));
        // Neither: no repo, every launch stays null.
        let got = pg.launch_repo_path(user, None, None).await.unwrap();
        assert_eq!(got, None);
        // Another person's crew is not theirs: the crew lookup misses and the
        // workspace fallback still applies.
        let stranger = uuid::Uuid::now_v7();
        let got = pg.launch_repo_path(stranger, Some(crew), Some(ws.0)).await.unwrap();
        assert_eq!(got.as_deref(), Some(ws_path.as_str()));
        sqlx_core::query::query(
            "update sensei.repositories set repo_key = $1 where repo_key is null and name = 'probe-ws'",
        )
        .bind(&ws_key)
        .execute(pg.pool())
        .await
        .unwrap();

        sqlx_core::query::query("delete from relay.crews where id = $1 or id = $2")
            .bind(crew)
            .bind(crewless)
            .execute(pg.pool())
            .await
            .unwrap();
        sqlx_core::query::query(
            "delete from sensei.repositories_in_workspaces where workspace_id = $1",
        )
        .bind(ws.0)
        .execute(pg.pool())
        .await
        .unwrap();
        sqlx_core::query::query("delete from sensei.workspaces where id = $1")
            .bind(ws.0)
            .execute(pg.pool())
            .await
            .unwrap();
        un_fx_repo(&pg, "crew").await;
        un_fx_repo(&pg, "ws").await;
        let _ = ws_key;
    }
}
