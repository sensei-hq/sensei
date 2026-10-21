//! Provider-neutral usage per ticket and per sprint (spec R7): what a set of AI
//! sessions cost in active time and tokens. Computed at read; nothing stored.

use std::collections::HashMap;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::tracker_usage::{SessionBrief, StoredSprint};
use crate::tracker::sprint::SprintState;

/// Sessions with at least one link of each source to the counted tasks.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BySource {
    pub branch: i64,
    pub prompt: i64,
}

/// The usage of a distinct set of sessions. A sum over no known values is
/// `None` (never `0`), and every sum carries the count of values it summed.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    pub sessions: i64,
    pub by_source: BySource,
    pub shared: i64,
    pub active_secs: Option<f64>,
    pub active_n: i64,
    pub tokens_fresh: Option<i64>,
    pub tokens_fresh_n: i64,
    pub cache_read: Option<i64>,
    pub cache_read_n: i64,
    pub cache_write: Option<i64>,
    pub cache_write_n: i64,
    pub tokens_out: Option<i64>,
    pub tokens_out_n: i64,
    pub tokens_reasoning: Option<i64>,
    pub tokens_reasoning_n: i64,
    pub metered_cost: Option<f64>,
    pub metered_cost_n: i64,
}

/// A sprint's session window `[from, to)`, or why it has no usage.
pub fn sprint_window(
    state: SprintState,
    start_at: Option<chrono::DateTime<chrono::Utc>>,
    end_at: Option<chrono::DateTime<chrono::Utc>>,
    complete_at: Option<chrono::DateTime<chrono::Utc>>,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>), &'static str> {
    let from = match state {
        SprintState::Future => return Err("not_started"),
        _ => start_at.ok_or("no_dates")?,
    };
    let to = match state {
        SprintState::Active => now,
        _ => complete_at.or(end_at).ok_or("no_dates")?,
    };
    Ok((from, to))
}

/// A ticket's usage within one repository of the workspace.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RepoUsage {
    pub repository_id: uuid::Uuid,
    pub name: String,
    pub usage: Usage,
}

/// A sprint's partition of the candidate sessions in its window (R7).
#[derive(Debug, Clone, PartialEq)]
pub struct SprintParts {
    pub candidates_in_window: i64,
    pub linked_sprint: Usage,
    pub linked_other: Usage,
    pub unlinked: Usage,
}

/// One sprint with its usage, or the reason it has none.
#[derive(Debug, Clone, PartialEq)]
pub struct SprintUsage {
    pub sprint: StoredSprint,
    pub parts: Result<SprintParts, &'static str>,
}

/// Usage of each ticket (all of the tracker's, or `only`): its candidate sessions.
pub async fn ticket_usages(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    only: Option<&[uuid::Uuid]>,
) -> Result<HashMap<uuid::Uuid, Usage>, String> {
    let pairs = pg.ticket_sessions(tracker_id, only).await?;
    let members: Vec<(String, uuid::Uuid)> =
        pairs.iter().map(|p| (p.task_id.to_string(), p.session_id)).collect();
    let targets: Vec<(String, uuid::Uuid)> =
        pairs.iter().map(|p| (p.task_id.to_string(), p.task_id)).collect();
    pg.usage_grouped(tracker_id, &members, &targets)
        .await?
        .into_iter()
        .map(|(g, u)| g.parse().map(|id| (id, u)).map_err(|e| format!("group {g}: {e}")))
        .collect()
}

/// One ticket's usage split by repository; the parts sum to the ticket's total.
pub async fn ticket_usage_by_repo(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    task_id: uuid::Uuid,
) -> Result<Vec<RepoUsage>, String> {
    let pairs = pg.ticket_sessions(tracker_id, Some(&[task_id])).await?;
    let members: Vec<(String, uuid::Uuid)> =
        pairs.iter().map(|p| (p.repository_id.to_string(), p.session_id)).collect();
    let targets: Vec<(String, uuid::Uuid)> =
        pairs.iter().map(|p| (p.repository_id.to_string(), task_id)).collect();
    let mut usage = pg.usage_grouped(tracker_id, &members, &targets).await?;
    let mut repos: Vec<(uuid::Uuid, String)> =
        pairs.into_iter().map(|p| (p.repository_id, p.repository)).collect();
    repos.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    repos.dedup();
    repos
        .into_iter()
        .map(|(repository_id, name)| {
            let usage = usage
                .remove(&repository_id.to_string())
                .ok_or_else(|| format!("no usage for repository {repository_id}"))?;
            Ok(RepoUsage { repository_id, name, usage })
        })
        .collect()
}

