//! Tracker usage reads (spec R7, R8): who worked on which ticket and sprint,
//! computed at read time from session rows. Nothing is materialised.

use std::collections::HashMap;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::candidate_sessions;
use crate::tracker::usage::Usage;

type Ts = chrono::DateTime<chrono::Utc>;

/// One non-removed sprint of a tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSprint {
    pub id: uuid::Uuid,
    pub name: String,
    pub state: String,
    pub start_at: Option<chrono::DateTime<chrono::Utc>>,
    pub end_at: Option<chrono::DateTime<chrono::Utc>>,
    pub complete_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Tickets the sprint lists.
    pub tasks: i64,
}

/// A candidate session linked to a ticket, with its repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketSession {
    pub task_id: uuid::Uuid,
    pub session_id: uuid::Uuid,
    pub repository_id: uuid::Uuid,
    pub repository: String,
}

/// One ticket as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListRow {
    pub id: uuid::Uuid,
    pub key: String,
    pub summary: Option<String>,
    pub issue_type: Option<String>,
    pub status: Option<String>,
    pub removed: bool,
    pub current_sprint: Option<(uuid::Uuid, String)>,
    pub sprints: Vec<(uuid::Uuid, String)>,
}

/// Which tickets a list covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskScope {
    All,
    /// The tickets this sprint lists.
    Sprint(uuid::Uuid),
    /// The tickets no non-removed sprint lists (R7 backlog).
    Backlog,
}

/// Which part of a sprint's window a candidate session falls in (R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Part {
    LinkedSprint,
    LinkedOther,
    Unlinked,
}

/// A session as the unlinked list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionBrief {
    pub id: uuid::Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub task: String,
}

impl PgStore {
    /// R7 usage of each group of sessions. `members` pairs a group with a session
    /// (duplicates ignored); `targets` pairs a group with the tasks its `by_source`
    /// counts links to. A group with no members is absent.
    pub async fn usage_grouped(
        &self,
        tracker_id: uuid::Uuid,
        members: &[(String, uuid::Uuid)],
        targets: &[(String, uuid::Uuid)],
    ) -> Result<HashMap<String, Usage>, String> {
        let (m_group, m_session): (Vec<String>, Vec<uuid::Uuid>) = members.iter().cloned().unzip();
        let (t_group, t_task): (Vec<String>, Vec<uuid::Uuid>) = targets.iter().cloned().unzip();
        // Built as JSON in the R7 shape: a SUM over no values is NULL, never 0.
        let rows = sqlx_core::query_as::query_as::<_, (String, serde_json::Value)>(
            "WITH m AS (SELECT DISTINCT g, sid FROM unnest($2::text[], $3::uuid[]) AS m(g, sid)),
                  tg AS (SELECT DISTINCT g, tid FROM unnest($4::text[], $5::uuid[]) AS t(g, tid)),
                  tl AS (SELECT l.session_id, l.task_id, l.source
                           FROM sensei.session_task_links l
                           JOIN sensei.tracker_tasks t ON t.id = l.task_id
                          WHERE t.tracker_id = $1 AND l.session_id IN (SELECT sid FROM m)),
                  multi AS (SELECT session_id FROM tl GROUP BY session_id
                            HAVING count(DISTINCT task_id) >= 2),
                  src AS (SELECT DISTINCT tg.g, tl.session_id, tl.source::text AS source
                            FROM tl JOIN tg ON tg.tid = tl.task_id)
             SELECT m.g, jsonb_build_object(
                    'sessions', count(*),
                    'by_source', jsonb_build_object(
                        'branch', count(*) FILTER (WHERE EXISTS (SELECT 1 FROM src
                                   WHERE src.g = m.g AND src.session_id = m.sid AND src.source = 'branch')),
                        'prompt', count(*) FILTER (WHERE EXISTS (SELECT 1 FROM src
                                   WHERE src.g = m.g AND src.session_id = m.sid AND src.source = 'prompt'))),
                    'shared', count(*) FILTER (WHERE m.sid IN (SELECT session_id FROM multi)),
                    'active_secs', sum(extract(epoch FROM s.duration))::float8,
                    'active_n', count(s.duration),
                    'tokens_fresh', sum(s.tokens_fresh)::bigint, 'tokens_fresh_n', count(s.tokens_fresh),
                    'cache_read', sum(s.cache_read)::bigint, 'cache_read_n', count(s.cache_read),
                    'cache_write', sum(s.cache_write)::bigint, 'cache_write_n', count(s.cache_write),
                    'tokens_out', sum(s.tokens_out)::bigint, 'tokens_out_n', count(s.tokens_out),
                    'tokens_reasoning', sum(s.tokens_reasoning)::bigint,
                    'tokens_reasoning_n', count(s.tokens_reasoning),
                    'metered_cost', sum(s.metered_cost)::float8, 'metered_cost_n', count(s.metered_cost))
               FROM m JOIN activity.sessions s ON s.id = m.sid
              GROUP BY m.g",
        )
        .bind(tracker_id)
        .bind(&m_group)
        .bind(&m_session)
        .bind(&t_group)
        .bind(&t_task)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        rows.into_iter()
            .map(|(g, v)| serde_json::from_value(v).map(|u| (g, u)).map_err(|e| e.to_string()))
            .collect()
    }

