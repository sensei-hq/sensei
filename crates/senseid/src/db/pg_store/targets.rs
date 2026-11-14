//! Targets (docs/spec/dojo/targets-and-gates.md): the store side of the target
//! catalogue, resolution and evaluation. All target SQL lives here.

use super::PgStore;

/// One catalogue metric's gating eligibility (TG1): whether it may be a target,
/// how it is read, and when its reading is too thin. `comparator` comes from the
/// metric's own `direction` (`higher_better → ">="`, `lower_better → "<="`);
/// `None` for a `neutral` metric, which is never eligible. `unit` is the
/// catalogue's, never copied into the gating row.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GatingRow {
    pub metric_key: String,
    pub eligible: bool,
    pub delta_scoped: bool,
    pub min_coverage: f64,
    pub comparator: Option<String>,
    pub unit: Option<String>,
}

/// A gating row as the read returns it, before it becomes a [`GatingRow`].
type GatingTuple = (String, bool, bool, f64, Option<String>, Option<String>);

impl PgStore {
    /// Every catalogue metric that has a gating row, with its comparator. A
    /// metric with no gating row is not in the list: it is not eligible, and no
    /// default row is invented for it. A failed read is an error, never an empty
    /// list (TG4 reads this as "which targets exist"). A neutral metric is never
    /// eligible here, even if a row written outside the import says so.
    pub async fn metric_gating(&self) -> Result<Vec<GatingRow>, String> {
        let rows: Vec<GatingTuple> = sqlx_core::query_as::query_as(
            "SELECT m.key,
                        g.eligible AND m.direction <> 'neutral',
                        g.delta_scoped,
                        g.min_coverage::float8,
                        CASE m.direction WHEN 'higher_better' THEN '>='
                                         WHEN 'lower_better'  THEN '<=' END,
                        m.unit
                   FROM sensei.metric_gating g
                   JOIN sensei.metrics m ON m.id = g.metric_id
                  ORDER BY m.key",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("metric_gating read failed: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(metric_key, eligible, delta_scoped, min_coverage, comparator, unit)| GatingRow {
                metric_key,
                eligible,
                delta_scoped,
                min_coverage,
                comparator,
                unit,
            })
            .collect())
    }
}

/// What recording a fetch link did (TG5, D63).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchStart {
    /// The first fetch of this ticket in this session: work starts now.
    Recorded,
    /// The session had already fetched it; work started then, and nothing changed.
    AlreadyStarted(chrono::DateTime<chrono::Utc>),
}

impl PgStore {
    /// Every workspace a folder belongs to (`sensei.folder_workspaces`): none, one,
    /// or — when its repository serves several — more than one. Read here rather
    /// than `sole_workspace_of`, whose NULL conflates "none" with "several".
    pub async fn workspaces_of_folder(
        &self,
        folder_id: &uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, String> {
        let rows: Vec<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT DISTINCT workspace_id FROM sensei.folder_workspaces
              WHERE folder_id = $1 AND workspace_id IS NOT NULL ORDER BY 1",
        )
        .bind(folder_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("folder workspaces read failed: {e}"))?;
        Ok(rows.into_iter().map(|(w,)| w).collect())
    }

    /// The session an assistant knows by its own id (`activity.sessions.client_session_id`),
    /// or `None` when no such session has been ingested.
    pub async fn session_for_client_id(
        &self,
        client_session_id: &str,
    ) -> Result<Option<uuid::Uuid>, String> {
        let row: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "SELECT id FROM activity.sessions WHERE client_session_id = $1 ORDER BY started_at DESC LIMIT 1",
        )
        .bind(client_session_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("session read failed: {e}"))?;
        Ok(row.map(|(id,)| id))
    }

    /// Record that a session started work on a ticket: a `fetch` link with
    /// `started_at = now()`. A second fetch changes nothing and says when work
    /// started.
    pub async fn record_fetch_link(
        &self,
        session_id: &uuid::Uuid,
        task_id: &uuid::Uuid,
        key: &str,
    ) -> Result<FetchStart, String> {
        let inserted: Option<(uuid::Uuid,)> = sqlx_core::query_as::query_as(
            "INSERT INTO sensei.session_task_links
                    (session_id, task_id, source, matched_key, mentions, started_at)
             VALUES ($1, $2, 'fetch', $3, 1, now())
             ON CONFLICT (session_id, task_id, source) DO NOTHING
             RETURNING session_id",
        )
        .bind(session_id)
        .bind(task_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("fetch link write failed: {e}"))?;
        if inserted.is_some() {
            return Ok(FetchStart::Recorded);
        }
        let (started,): (Option<chrono::DateTime<chrono::Utc>>,) = sqlx_core::query_as::query_as(
            "SELECT started_at FROM sensei.session_task_links
              WHERE session_id = $1 AND task_id = $2 AND source = 'fetch'",
        )
        .bind(session_id)
        .bind(task_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("fetch link read failed: {e}"))?;
        started
            .map(FetchStart::AlreadyStarted)
            .ok_or_else(|| "fetch link without started_at".to_string())
    }
}
