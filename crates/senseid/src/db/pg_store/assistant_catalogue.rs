//! The seeded catalogue of supported tools and this machine's installs (#455).
use super::{AssistantInstallRow, CatalogueWordRow, PgStore};

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
    /// the catalogue's words. `configured_at` / `configured_version` move when a
    /// tool is first seen configured, or configured by another sensei version.
    /// Returns the registry ids the catalogue does not know; those are not stored.
    pub async fn upsert_assistant_installs(
        &self,
        rows: &[AssistantInstallRow<'_>],
        sensei_version: &str,
    ) -> Result<Vec<String>, String> {
        let mut unknown = Vec::new();
        let mut tx = self.pool.begin().await.map_err(|e| e.to_string())?;
        for r in rows {
            let stored: Option<String> = sqlx_core::query_scalar::query_scalar(
                "insert into sensei.assistant_installs as i
                        (catalogue_id, installed, config_path, configured,
                         configured_at, configured_version, detected_at, modified_at)
                 select w.id, $2, $3, $4,
                        case when $4 then now() end, case when $4 then $5 end, now(), now()
                   from sensei.assistant_catalogue_words w
                  where w.word = $1
                 on conflict (catalogue_id) do update set
                        installed   = excluded.installed,
                        config_path = excluded.config_path,
                        configured  = excluded.configured,
                        configured_at = case
                            when excluded.configured
                             and (not i.configured or i.configured_version is distinct from $5)
                            then now() else i.configured_at end,
                        configured_version = case
                            when excluded.configured then $5 else i.configured_version end,
                        detected_at = now(),
                        modified_at = now()
                 returning catalogue_id",
            )
            .bind(r.registry_id)
            .bind(r.installed)
            .bind(r.config_path)
            .bind(r.configured)
            .bind(sensei_version)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("store the install of {}: {e}", r.registry_id))?;
            if stored.is_none() {
                unknown.push(r.registry_id.to_string());
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
}