    /// Candidate sessions (R1) linked to the tracker's tasks (all, or `only`).
    pub async fn ticket_sessions(
        &self,
        tracker_id: uuid::Uuid,
        only: Option<&[uuid::Uuid]>,
    ) -> Result<Vec<TicketSession>, String> {
        let rows =
            sqlx_core::query_as::query_as::<_, (uuid::Uuid, uuid::Uuid, uuid::Uuid, String)>(
                &format!(
                    "SELECT DISTINCT l.task_id, cs.session_id, cs.repository_id, r.name
                   FROM sensei.session_task_links l
                   JOIN sensei.tracker_tasks t ON t.id = l.task_id
                   JOIN ({candidates}) cs ON cs.session_id = l.session_id
                   JOIN sensei.repositories r ON r.id = cs.repository_id
                  WHERE t.tracker_id = $1 AND ($2::uuid[] IS NULL OR l.task_id = ANY($2))",
                    candidates = candidate_sessions("$1"),
                ),
            )
            .bind(tracker_id)
            .bind(only.map(<[uuid::Uuid]>::to_vec))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(task_id, session_id, repository_id, repository)| TicketSession {
                task_id,
                session_id,
                repository_id,
                repository,
            })
            .collect())
    }

    /// The tracker's non-removed sprints, newest first.
    pub async fn tracker_sprints(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<Vec<StoredSprint>, String> {
        let rows = sqlx_core::query_as::query_as::<
            _,
            (uuid::Uuid, String, String, Option<Ts>, Option<Ts>, Option<Ts>, i64),
        >(
            "SELECT s.id, s.name, s.state, s.start_at, s.end_at, s.complete_at,
                    (SELECT count(*) FROM sensei.tracker_task_sprints m WHERE m.sprint_id = s.id)
               FROM sensei.sprints s
              WHERE s.tracker_id = $1 AND s.removed_at IS NULL
              ORDER BY s.start_at DESC NULLS LAST, s.seq DESC",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(|(id, name, state, start_at, end_at, complete_at, tasks)| StoredSprint {
                id,
                name,
                state,
                start_at,
                end_at,
                complete_at,
                tasks,
            })
            .collect())
    }

    /// `(sprint, task)` for every membership of the tracker's sprints.
    pub async fn sprint_members(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<Vec<(uuid::Uuid, uuid::Uuid)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT m.sprint_id, m.task_id FROM sensei.tracker_task_sprints m
               JOIN sensei.sprints s ON s.id = m.sprint_id WHERE s.tracker_id = $1",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())
    }

    /// Every candidate session started inside one of `windows` (sprint, from, to),
    /// with its part of that sprint.
    pub async fn sprint_window_sessions(
        &self,
        tracker_id: uuid::Uuid,
        windows: &[(uuid::Uuid, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)],
    ) -> Result<Vec<(uuid::Uuid, uuid::Uuid, Part)>, String> {
        let w_sprint: Vec<uuid::Uuid> = windows.iter().map(|w| w.0).collect();
        let w_from: Vec<Ts> = windows.iter().map(|w| w.1).collect();
        let w_to: Vec<Ts> = windows.iter().map(|w| w.2).collect();
        let rows = sqlx_core::query_as::query_as::<_, (uuid::Uuid, uuid::Uuid, String)>(&format!(
            "WITH w AS (SELECT * FROM unnest($2::uuid[], $3::timestamptz[], $4::timestamptz[])
                          AS w(sprint_id, f, t)),
                  c AS (SELECT cs.session_id, s.started_at FROM ({candidates}) cs
                          JOIN activity.sessions s ON s.id = cs.session_id),
                  tl AS (SELECT l.session_id, l.task_id FROM sensei.session_task_links l
                           JOIN sensei.tracker_tasks t ON t.id = l.task_id WHERE t.tracker_id = $1)
             SELECT w.sprint_id, c.session_id,
                    CASE WHEN EXISTS (SELECT 1 FROM tl JOIN sensei.tracker_task_sprints m
                                        ON m.task_id = tl.task_id
                                       WHERE tl.session_id = c.session_id AND m.sprint_id = w.sprint_id)
                         THEN 'linked_sprint'
                         WHEN EXISTS (SELECT 1 FROM tl WHERE tl.session_id = c.session_id)
                         THEN 'linked_other'
                         ELSE 'unlinked' END
               FROM w JOIN c ON c.started_at >= w.f AND c.started_at < w.t",
            candidates = candidate_sessions("$1"),
        ))
        .bind(tracker_id)
        .bind(&w_sprint)
        .bind(&w_from)
        .bind(&w_to)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        rows.into_iter()
            .map(|(sprint, session, part)| {
                let part = match part.as_str() {
                    "linked_sprint" => Part::LinkedSprint,
                    "linked_other" => Part::LinkedOther,
                    "unlinked" => Part::Unlinked,
                    other => return Err(format!("unknown sprint part {other}")),
                };
                Ok((sprint, session, part))
            })
            .collect()
    }

    /// The tracker's tickets (a sprint's members with `sprint`), in natural key
    /// order (prefix, then the trailing number).
    pub async fn tracker_task_rows(
        &self,
        tracker_id: uuid::Uuid,
        scope: TaskScope,
        include_removed: bool,
    ) -> Result<Vec<TaskListRow>, String> {
        let (sprint, backlog) = match scope {
            TaskScope::All => (None, false),
            TaskScope::Sprint(id) => (Some(id), false),
            TaskScope::Backlog => (None, true),
        };
        type Row = (
            uuid::Uuid,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            bool,
            Option<uuid::Uuid>,
            Option<String>,
            Vec<uuid::Uuid>,
            Vec<String>,
        );
        let rows = sqlx_core::query_as::query_as::<_, Row>(
            "SELECT t.id, t.key, t.summary, t.issue_type, t.status, t.removed_at IS NOT NULL,
                    cur.id, cur.name,
                    coalesce(array_agg(sp.id ORDER BY sp.seq) FILTER (WHERE sp.id IS NOT NULL), '{}'),
                    coalesce(array_agg(sp.name ORDER BY sp.seq) FILTER (WHERE sp.id IS NOT NULL), '{}')
               FROM sensei.tracker_tasks t
               LEFT JOIN sensei.sprints cur ON cur.id = t.current_sprint_id
               LEFT JOIN sensei.tracker_task_sprints m ON m.task_id = t.id
               LEFT JOIN sensei.sprints sp ON sp.id = m.sprint_id AND sp.removed_at IS NULL
              WHERE t.tracker_id = $1 AND ($3 OR t.removed_at IS NULL)
                AND ($2::uuid IS NULL OR EXISTS (SELECT 1 FROM sensei.tracker_task_sprints x
                                                  WHERE x.task_id = t.id AND x.sprint_id = $2))
                AND (NOT $4 OR NOT EXISTS (SELECT 1 FROM sensei.tracker_task_sprints x
                                             JOIN sensei.sprints s ON s.id = x.sprint_id AND s.removed_at IS NULL
                                            WHERE x.task_id = t.id))
              GROUP BY t.id, cur.id, cur.name
              ORDER BY regexp_replace(t.key, '[0-9]+$', ''),
                       nullif(substring(t.key FROM '([0-9]+)$'), '')::numeric NULLS FIRST, t.key",
        )
        .bind(tracker_id)
        .bind(sprint)
        .bind(include_removed)
        .bind(backlog)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    key,
                    summary,
                    issue_type,
                    status,
                    removed,
                    cur_id,
                    cur_name,
                    sp_ids,
                    sp_names,
                )| {
                    TaskListRow {
                        id,
                        key,
                        summary,
                        issue_type,
                        status,
                        removed,
                        current_sprint: cur_id.zip(cur_name),
                        sprints: sp_ids.into_iter().zip(sp_names).collect(),
                    }
                },
            )
            .collect())
    }

    /// The tracker's task ids (removed ones included).
    pub async fn tracker_task_ids(
        &self,
        tracker_id: uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, String> {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.tracker_tasks WHERE tracker_id = $1",
        )
        .bind(tracker_id)
        .fetch_all(&self.pool)
        .await
        .map(|rows| rows.into_iter().map(|(id,)| id).collect())
        .map_err(|e| e.to_string())
    }

    /// Sessions by id, oldest first.
    pub async fn sessions_brief(&self, ids: &[uuid::Uuid]) -> Result<Vec<SessionBrief>, String> {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid, Ts, String)>(
            "SELECT id, started_at, task FROM activity.sessions
              WHERE id = ANY($1::uuid[]) ORDER BY started_at, id",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|(id, started_at, task)| SessionBrief { id, started_at, task })
                .collect()
        })
        .map_err(|e| e.to_string())
    }
}
