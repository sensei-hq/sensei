//! A tracker's ticket history (spec tracker-sdlc-metrics S4): the board's terms,
//! its statuses, and each ticket's changelog entries for sprint, status and
//! estimate. Written by the history task, never by sync.

use super::*;
use crate::tracker::{BoardTerms, ChangeRow};

/// A ticket whose changelog is unread or older than its last update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCandidate {
    pub task_id: uuid::Uuid,
    pub external_id: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl PgStore {
    /// Store what the board estimates in, its site's Sprint field and its statuses
    /// (S4), replacing the statuses in one transaction.
    pub async fn set_board_terms(
        &self,
        tracker_id: uuid::Uuid,
        terms: &BoardTerms,
    ) -> Result<(), String> {
        let db = |e: sqlx_core::Error| format!("set_board_terms({tracker_id}): {e}");
        let mut tx = self.pool.begin().await.map_err(db)?;
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET estimation_field = $2, sprint_field = $3, terms_at = now() WHERE id = $1",
        )
        .bind(tracker_id)
        .bind(&terms.estimation_field)
        .bind(&terms.sprint_field)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx_core::query::query("DELETE FROM sensei.tracker_statuses WHERE tracker_id = $1")
            .bind(tracker_id)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let (ids, (names, categories)): (Vec<&str>, (Vec<&str>, Vec<&str>)) = terms
            .statuses
            .iter()
            .map(|(id, name, category)| (id.as_str(), (name.as_str(), category.as_str())))
            .unzip();
        sqlx_core::query::query(
            "INSERT INTO sensei.tracker_statuses (tracker_id, status_id, name, category)
             SELECT $1, x.i, x.n, x.c
               FROM unnest($2::text[], $3::text[], $4::text[]) AS x(i, n, c)
              WHERE x.c IN ('new', 'indeterminate', 'done', 'undefined')
             ON CONFLICT DO NOTHING",
        )
        .bind(tracker_id)
        .bind(&ids)
        .bind(&names)
        .bind(&categories)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)
    }

    /// Tickets whose history must be read (S4), oldest update first. A ticket the
    /// tracker gave no `updated` is never selected.
    pub async fn history_candidates(
        &self,
        tracker_id: uuid::Uuid,
        limit: i64,
    ) -> Result<Vec<HistoryCandidate>, String> {
        let rows: Vec<(uuid::Uuid, String, chrono::DateTime<chrono::Utc>)> =
            sqlx_core::query_as::query_as(
                "SELECT id, external_id, issue_updated_at
               FROM sensei.tracker_tasks
              WHERE tracker_id = $1 AND removed_at IS NULL
                AND issue_updated_at IS NOT NULL
                AND (history_through IS NULL OR issue_updated_at > history_through)
              ORDER BY issue_updated_at, id
              LIMIT $2",
            )
            .bind(tracker_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("history_candidates({tracker_id}): {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(task_id, external_id, updated_at)| HistoryCandidate {
                task_id,
                external_id,
                updated_at,
            })
            .collect())
    }

    /// Insert a ticket's changelog rows (existing ones skipped) and mark it read
    /// through `through`, in one transaction.
    pub async fn store_task_history(
        &self,
        task_id: uuid::Uuid,
        changes: &[ChangeRow],
        through: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        let db = |e: sqlx_core::Error| format!("store_task_history({task_id}): {e}");
        let mut tx = self.pool.begin().await.map_err(db)?;
        let ids: Vec<&str> = changes.iter().map(|c| c.change_id.as_str()).collect();
        let fields: Vec<&str> = changes.iter().map(|c| c.field).collect();
        let ats: Vec<chrono::DateTime<chrono::Utc>> = changes.iter().map(|c| c.at).collect();
        let froms: Vec<Option<&str>> = changes.iter().map(|c| c.from_value.as_deref()).collect();
        let tos: Vec<Option<&str>> = changes.iter().map(|c| c.to_value.as_deref()).collect();
        sqlx_core::query::query(
            "INSERT INTO sensei.tracker_task_changes (task_id, change_id, field, at, from_value, to_value)
             SELECT $1, x.i, x.f::sensei.tracked_change_field, x.a, x.fr, x.t
               FROM unnest($2::text[], $3::text[], $4::timestamptz[], $5::text[], $6::text[])
                    AS x(i, f, a, fr, t)
             ON CONFLICT DO NOTHING",
        )
        .bind(task_id)
        .bind(&ids)
        .bind(&fields)
        .bind(&ats)
        .bind(&froms)
        .bind(&tos)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx_core::query::query(
            "UPDATE sensei.tracker_tasks SET history_through = $2 WHERE id = $1",
        )
        .bind(task_id)
        .bind(through)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        tx.commit().await.map_err(db)
    }

    /// End a history run: when it ran, and why it stopped early (`None` when it
    /// completed or ran out of time cleanly). Never touches `last_error`.
    pub async fn set_history_outcome(
        &self,
        tracker_id: uuid::Uuid,
        error: Option<&str>,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.trackers SET history_at = now(), history_error = $2::sensei.tracker_error
              WHERE id = $1",
        )
        .bind(tracker_id)
        .bind(error)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| format!("set_history_outcome({tracker_id}): {e}"))
    }

    /// What a ticket's figures need from its history (S6): its creation time, its
    /// points, and whether the board's estimation is known yet.
    pub async fn ticket_history_facts(
        &self,
        task_ids: &[uuid::Uuid],
    ) -> Result<std::collections::HashMap<uuid::Uuid, TicketHistoryFacts>, String> {
        type Row =
            (uuid::Uuid, Option<chrono::DateTime<chrono::Utc>>, Option<f64>, Option<String>, bool);
        let rows: Vec<Row> =
            sqlx_core::query_as::query_as(
                "SELECT t.id, t.issue_created_at, t.points::float8, tr.estimation_field,
                        tr.terms_at IS NOT NULL
                        AND tr.points_field IS NOT DISTINCT FROM coalesce(tr.estimation_field, 'none')
                   FROM sensei.tracker_tasks t JOIN sensei.trackers tr ON tr.id = t.tracker_id
                  WHERE t.id = ANY($1)",
            )
            .bind(task_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| format!("ticket_history_facts: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(id, created_at, points, field, learned)| {
                (id, TicketHistoryFacts { created_at, points, has_field: field.is_some(), learned })
            })
            .collect())
    }

    /// The tracker's history outcome, for its listing (S7).
    pub async fn tracker_history_state(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<(Option<String>, Option<chrono::DateTime<chrono::Utc>>), String> {
        sqlx_core::query_as::query_as(
            "SELECT history_error::text, history_at FROM sensei.trackers WHERE id = $1",
        )
        .bind(tracker_id)
        .fetch_optional(&self.pool)
        .await
        .map(|r| r.unwrap_or((None, None)))
        .map_err(|e| format!("tracker_history_state({tracker_id}): {e}"))
    }

    /// Everything the sprint replay reads for a tracker (S5): its sprints, its
    /// tickets with their history, its statuses, and whether its points are usable.
    pub async fn replay_inputs(&self, tracker_id: uuid::Uuid) -> Result<ReplayInputs, String> {
        use crate::sdlc::sprint_replay::{Change, ChangeField, ReplayTask};
        let db = |e: sqlx_core::Error| format!("replay_inputs({tracker_id}): {e}");
        type SprintRow = (
            uuid::Uuid,
            String,
            i64,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        );
        let sprints: Vec<SprintRow> = sqlx_core::query_as::query_as(
            "SELECT id, external_id, seq, state, start_at, end_at, complete_at
               FROM sensei.sprints WHERE tracker_id = $1 AND removed_at IS NULL ORDER BY seq DESC",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        type TaskRow = (
            uuid::Uuid,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<String>,
            Option<f64>,
            Vec<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        );
        let tasks: Vec<TaskRow> = sqlx_core::query_as::query_as(
            "SELECT t.id, t.issue_created_at, t.status_category, t.points::float8,
                    coalesce(array_agg(s.external_id) FILTER (WHERE s.id IS NOT NULL), '{}'),
                    t.issue_updated_at, t.history_through
               FROM sensei.tracker_tasks t
               LEFT JOIN sensei.tracker_task_sprints m ON m.task_id = t.id
               LEFT JOIN sensei.sprints s ON s.id = m.sprint_id AND s.removed_at IS NULL
              WHERE t.tracker_id = $1 AND t.removed_at IS NULL
              GROUP BY t.id",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        type ChangeRow =
            (uuid::Uuid, String, chrono::DateTime<chrono::Utc>, Option<String>, Option<String>);
        let changes: Vec<ChangeRow> = sqlx_core::query_as::query_as(
            "SELECT c.task_id, c.field::text, c.at, c.from_value, c.to_value
               FROM sensei.tracker_task_changes c JOIN sensei.tracker_tasks t ON t.id = c.task_id
              WHERE t.tracker_id = $1
              ORDER BY c.at, length(c.change_id), c.change_id",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        let statuses: Vec<(String, String)> = sqlx_core::query_as::query_as(
            "SELECT status_id, category FROM sensei.tracker_statuses WHERE tracker_id = $1",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;
        let (field, learned): (Option<String>, bool) = sqlx_core::query_as::query_as(
            "SELECT estimation_field,
                    terms_at IS NOT NULL AND points_field IS NOT DISTINCT FROM coalesce(estimation_field, 'none')
               FROM sensei.trackers WHERE id = $1",
        )
        .bind(tracker_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?
        .ok_or_else(|| format!("replay_inputs: tracker {tracker_id} not found"))?;

        let mut by_task: std::collections::HashMap<uuid::Uuid, Vec<Change>> = Default::default();
        for (task, field, at, from, to) in changes {
            let field = match field.as_str() {
                "sprint" => ChangeField::Sprint,
                "status" => ChangeField::Status,
                "estimate" => ChangeField::Estimate,
                other => return Err(format!("unknown change field {other}")),
            };
            by_task.entry(task).or_default().push(Change {
                field,
                at: at.timestamp_millis(),
                from,
                to,
            });
        }
        Ok(ReplayInputs {
            sprints: sprints
                .into_iter()
                .map(|(id, external_id, seq, state, start_at, end_at, complete_at)| ReplaySprint {
                    id,
                    external_id,
                    seq,
                    state,
                    start_at,
                    end_at,
                    complete_at,
                })
                .collect(),
            tasks: tasks
                .into_iter()
                .map(|(id, created, category, points, current, updated, through)| ReplayTask {
                    created_at: created.map(|c| c.timestamp_millis()),
                    current_sprints: current,
                    current_category: category,
                    current_points: points,
                    changes: by_task.remove(&id).unwrap_or_default(),
                    // S4 reads a ticket's changelog through its update time.
                    history_gap_from: match (updated, through) {
                        (Some(u), Some(h)) if u <= h => None,
                        (_, Some(h)) => Some(h.timestamp_millis()),
                        (_, None) => Some(i64::MIN),
                    },
                })
                .collect(),
            statuses: statuses.into_iter().collect(),
            estimation: match (learned, field) {
                (false, _) => Err("not_synced"),
                (true, None) => Err("no_estimation_field"),
                (true, Some(_)) => Ok(()),
            },
        })
    }

    /// The token totals of these sessions (S6 project), and how many lack one of the
    /// four token fields.
    pub async fn session_token_totals(
        &self,
        session_ids: &[uuid::Uuid],
    ) -> Result<(i64, i64), String> {
        sqlx_core::query_as::query_as(
            "SELECT coalesce(sum(tokens_fresh + cache_read + cache_write + tokens_out), 0)::int8,
                    count(*) FILTER (WHERE tokens_fresh IS NULL OR cache_read IS NULL
                                        OR cache_write IS NULL OR tokens_out IS NULL)
               FROM activity.sessions WHERE id = ANY($1)",
        )
        .bind(session_ids)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("session_token_totals: {e}"))
    }
}

/// A ticket's history-derived facts (S6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TicketHistoryFacts {
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub points: Option<f64>,
    /// The board has an estimation field (meaningful once `learned`).
    pub has_field: bool,
    /// The history task has read the board's configuration, and the last sync read
    /// points under the field it learned (A6).
    pub learned: bool,
}

/// A sprint as the replay reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaySprint {
    pub id: uuid::Uuid,
    pub external_id: String,
    pub seq: i64,
    pub state: String,
    pub start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub end_at: Option<chrono::DateTime<chrono::Utc>>,
    pub complete_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// What [`PgStore::replay_inputs`] returns.
#[derive(Debug, Clone)]
pub struct ReplayInputs {
    /// Newest first (by `seq`).
    pub sprints: Vec<ReplaySprint>,
    pub tasks: Vec<crate::sdlc::sprint_replay::ReplayTask>,
    pub statuses: std::collections::HashMap<String, String>,
    /// Whether the board's points may be used, or why not.
    pub estimation: Result<(), &'static str>,
}
