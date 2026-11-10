//! The reads and marks behind a tracker board's delivery push (spec
//! tracker-delivery-metrics D3; #352): which boards may travel, their sprints and
//! non-removed tickets, a consistency token, and the re-push watermark.

use super::*;
use crate::db::pg_store::tracker_history::HISTORY_DUE;
use crate::dojo::delivery::MarkState;

/// A synced sprint board, with what identifies it canonically.
#[derive(Debug, Clone)]
pub struct DeliveryTracker {
    pub id: uuid::Uuid,
    pub workspace_id: uuid::Uuid,
    pub kind: String,
    pub base_url: String,
    pub board_ref: String,
    pub board_name: Option<String>,
    /// When the history task last ran; sent, never hashed.
    pub history_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// One sprint of a board, the removed included (it travels with no figures).
#[derive(Debug, Clone)]
pub struct DeliverySprintRow {
    pub id: uuid::Uuid,
    pub external_id: String,
    pub seq: i64,
    pub name: String,
    pub state: String,
    pub start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub end_at: Option<chrono::DateTime<chrono::Utc>>,
    pub complete_at: Option<chrono::DateTime<chrono::Utc>>,
    pub removed: bool,
}

/// One non-removed ticket, with every non-removed sprint that lists it.
#[derive(Debug, Clone)]
pub struct DeliveryTicketRow {
    pub key: String,
    pub title: Option<String>,
    pub status_category: Option<String>,
    pub points: Option<f64>,
    pub sprints: Vec<String>,
    pub issue_created_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// What a sync or the history task moves. Read before and after a board's facts;
/// a board whose token moved in between is skipped that tick, never sent half-updated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryToken {
    pub last_success_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Tickets the history task would still read (its own predicate).
    pub pending: i64,
    pub history_through: Option<chrono::DateTime<chrono::Utc>>,
    pub issue_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

type TrackerRow = (
    uuid::Uuid,
    uuid::Uuid,
    String,
    String,
    String,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
);
type SprintRow = (
    uuid::Uuid,
    String,
    i64,
    String,
    String,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
    bool,
);
type TicketRow = (
    String,
    Option<String>,
    Option<String>,
    Option<f64>,
    Vec<String>,
    Option<chrono::DateTime<chrono::Utc>>,
);
type MarkRow = (
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
);
/// One part of a board as one persona pushed it to one tenant: a
/// `sensei.tracker_push_marks` row's key.
#[derive(Debug, Clone, Copy)]
pub struct PartKey<'a> {
    pub tracker_id: uuid::Uuid,
    pub persona_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub part: &'a str,
}

/// One queued withdrawal: a board this install pushed to a tenant (B13; #355).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawalKey {
    pub board_key: String,
    pub tenant_id: uuid::Uuid,
    pub persona_id: uuid::Uuid,
}

type TokenRow = (
    String,
    Option<chrono::DateTime<chrono::Utc>>,
    i64,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
);