/// Every non-removed sprint's partition, newest first.
pub async fn sprint_usages(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<SprintUsage>, String> {
    use crate::db::pg_store::tracker_usage::Part;
    let sprints = pg.tracker_sprints(tracker_id).await?;
    let windows: Vec<(uuid::Uuid, Result<_, &'static str>)> = sprints
        .iter()
        .map(|s| {
            let state = SprintState::parse(&s.state)
                .ok_or_else(|| format!("sprint {} has unknown state {}", s.id, s.state))?;
            Ok((s.id, sprint_window(state, s.start_at, s.end_at, s.complete_at, now)))
        })
        .collect::<Result<_, String>>()?;
    let open: Vec<_> =
        windows.iter().filter_map(|(id, w)| w.as_ref().ok().map(|(f, t)| (*id, *f, *t))).collect();
    let classified = pg.sprint_window_sessions(tracker_id, &open).await?;
    let group = |sprint: uuid::Uuid, part: &str| format!("{sprint}/{part}");
    let name = |part: Part| match part {
        Part::LinkedSprint => "linked_sprint",
        Part::LinkedOther => "linked_other",
        Part::Unlinked => "unlinked",
    };
    let members: Vec<(String, uuid::Uuid)> =
        classified.iter().map(|(sp, s, part)| (group(*sp, name(*part)), *s)).collect();
    // by_source counts links to the sprint's tickets, or to any ticket for linked_other.
    let mut targets: Vec<(String, uuid::Uuid)> = pg
        .sprint_members(tracker_id)
        .await?
        .into_iter()
        .map(|(sp, task)| (group(sp, "linked_sprint"), task))
        .collect();
    let all_tasks = pg.tracker_task_ids(tracker_id).await?;
    for (sp, _, _) in &open {
        targets.extend(all_tasks.iter().map(|t| (group(*sp, "linked_other"), *t)));
    }
    let mut usage = pg.usage_grouped(tracker_id, &members, &targets).await?;
    let mut take = |sp: uuid::Uuid, part: &str| usage.remove(&group(sp, part)).unwrap_or_default();
    Ok(sprints
        .into_iter()
        .zip(windows)
        .map(|(sprint, (id, window))| {
            let parts = window.map(|_| SprintParts {
                candidates_in_window: classified.iter().filter(|(sp, _, _)| *sp == id).count()
                    as i64,
                linked_sprint: take(id, "linked_sprint"),
                linked_other: take(id, "linked_other"),
                unlinked: take(id, "unlinked"),
            });
            SprintUsage { sprint, parts }
        })
        .collect())
}

/// The backlog's ticket count and its all-time usage (R7 backlog usage).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BacklogUsage {
    pub tickets: i64,
    pub usage: Usage,
}

/// The backlog: the tracker's live tickets no live sprint lists, and the usage of
/// the distinct candidate sessions linked to any of them.
pub async fn backlog_usage(pg: &PgStore, tracker_id: uuid::Uuid) -> Result<BacklogUsage, String> {
    use crate::db::pg_store::tracker_usage::TaskScope;
    let tickets: Vec<uuid::Uuid> = pg
        .tracker_task_rows(tracker_id, TaskScope::Backlog, false)
        .await?
        .into_iter()
        .map(|t| t.id)
        .collect();
    let pairs = pg.ticket_sessions(tracker_id, Some(&tickets)).await?;
    // One group: a session linked to two backlog tickets counts once.
    let members: Vec<(String, uuid::Uuid)> =
        pairs.iter().map(|p| (BACKLOG_GROUP.into(), p.session_id)).collect();
    let targets: Vec<(String, uuid::Uuid)> =
        tickets.iter().map(|t| (BACKLOG_GROUP.into(), *t)).collect();
    let usage = pg
        .usage_grouped(tracker_id, &members, &targets)
        .await?
        .remove(BACKLOG_GROUP)
        // No linked session is a genuinely empty set: zero sessions, null sums.
        .unwrap_or_default();
    Ok(BacklogUsage { tickets: tickets.len() as i64, usage })
}

const BACKLOG_GROUP: &str = "backlog";

/// The sprint's `unlinked` sessions; `None` for a sprint the tracker doesn't have,
/// `Some([])` for one without a window.
pub async fn unlinked_in_sprint(
    pg: &PgStore,
    tracker_id: uuid::Uuid,
    sprint_id: uuid::Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Option<Vec<SessionBrief>>, String> {
    use crate::db::pg_store::tracker_usage::Part;
    let Some(sprint) =
        pg.tracker_sprints(tracker_id).await?.into_iter().find(|s| s.id == sprint_id)
    else {
        return Ok(None);
    };
    let Some(state) = SprintState::parse(&sprint.state) else {
        return Err(format!("sprint {sprint_id} has unknown state {}", sprint.state));
    };
    let Ok((from, to)) =
        sprint_window(state, sprint.start_at, sprint.end_at, sprint.complete_at, now)
    else {
        return Ok(Some(Vec::new()));
    };
    let ids: Vec<uuid::Uuid> = pg
        .sprint_window_sessions(tracker_id, &[(sprint_id, from, to)])
        .await?
        .into_iter()
        .filter(|(_, _, part)| *part == Part::Unlinked)
        .map(|(_, s, _)| s)
        .collect();
    pg.sessions_brief(&ids).await.map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};

    fn at(m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, m, d, h, 0, 0).unwrap()
    }

    #[test]
    fn sprint_windows() {
        let now = at(10, 17, 12);
        let (s, e, c) = (Some(at(10, 1, 0)), Some(at(10, 15, 0)), Some(at(10, 15, 10)));
        assert_eq!(
            sprint_window(SprintState::Active, s, e, None, now),
            Ok((at(10, 1, 0), now)),
            "an active sprint runs to now, past its end date"
        );
        assert_eq!(
            sprint_window(SprintState::Closed, s, e, c, now),
            Ok((at(10, 1, 0), at(10, 15, 10)))
        );
        assert_eq!(
            sprint_window(SprintState::Closed, s, e, None, now),
            Ok((at(10, 1, 0), at(10, 15, 0))),
            "no completion date: the end date"
        );
        assert_eq!(sprint_window(SprintState::Future, s, e, None, now), Err("not_started"));
        assert_eq!(sprint_window(SprintState::Active, None, e, None, now), Err("no_dates"));
        assert_eq!(sprint_window(SprintState::Closed, s, None, None, now), Err("no_dates"));
    }

    #[test]
    fn an_empty_set_has_null_sums_and_zero_counts() {
        let v = serde_json::to_value(Usage::default()).unwrap();
        assert_eq!(v["sessions"], 0);
        assert_eq!(v["by_source"], serde_json::json!({"branch": 0, "prompt": 0}));
        for f in [
            "active_secs",
            "tokens_fresh",
            "cache_read",
            "cache_write",
            "tokens_out",
            "tokens_reasoning",
            "metered_cost",
        ] {
            assert!(v[f].is_null(), "{f} is null, not 0");
        }
        assert_eq!(v["active_n"], 0);
        assert!(v.get("tokens_in").is_none(), "tokens_in is not reported");
    }

    // ── Against the database (fake board, seeded sessions) ──────────────────

    use crate::db::pg_store::tracker_usage::TaskScope;
    use crate::tracker::connect::tests::{connected_tracker, workspace};
    use crate::tracker::jira::JiraAdapter;
    use crate::tracker::sync::{SYNC_DEADLINE, TrackerLocks, sync_tracker};
    use crate::tracker::test_support::{FakeBoard, repo_in, session_with};
    use serde_json::json;

    async fn sync(pg: &PgStore, t: uuid::Uuid, board: &FakeBoard) {
        sync_tracker(pg, &TrackerLocks::default(), t, &JiraAdapter, board, SYNC_DEADLINE)
            .await
            .unwrap();
    }

    async fn task_id(pg: &PgStore, t: uuid::Uuid, issue: &str) -> uuid::Uuid {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = $2",
        )
        .bind(t)
        .bind(issue)
        .fetch_one(pg.pool())
        .await
        .unwrap()
        .0
    }

    async fn sprint_id(pg: &PgStore, t: uuid::Uuid, ext: &str) -> uuid::Uuid {
        sqlx_core::query_as::query_as::<_, (uuid::Uuid,)>(
            "SELECT id FROM sensei.sprints WHERE tracker_id = $1 AND external_id = $2",
        )
        .bind(t)
        .bind(ext)
        .fetch_one(pg.pool())
        .await
        .unwrap()
        .0
    }

    /// Set what the ingest would have measured for a session.
    async fn measured(
        pg: &PgStore,
        s: uuid::Uuid,
        started: DateTime<Utc>,
        secs: Option<i64>,
        fresh: Option<i32>,
    ) {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET started_at = $2,
                    duration = make_interval(secs => $3::float8), tokens_fresh = $4
              WHERE id = $1",
        )
        .bind(s)
        .bind(started)
        .bind(secs.map(|v| v as f64))
        .bind(fresh)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn ticket_usage_sums_only_known_values() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, p) = connected_tracker(&pg).await;
        let r = repo_in(&pg, p).await;
        let a = session_with(&pg, Some(r), &["fix/AUTH-2"], &["AUTH-2 again"]).await;
        let b = session_with(&pg, Some(r), &["fix/AUTH-2"], &[]).await;
        let c = session_with(&pg, Some(r), &["fix/AUTH-2"], &[]).await;
        measured(&pg, a, at(10, 2, 9), Some(60), Some(10)).await;
        measured(&pg, b, at(10, 3, 9), Some(120), None).await;
        measured(&pg, c, at(10, 4, 9), None, Some(5)).await;
        sync(&pg, t, &FakeBoard::standard()).await;
        let auth2 = task_id(&pg, t, "10002").await;
        let u = ticket_usages(&pg, t, Some(&[auth2])).await.unwrap().remove(&auth2).unwrap();
        assert_eq!((u.sessions, u.by_source.clone()), (3, BySource { branch: 3, prompt: 1 }));
        assert_eq!((u.active_secs, u.active_n), (Some(180.0), 2), "a NULL duration is not 0");
        assert_eq!((u.tokens_fresh, u.tokens_fresh_n), (Some(15), 2));
        assert_eq!((u.metered_cost, u.metered_cost_n), (None, 0), "nothing known: null, not 0");
        assert_eq!((u.cache_read, u.cache_read_n), (None, 0));
        assert_eq!(u.shared, 0);
        let all = ticket_usages(&pg, t, None).await.unwrap();
        assert_eq!(all.get(&auth2).map(|u| u.sessions), Some(3));
        assert!(!all.contains_key(&task_id(&pg, t, "10003").await), "no sessions: no usage row");
    }

    #[tokio::test]
    async fn a_ticket_counts_every_workspace_repo_and_drops_a_removed_repo_at_once() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, p) = connected_tracker(&pg).await;
        let repos = [repo_in(&pg, p).await, repo_in(&pg, p).await, repo_in(&pg, p).await];
        for r in repos {
            session_with(&pg, Some(r), &["fix/AUTH-2-x"], &[]).await;
        }
        // One of them also worked on AUTH-3: shared.
        session_with(&pg, Some(repos[0]), &["fix/AUTH-2-y"], &["and AUTH-3"]).await;
        sync(&pg, t, &FakeBoard::standard()).await;
        let auth2 = task_id(&pg, t, "10002").await;
        let total = ticket_usages(&pg, t, Some(&[auth2])).await.unwrap()[&auth2].clone();
        assert_eq!((total.sessions, total.shared), (4, 1));
        // That session's AUTH-2 branch is no branch link of AUTH-3's, also when
        // every ticket is read in one query (as the ticket list does).
        let auth3 = task_id(&pg, t, "10003").await;
        let only_prompt = ticket_usages(&pg, t, None).await.unwrap()[&auth3].clone();
        assert_eq!(only_prompt.by_source, BySource { branch: 0, prompt: 1 });
        let parts = ticket_usage_by_repo(&pg, t, auth2).await.unwrap();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts.iter().map(|r| r.usage.sessions).sum::<i64>(), total.sessions);

        // The second repo leaves the workspace: its session drops out, no resync.
        sqlx_core::query::query(
            "DELETE FROM sensei.repositories_in_workspaces
              WHERE workspace_id = $2
                AND repository_id = (SELECT repository_id FROM sensei.folders WHERE id = $1)",
        )
        .bind(repos[1])
        .bind(p)
        .execute(pg.pool())
        .await
        .unwrap();
        assert_eq!(ticket_usages(&pg, t, Some(&[auth2])).await.unwrap()[&auth2].sessions, 3);
        assert_eq!(ticket_usage_by_repo(&pg, t, auth2).await.unwrap().len(), 2);
    }

    /// Sprints 100 (closed, 09-01 → completed 09-15 10:00) and 101 (active from
    /// 10-01, ends 10-15); 10001 sits in 100, 10002 + 10003 in 101.
    #[tokio::test]
    async fn a_sprint_partitions_the_candidates_in_its_window() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, p) = connected_tracker(&pg).await;
        let r = repo_in(&pg, p).await;
        let s = |b: &'static [&'static str], pr: &'static [&'static str]| {
            let pg = pg.clone();
            async move { session_with(&pg, Some(r), b, pr).await }
        };
        let in_sprint = s(&["fix/AUTH-2"], &[]).await;
        let after_end = s(&["fix/AUTH-3"], &[]).await;
        let both = s(&["fix/AUTH-2"], &["AUTH-3 too"]).await;
        let other_sprint = s(&["fix/AUTH-1"], &[]).await;
        let no_key = s(&["main"], &["refactor"]).await;
        let other_tracker = s(&["main"], &[]).await;
        let in_100 = s(&["fix/AUTH-1"], &[]).await;
        let outside_workspace =
            session_with(&pg, Some(repo_in(&pg, workspace(&pg).await).await), &["fix/AUTH-2"], &[])
                .await;
        for (id, when) in [
            (in_sprint, at(10, 5, 9)),
            (after_end, at(10, 16, 9)),
            (both, at(10, 5, 10)),
            (other_sprint, at(10, 6, 9)),
            (no_key, at(10, 7, 9)),
            (other_tracker, at(10, 8, 9)),
            (in_100, at(9, 5, 9)),
            (outside_workspace, at(10, 5, 9)),
        ] {
            measured(&pg, id, when, Some(60), None).await;
        }
        sync(&pg, t, &FakeBoard::standard()).await;
        // Linked only to another tracker's ticket.
        let (t2, _) = connected_tracker(&pg).await;
        sync(&pg, t2, &FakeBoard::standard()).await;
        sqlx_core::query::query(
            "INSERT INTO sensei.session_task_links (session_id, task_id, source, matched_key, mentions)
             VALUES ($1, $2, 'branch', 'AUTH-2', 1)",
        )
        .bind(other_tracker)
        .bind(task_id(&pg, t2, "10002").await)
        .execute(&pg.pool().clone())
        .await
        .unwrap();

        let now = Utc::now();
        let sprints = sprint_usages(&pg, t, now).await.unwrap();
        assert_eq!(
            sprints.iter().map(|s| s.sprint.name.as_str()).collect::<Vec<_>>(),
            vec!["Sprint 2", "Sprint 1"],
            "newest first"
        );
        let s101 = sprints[0].parts.clone().unwrap();
        let (independent,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM activity.sessions s
               JOIN sensei.folder_workspaces fp ON fp.folder_id = s.repo_folder_id
              WHERE fp.workspace_id = $1 AND s.started_at >= '2026-10-01' AND s.started_at < $2",
        )
        .bind(p)
        .bind(now)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(s101.candidates_in_window, independent);
        assert_eq!(independent, 6);
        assert_eq!(
            (s101.linked_sprint.sessions, s101.linked_sprint.shared),
            (3, 1),
            "after its end date an active sprint still counts; a two-ticket session counts once"
        );
        assert_eq!(s101.linked_sprint.active_secs, Some(180.0));
        assert_eq!(s101.linked_other.sessions, 1);
        assert_eq!(
            s101.unlinked.sessions, 2,
            "a link to another tracker's ticket is unlinked here"
        );
        let parts =
            s101.linked_sprint.sessions + s101.linked_other.sessions + s101.unlinked.sessions;
        assert_eq!(parts, s101.candidates_in_window, "the parts partition the window");
        let s100 = sprints[1].parts.clone().unwrap();
        assert_eq!((s100.candidates_in_window, s100.linked_sprint.sessions), (1, 1));

        let s101_id = sprint_id(&pg, t, "101").await;
        let mut unlinked: Vec<uuid::Uuid> = unlinked_in_sprint(&pg, t, s101_id, now)
            .await
            .unwrap()
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        unlinked.sort();
        let mut want = vec![no_key, other_tracker];
        want.sort();
        assert_eq!(unlinked, want);
        assert_eq!(unlinked_in_sprint(&pg, t, uuid::Uuid::new_v4(), now).await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_sprint_without_a_window_has_no_usage_not_zero() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, _p) = connected_tracker(&pg).await;
        let board = FakeBoard::standard()
            .with("/rest/agile/1.0/board/42/sprint", 0, json!({"isLast": true, "values": [
                {"id": 102, "name": "Sprint 3", "state": "future", "startDate": "2026-10-20T00:00:00.000Z"},
                {"id": 103, "name": "Undated", "state": "active"}]}))
            .with("/rest/agile/1.0/board/42/sprint/102/issue", 0, json!({"total": 0, "issues": []}))
            .with("/rest/agile/1.0/board/42/sprint/103/issue", 0, json!({"total": 0, "issues": []}));
        sync(&pg, t, &board).await;
        let sprints = sprint_usages(&pg, t, Utc::now()).await.unwrap();
        let reason = |name: &str| {
            sprints.iter().find(|s| s.sprint.name == name).map(|s| s.parts.clone().err())
        };
        assert_eq!(reason("Sprint 3"), Some(Some("not_started")));
        assert_eq!(reason("Undated"), Some(Some("no_dates")));
    }

    #[tokio::test]
    async fn the_ticket_list_filters_by_sprint_and_shows_carried_tickets_in_both() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, _p) = connected_tracker(&pg).await;
        // 10001 was in closed 100 and is carried into active 101.
        let carried = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/sprint/101/issue",
            0,
            json!({"total": 3, "issues": [
                {"id": "10001", "key": "AUTH-1", "fields": {}},
                {"id": "10002", "key": "AUTH-2", "fields": {}},
                {"id": "10003", "key": "AUTH-3", "fields": {}}]}),
        );
        sync(&pg, t, &carried).await;
        let s100 = sprint_id(&pg, t, "100").await;
        let s101 = sprint_id(&pg, t, "101").await;
        let keys = |rows: Vec<crate::db::pg_store::tracker_usage::TaskListRow>| {
            rows.into_iter().map(|r| r.key).collect::<Vec<_>>()
        };
        assert_eq!(
            keys(pg.tracker_task_rows(t, TaskScope::Sprint(s100), false).await.unwrap()),
            vec!["AUTH-1"]
        );
        assert_eq!(
            keys(pg.tracker_task_rows(t, TaskScope::Sprint(s101), false).await.unwrap()),
            vec!["AUTH-1", "AUTH-2", "AUTH-3"]
        );
        let all = pg.tracker_task_rows(t, TaskScope::All, false).await.unwrap();
        let auth1 = all.iter().find(|r| r.key == "AUTH-1").unwrap();
        assert_eq!(auth1.current_sprint, Some((s101, "Sprint 2".to_string())));
        let mut listed: Vec<_> = auth1.sprints.iter().map(|(id, _)| *id).collect();
        listed.sort();
        let mut want = vec![s100, s101];
        want.sort();
        assert_eq!(listed, want);

        // A removed ticket is hidden unless asked for.
        let without_3 = carried.with(
            "/rest/agile/1.0/board/42/issue",
            2,
            json!({"startAt": 2, "total": 2, "issues": []}),
        );
        sync(&pg, t, &without_3).await;
        assert_eq!(
            keys(pg.tracker_task_rows(t, TaskScope::All, false).await.unwrap()),
            vec!["AUTH-1", "AUTH-2"]
        );
        let with_removed = pg.tracker_task_rows(t, TaskScope::All, true).await.unwrap();
        assert!(with_removed.iter().any(|r| r.key == "AUTH-3" && r.removed));
    }

    /// The standard board plus AUTH-4 (no sprint) and AUTH-5 (only in future
    /// sprint 102); `with_102 = false` is the same board after 102 is deleted.
    fn with_backlog(with_102: bool, with_4: bool) -> FakeBoard {
        let mut issues = vec![json!({"id": "10003", "key": "AUTH-3", "fields": {}})];
        if with_4 {
            issues
                .push(json!({"id": "10004", "key": "AUTH-4", "fields": {"summary": "no sprint"}}));
        }
        issues.push(json!({"id": "10005", "key": "AUTH-5", "fields": {}}));
        let total = 2 + issues.len();
        let mut sprints = vec![
            json!({"id": 100, "name": "Sprint 1", "state": "closed",
                   "startDate": "2026-09-01T00:00:00.000Z", "endDate": "2026-09-15T00:00:00.000Z"}),
            json!({"id": 101, "name": "Sprint 2", "state": "active", "startDate": "2026-10-01T00:00:00.000Z"}),
        ];
        if with_102 {
            sprints.push(json!({"id": 102, "name": "Sprint 3", "state": "future"}));
        }
        FakeBoard::standard()
            .with(
                "/rest/agile/1.0/board/42/issue",
                2,
                json!({"startAt": 2, "total": total, "issues": issues}),
            )
            .with("/rest/agile/1.0/board/42/sprint", 0, json!({"isLast": true, "values": sprints}))
            .with(
                "/rest/agile/1.0/board/42/sprint/102/issue",
                0,
                json!({"total": 1, "issues": [
                {"id": "10005", "key": "AUTH-5", "fields": {}}]}),
            )
    }

    #[tokio::test]
    async fn the_backlog_is_the_tickets_no_live_sprint_lists() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, _p) = connected_tracker(&pg).await;
        let keys = |rows: Vec<crate::db::pg_store::tracker_usage::TaskListRow>| {
            rows.into_iter().map(|r| r.key).collect::<Vec<_>>()
        };
        sync(&pg, t, &with_backlog(true, true)).await;
        assert_eq!(
            keys(pg.tracker_task_rows(t, TaskScope::Backlog, false).await.unwrap()),
            vec!["AUTH-4"],
            "AUTH-1 is in a closed sprint, AUTH-5 in a future one: neither is backlog"
        );
        // Sprint 102 is deleted in Jira: its ticket falls back to the backlog.
        sync(&pg, t, &with_backlog(false, true)).await;
        assert_eq!(
            keys(pg.tracker_task_rows(t, TaskScope::Backlog, false).await.unwrap()),
            vec!["AUTH-4", "AUTH-5"]
        );
        // AUTH-4 leaves the board: a removed ticket shows only when asked for.
        sync(&pg, t, &with_backlog(false, false)).await;
        assert_eq!(
            keys(pg.tracker_task_rows(t, TaskScope::Backlog, false).await.unwrap()),
            vec!["AUTH-5"]
        );
        assert_eq!(
            keys(pg.tracker_task_rows(t, TaskScope::Backlog, true).await.unwrap()),
            vec!["AUTH-4", "AUTH-5"]
        );
    }

    #[tokio::test]
    async fn backlog_usage_counts_each_session_once() {
        let Ok(pg) = PgStore::connect_test().await else { return };
        let (t, p) = connected_tracker(&pg).await;
        let r = repo_in(&pg, p).await;
        let both = session_with(&pg, Some(r), &["fix/AUTH-4"], &["and AUTH-5"]).await;
        let sprint_only = session_with(&pg, Some(r), &["fix/AUTH-2"], &[]).await;
        measured(&pg, both, at(10, 5, 9), Some(600), Some(10)).await;
        measured(&pg, sprint_only, at(10, 5, 10), Some(60), Some(1)).await;
        sync(&pg, t, &with_backlog(false, true)).await;
        let b = backlog_usage(&pg, t).await.unwrap();
        assert_eq!(b.tickets, 2, "AUTH-4 and AUTH-5");
        assert_eq!(
            (b.usage.sessions, b.usage.shared),
            (1, 1),
            "one session, linked to two backlog tickets"
        );
        assert_eq!(b.usage.by_source, BySource { branch: 1, prompt: 1 });
        assert_eq!(
            (b.usage.active_secs, b.usage.tokens_fresh),
            (Some(600.0), Some(10)),
            "the sprint ticket's session is not counted"
        );
    }
}
