//! The seeded catalogue of supported tools and this machine's installs (#455).
use super::{AssistantInstallRow, PgStore};
use crate::assistants::CatalogueWordRow;

impl PgStore {
    /// Every spelling of every supported tool (`sensei.assistant_catalogue_words`),
    /// one query. A failed read is an error, never an empty catalogue.
    pub async fn catalogue_words(&self) -> Result<Vec<CatalogueWordRow>, String> {
        let rows: Vec<(String, String, String, String, bool)> = sqlx_core::query_as::query_as(
            "select word, id, name, kind::text, joins_crew from sensei.assistant_catalogue_words",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("read the assistant catalogue: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(word, id, name, kind, joins_crew)| CatalogueWordRow {
                word,
                id,
                name,
                kind,
                joins_crew,
            })
            .collect())
    }

    /// The catalogue's words as [`crate::assistants::CatalogueNames`].
    pub async fn catalogue_names(&self) -> Result<crate::assistants::CatalogueNames, String> {
        Ok(crate::assistants::CatalogueNames::from_rows(self.catalogue_words().await?))
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
    /// what sessions named, shaped by [`crate::assistants::assemble_catalogue`].
    /// Reads only the database, never the filesystem; any failed read is an error.
    pub async fn relay_catalogue(&self) -> Result<serde_json::Value, String> {
        let names = self.catalogue_names().await?;
        let installs: Vec<(String, bool, bool, Option<String>)> = sqlx_core::query_as::query_as(
            "select catalogue_id, installed, configured, config_path from sensei.assistant_installs",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("read the assistant installs: {e}"))?;
        let installs: Vec<crate::assistants::ToolInstall> = installs
            .into_iter()
            .map(|(id, installed, configured, config_path)| crate::assistants::ToolInstall {
                id,
                installed,
                configured,
                config_path,
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
        let usage: Vec<crate::assistants::WordUsage> = usage
            .into_iter()
            .map(|(word, last_used_at, sessions_week)| crate::assistants::WordUsage {
                word,
                last_used_at,
                sessions_week,
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
        Ok(crate::assistants::assemble_catalogue(&names, &installs, &usage, &models))
    }
}