impl PgStore {
    /// Sprint boards that have synced at least once: the boards with facts to send.
    pub async fn delivery_trackers(&self) -> Result<Vec<DeliveryTracker>, String> {
        let rows: Vec<TrackerRow> = sqlx_core::query_as::query_as(
            "SELECT id, workspace_id, kind::text, base_url, board_ref, params ->> 'board_name', history_at
               FROM sensei.trackers
              WHERE level = 'sprint' AND last_success_at IS NOT NULL
              ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("delivery_trackers: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(id, workspace_id, kind, base_url, board_ref, board_name, history_at)| {
                DeliveryTracker {
                    id,
                    workspace_id,
                    kind,
                    base_url,
                    board_ref,
                    board_name,
                    history_at,
                }
            })
            .collect())
    }

    /// Every sprint of the board, removed ones flagged, in `(seq, external_id)` order.
    pub async fn delivery_sprints(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<Vec<DeliverySprintRow>, String> {
        let rows: Vec<SprintRow> = sqlx_core::query_as::query_as(
            "SELECT id, external_id, seq, name, state, start_at, end_at, complete_at, removed_at IS NOT NULL
               FROM sensei.sprints
              WHERE tracker_id = $1
              ORDER BY seq, external_id",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("delivery_sprints({tracker_id}): {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(id, external_id, seq, name, state, start_at, end_at, complete_at, removed)| {
                DeliverySprintRow {
                    id,
                    external_id,
                    seq,
                    name,
                    state,
                    start_at,
                    end_at,
                    complete_at,
                    removed,
                }
            })
            .collect())
    }

    /// The board's non-removed tickets, each with the external ids of every
    /// non-removed sprint that lists it (its membership), in key order.
    pub async fn delivery_tickets(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<Vec<DeliveryTicketRow>, String> {
        let rows: Vec<TicketRow> = sqlx_core::query_as::query_as(
            "SELECT t.key, t.summary, t.status_category, t.points::float8,
                    array_remove(array_agg(s.external_id ORDER BY s.seq, s.external_id), NULL),
                    t.issue_created_at
               FROM sensei.tracker_tasks t
               LEFT JOIN sensei.tracker_task_sprints x ON x.task_id = t.id
               LEFT JOIN sensei.sprints s ON s.id = x.sprint_id AND s.removed_at IS NULL
              WHERE t.tracker_id = $1 AND t.removed_at IS NULL
              GROUP BY t.id
              ORDER BY t.key",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("delivery_tickets({tracker_id}): {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(key, title, status_category, points, sprints, issue_created_at)| {
                DeliveryTicketRow { key, title, status_category, points, sprints, issue_created_at }
            })
            .collect())
    }

    /// The board's consistency token: its last sync, the history task's pending
    /// count, and the newest history and update it holds. A tracker whose adapter
    /// reads no history has nothing pending: the history task never reads it (G8).
    pub async fn delivery_token(&self, tracker_id: uuid::Uuid) -> Result<DeliveryToken, String> {
        let sql = format!(
            "SELECT tr.kind::text, tr.last_success_at,
                    (SELECT count(*) FROM sensei.tracker_tasks WHERE tracker_id = tr.id AND {HISTORY_DUE}),
                    (SELECT max(history_through) FROM sensei.tracker_tasks WHERE tracker_id = tr.id),
                    (SELECT max(issue_updated_at) FROM sensei.tracker_tasks WHERE tracker_id = tr.id)
               FROM sensei.trackers tr
              WHERE tr.id = $1"
        );
        let row: Option<TokenRow> = sqlx_core::query_as::query_as(&sql)
            .bind(tracker_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("delivery_token({tracker_id}): {e}"))?;
        let (kind, last_success_at, pending, history_through, issue_updated_at) =
            row.ok_or_else(|| format!("delivery_token({tracker_id}): no such tracker"))?;
        let pending =
            if crate::tracker::capabilities_of_kind(&kind)?.history { pending } else { 0 };
        Ok(DeliveryToken { last_success_at, pending, history_through, issue_updated_at })
    }

    /// The tracker's `last_success_at` and the whole seconds since it, both on the
    /// database's clock in one read (A5): the age a no-history board sends.
    pub async fn delivery_sync_age(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<(Option<chrono::DateTime<chrono::Utc>>, Option<i64>), String> {
        let row: Option<(Option<chrono::DateTime<chrono::Utc>>, Option<i64>)> =
            sqlx_core::query_as::query_as(
                "SELECT last_success_at, floor(extract(epoch FROM now() - last_success_at))::bigint
                   FROM sensei.trackers WHERE id = $1",
            )
            .bind(tracker_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("delivery_sync_age({tracker_id}): {e}"))?;
        row.ok_or_else(|| format!("delivery_sync_age({tracker_id}): no such tracker"))
    }

    /// Each of a tracker's candidate sessions and its repository's `repo_key` (`None`
    /// when the repository has none): what proves a session a persona's (#240, A2).
    pub async fn candidate_session_keys(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<std::collections::HashMap<uuid::Uuid, Option<String>>, String> {
        let rows: Vec<(uuid::Uuid, Option<String>)> = sqlx_core::query_as::query_as(&format!(
            "SELECT cs.session_id, r.repo_key
               FROM ({candidates}) cs
               JOIN sensei.repositories r ON r.id = cs.repository_id",
            candidates = crate::db::pg_store::trackers::candidate_sessions("$1"),
        ))
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("candidate_session_keys({tracker_id}): {e}"))?;
        Ok(rows.into_iter().collect())
    }

    /// The keys of a workspace's shareable repositories (those with a `repo_key`).
    pub async fn workspace_repo_keys(
        &self,
        workspace_id: uuid::Uuid,
    ) -> Result<Vec<String>, String> {
        let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
            "SELECT r.repo_key
               FROM sensei.repositories_in_workspaces riw
               JOIN sensei.repositories r ON r.id = riw.repository_id
              WHERE riw.workspace_id = $1 AND r.repo_key IS NOT NULL
              ORDER BY r.repo_key",
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("workspace_repo_keys({workspace_id}): {e}"))?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// A board that stopped being eligible for this persona: queue a withdrawal for
    /// every tenant it was pushed to, and drop those marks, in one transaction (B13).
    /// Answers how many were queued (0 when it was never pushed).
    pub async fn queue_withdrawal(
        &self,
        tracker_id: uuid::Uuid,
        persona_id: uuid::Uuid,
    ) -> Result<u64, String> {
        let err = |e: sqlx_core::Error| format!("queue_withdrawal({tracker_id}): {e}");
        let mut tx = self.pool.begin().await.map_err(err)?;
        let queued = sqlx_core::query::query(
            "INSERT INTO sensei.delivery_withdrawals (board_key, tenant_id, persona_id)
             SELECT DISTINCT m.board_key, m.tenant_id, m.persona_id
               FROM sensei.tracker_push_marks m WHERE m.tracker_id = $1 AND m.persona_id = $2
             ON CONFLICT (board_key, tenant_id, persona_id)
             DO UPDATE SET queued_at = now(), refused_at = NULL, error = NULL",
        )
        .bind(tracker_id)
        .bind(persona_id)
        .execute(&mut *tx)
        .await
        .map_err(err)?
        .rows_affected();
        sqlx_core::query::query(
            "DELETE FROM sensei.tracker_push_marks WHERE tracker_id = $1 AND persona_id = $2",
        )
        .bind(tracker_id)
        .bind(persona_id)
        .execute(&mut *tx)
        .await
        .map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(queued)
    }

    /// This persona's queued withdrawals due now, oldest first: one the dōjō refused
    /// waits an hour (B13; #355).
    pub async fn delivery_withdrawals(
        &self,
        persona_id: uuid::Uuid,
    ) -> Result<Vec<WithdrawalKey>, String> {
        let rows: Vec<(String, uuid::Uuid)> = sqlx_core::query_as::query_as(
            "SELECT board_key, tenant_id FROM sensei.delivery_withdrawals
              WHERE persona_id = $1
                AND (refused_at IS NULL OR refused_at < now() - interval '1 hour')
              ORDER BY queued_at, board_key",
        )
        .bind(persona_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("delivery_withdrawals({persona_id}): {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(board_key, tenant_id)| WithdrawalKey { board_key, tenant_id, persona_id })
            .collect())
    }

    /// A withdrawal landed, or the board was pushed and acknowledged again since it
    /// was queued: either way it leaves the queue.
    pub async fn withdrawal_done(&self, w: &WithdrawalKey) -> Result<(), String> {
        sqlx_core::query::query(
            "DELETE FROM sensei.delivery_withdrawals
              WHERE board_key = $1 AND tenant_id = $2 AND persona_id = $3",
        )
        .bind(&w.board_key)
        .bind(w.tenant_id)
        .bind(w.persona_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| format!("withdrawal_done: {e}"))
    }

    /// A withdrawal did not land: it stays queued, with why; a refusal waits an hour.
    pub async fn withdrawal_failed(
        &self,
        w: &WithdrawalKey,
        error: &str,
        refused: bool,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.delivery_withdrawals
                SET attempted_at = now(), error = $4,
                    refused_at = CASE WHEN $5 THEN now() END
              WHERE board_key = $1 AND tenant_id = $2 AND persona_id = $3",
        )
        .bind(&w.board_key)
        .bind(w.tenant_id)
        .bind(w.persona_id)
        .bind(error)
        .bind(refused)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|e| format!("withdrawal_failed: {e}"))
    }

    /// What was last recorded of a part's push, or `None` when never attempted.
    pub async fn push_mark(&self, key: &PartKey<'_>) -> Result<Option<MarkState>, String> {
        let row: Option<MarkRow> = sqlx_core::query_as::query_as(
            "SELECT hash, pushed_at, failed_hash, failed_at FROM sensei.tracker_push_marks
              WHERE tracker_id = $1 AND persona_id = $2 AND tenant_id = $3 AND part = $4",
        )
        .bind(key.tracker_id)
        .bind(key.persona_id)
        .bind(key.tenant_id)
        .bind(key.part)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("push_mark: {e}"))?;
        Ok(row.map(|(hash, pushed_at, failed_hash, failed_at)| MarkState {
            hash,
            pushed_at,
            failed_hash,
            failed_at,
        }))
    }

    /// Record that a part is about to be sent, BEFORE sending it: a push the dōjō
    /// commits but whose answer is lost still leaves this row behind.
    pub async fn mark_push_attempt(
        &self,
        key: &PartKey<'_>,
        board_key: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.tracker_push_marks (tracker_id, persona_id, tenant_id, part, board_key)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (tracker_id, persona_id, tenant_id, part)
             DO UPDATE SET attempted_at = now(), board_key = EXCLUDED.board_key",
        )
        .bind(key.tracker_id)
        .bind(key.persona_id)
        .bind(key.tenant_id)
        .bind(key.part)
        .bind(board_key)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_push_attempt: {e}"))?;
        Ok(())
    }

    /// The dōjō accepted the part, or kept a better copy (stale): this hash is sent.
    pub async fn mark_push_acknowledged(
        &self,
        key: &PartKey<'_>,
        hash: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.tracker_push_marks
                SET hash = $5, pushed_at = now(), failed_hash = NULL, failed_at = NULL, error = NULL
              WHERE tracker_id = $1 AND persona_id = $2 AND tenant_id = $3 AND part = $4",
        )
        .bind(key.tracker_id)
        .bind(key.persona_id)
        .bind(key.tenant_id)
        .bind(key.part)
        .bind(hash)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_push_acknowledged: {e}"))?;
        Ok(())
    }

    /// The dōjō refused this hash; it is not re-sent within the backoff.
    pub async fn mark_push_refused(
        &self,
        key: &PartKey<'_>,
        hash: &str,
        error: &str,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "UPDATE sensei.tracker_push_marks
                SET failed_hash = $5, failed_at = now(), error = $6
              WHERE tracker_id = $1 AND persona_id = $2 AND tenant_id = $3 AND part = $4",
        )
        .bind(key.tracker_id)
        .bind(key.persona_id)
        .bind(key.tenant_id)
        .bind(key.part)
        .bind(hash)
        .bind(error)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("mark_push_refused: {e}"))?;
        Ok(())
    }
}
