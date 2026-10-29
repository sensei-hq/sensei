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
