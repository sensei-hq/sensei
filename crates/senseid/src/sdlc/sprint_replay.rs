//! A sprint's commitment, scope change and completion, replayed from its tickets'
//! tracker history (spec S5) — never read from today's membership, which the
//! tracker overwrites. Pure; times are Unix milliseconds.

use std::collections::{BTreeSet, HashMap};

/// What a history row changed (`sensei.tracked_change_field`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeField {
    Sprint,
    Status,
    Estimate,
}

/// One history row: sprint ids comma-separated, a status id, or points as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub field: ChangeField,
    pub at: i64,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// A ticket as the replay needs it: its current state, and its history.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReplayTask {
    pub created_at: Option<i64>,
    /// The sprints that list it now (their external ids).
    pub current_sprints: Vec<String>,
    pub current_category: Option<String>,
    pub current_points: Option<f64>,
    pub changes: Vec<Change>,
    /// From when its history is unread: `None` when read through its last update,
    /// `i64::MIN` when never read.
    pub history_gap_from: Option<i64>,
}

/// A sprint of the tracker: its id and creation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprintRef {
    pub external_id: String,
    pub seq: i64,
}

/// The sets of spec S5, as counts, with points sums over parseable values.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct SprintFacts {
    pub committed: i64,
    pub added: i64,
    pub removed: i64,
    pub completed_committed: i64,
    pub completed_added: i64,
    pub carried_over: i64,
    pub committed_points: Option<f64>,
    pub committed_points_n: i64,
    pub completed_committed_points: Option<f64>,
    pub completed_committed_points_n: i64,
    pub completed_added_points: Option<f64>,
    pub completed_added_points_n: i64,
    pub points_unparsed: i64,
    /// Tickets whose status at `E` is not one of the board's statuses.
    pub unknown_status: i64,
}

/// Replay `sprint` over the window `[start, end]` (spec S5).
pub fn replay(
    sprint: &SprintRef,
    window: (i64, i64),
    sprints: &[SprintRef],
    tasks: &[ReplayTask],
    statuses: &HashMap<String, String>,
) -> SprintFacts {
    let (start, end) = window;
    let later: BTreeSet<&str> =
        sprints.iter().filter(|s| s.seq > sprint.seq).map(|s| s.external_id.as_str()).collect();
    let id = sprint.external_id.as_str();
    let mut f = SprintFacts::default();
    let mut unparsed: BTreeSet<usize> = BTreeSet::new();
    let mut sums: [(f64, i64); 3] = [(0.0, 0); 3];
    for (i, task) in tasks.iter().enumerate() {
        let in_at = |t: i64| membership(task, t).contains(id);
        let sprint_times: Vec<i64> =
            task.changes.iter().filter(|c| c.field == ChangeField::Sprint).map(|c| c.at).collect();
        let committed = in_at(start);
        let at_end = in_at(end);
        let added = !committed
            && (at_end
                || sprint_times
                    .iter()
                    .chain(task.created_at.iter())
                    .any(|&t| t > start && t <= end && in_at(t)));
        let removed = !at_end
            && std::iter::once(start)
                .chain(sprint_times.iter().copied())
                .chain(task.created_at)
                .any(|t| t >= start && t < end && in_at(t));
        let done = at_end
            && match done_at(task, end, statuses) {
                Some(d) => d,
                None => {
                    f.unknown_status += 1;
                    false
                }
            };
        let carried = at_end
            && !done
            && std::iter::once(end)
                .chain(sprint_times.iter().copied().filter(|&t| t > end))
                .chain(std::iter::once(i64::MAX))
                .any(|t| membership(task, t).iter().any(|s| later.contains(s.as_str())));
        f.committed += i64::from(committed);
        f.added += i64::from(added);
        f.removed += i64::from(removed);
        f.completed_committed += i64::from(committed && done);
        f.completed_added += i64::from(added && done);
        f.carried_over += i64::from(carried);
        let mut sum = |slot: usize, t: i64| match points_at(task, t) {
            Points::Value(p) => {
                sums[slot].0 += p;
                sums[slot].1 += 1;
            }
            Points::Unparsed => {
                unparsed.insert(i);
            }
            Points::None => {}
        };
        if committed {
            sum(0, start);
        }
        if committed && done {
            sum(1, end);
        }
        if added && done {
            sum(2, end);
        }
    }
    let total = |(sum, n): (f64, i64)| (n > 0).then_some(sum);
    (f.committed_points, f.committed_points_n) = (total(sums[0]), sums[0].1);
    (f.completed_committed_points, f.completed_committed_points_n) = (total(sums[1]), sums[1].1);
    (f.completed_added_points, f.completed_added_points_n) = (total(sums[2]), sums[2].1);
    f.points_unparsed = unparsed.len() as i64;
    f
}

/// A field's value at `t`: the earliest change's `from` (or `current` when the
/// field never changed), then every change at or before `t`.
fn value_at(
    task: &ReplayTask,
    field: ChangeField,
    t: i64,
    current: Option<String>,
) -> Option<String> {
    let mut changes: Vec<&Change> = task.changes.iter().filter(|c| c.field == field).collect();
    if changes.is_empty() {
        return current;
    }
    changes.sort_by_key(|c| c.at);
    let mut value = changes[0].from.clone();
    for c in changes.iter().take_while(|c| c.at <= t) {
        value = c.to.clone();
    }
    value
}

/// The sprints listing the ticket at `t`; none before it was created.
fn membership(task: &ReplayTask, t: i64) -> BTreeSet<String> {
    if task.created_at.is_some_and(|c| c > t) {
        return BTreeSet::new();
    }
    let current = (!task.current_sprints.is_empty()).then(|| task.current_sprints.join(","));
    value_at(task, ChangeField::Sprint, t, current)
        .map(|v| v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect())
        .unwrap_or_default()
}

/// Whether the ticket's status at `t` is in the `done` category; `None` when that
/// status is not one of the board's (or the history names none).
fn done_at(task: &ReplayTask, t: i64, statuses: &HashMap<String, String>) -> Option<bool> {
    let changed = task.changes.iter().any(|c| c.field == ChangeField::Status);
    if !changed {
        return Some(task.current_category.as_deref() == Some("done"));
    }
    value_at(task, ChangeField::Status, t, None)
        .and_then(|id| statuses.get(&id))
        .map(|c| c == "done")
}

/// Whether a ticket that could be in a sprint ending at `end` has history unread
/// before `end`, so the replay cannot be trusted (A8).
pub fn history_pending(tasks: &[ReplayTask], end: i64) -> bool {
    tasks.iter().any(|t| {
        t.created_at.is_none_or(|c| c <= end) && t.history_gap_from.is_some_and(|g| g < end)
    })
}

enum Points {
    Value(f64),
    Unparsed,
    None,
}

fn points_at(task: &ReplayTask, t: i64) -> Points {
    let changed = task.changes.iter().any(|c| c.field == ChangeField::Estimate);
    if !changed {
        return task.current_points.map_or(Points::None, Points::Value);
    }
    match value_at(task, ChangeField::Estimate, t, None) {
        None => Points::None,
        Some(text) => text.trim().parse::<f64>().map_or(Points::Unparsed, Points::Value),
    }
}

/// Population standard deviation of the last 6 closed sprints' commitment
/// reliability that have one (spec S6), newest first; `Err` below 2.
pub fn predictability(reliabilities: &[Option<f64>]) -> Result<f64, &'static str> {
    let values: Vec<f64> = reliabilities.iter().flatten().take(6).copied().collect();
    if values.len() < 2 {
        return Err("too_few_sprints");
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    Ok((values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt())
}

/// A sprint's figures (spec S6 "Sprint"). `reasons` explains every `None`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct SprintSdlc {
    pub commitment_reliability: Option<f64>,
    pub total_completion: Option<f64>,
    pub scope_change: Option<f64>,
    pub carry_over_rate: Option<f64>,
    pub unplanned_work_ratio: Option<f64>,
    pub velocity: Option<f64>,
    pub points_commitment_reliability: Option<f64>,
    pub facts: Option<SprintFacts>,
    pub reasons: std::collections::BTreeMap<&'static str, &'static str>,
}

/// The sprint's figures from its replay, or from why it has no window (R7's
/// `not_started` / `no_dates`). `estimation` is the board's: `Err(reason)` when
/// its points are not to be used (`not_synced`, `no_estimation_field`).
pub fn sprint_figures(
    facts: Result<SprintFacts, &'static str>,
    estimation: Result<(), &'static str>,
) -> SprintSdlc {
    const KEYS: [&str; 7] = [
        "commitment_reliability",
        "total_completion",
        "scope_change",
        "carry_over_rate",
        "unplanned_work_ratio",
        "velocity",
        "points_commitment_reliability",
    ];
    let mut s = SprintSdlc::default();
    let f = match facts {
        Ok(f) => f,
        Err(why) => {
            for k in KEYS {
                s.reasons.insert(k, why);
            }
            return s;
        }
    };
    let pct = |n: i64, d: i64| (d > 0).then(|| n as f64 / d as f64 * 100.0);
    let put = |s: &mut SprintSdlc, key: &'static str, v: Option<f64>, why: &'static str| {
        if v.is_none() {
            s.reasons.insert(key, why);
        }
        v
    };
    s.commitment_reliability = put(
        &mut s,
        "commitment_reliability",
        pct(f.completed_committed, f.committed),
        "no_commitment",
    );
    s.total_completion = put(
        &mut s,
        "total_completion",
        pct(f.completed_committed + f.completed_added, f.committed),
        "no_commitment",
    );
    s.scope_change =
        put(&mut s, "scope_change", pct(f.added + f.removed, f.committed), "no_commitment");
    s.carry_over_rate =
        put(&mut s, "carry_over_rate", pct(f.carried_over, f.committed), "no_commitment");
    s.unplanned_work_ratio = put(
        &mut s,
        "unplanned_work_ratio",
        pct(f.completed_added, f.completed_committed + f.completed_added),
        "nothing_completed",
    );
    match estimation {
        Err(why) => {
            s.reasons.insert("velocity", why);
            s.reasons.insert("points_commitment_reliability", why);
        }
        Ok(()) => {
            // A8: nothing completed is a velocity of 0, not "not estimated".
            let velocity = match (f.completed_committed_points, f.completed_added_points) {
                _ if f.completed_committed + f.completed_added == 0 => Some(0.0),
                (None, None) => None,
                (a, b) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0)),
            };
            s.velocity = put(&mut s, "velocity", velocity, "not_estimated");
            let done = match f.completed_committed_points {
                _ if f.completed_committed == 0 => Some(0.0),
                d => d,
            };
            s.points_commitment_reliability = match (done, f.committed_points) {
                _ if f.committed == 0 => {
                    put(&mut s, "points_commitment_reliability", None, "no_commitment")
                }
                (Some(done), Some(all)) if all > 0.0 => Some(done / all * 100.0),
                (Some(_), Some(_)) => {
                    put(&mut s, "points_commitment_reliability", None, "no_commitment")
                }
                _ => put(&mut s, "points_commitment_reliability", None, "not_estimated"),
            };
        }
    }
    if f.unknown_status > 0 {
        // Completion is unknown; only the membership-based scope change stands.
        for k in KEYS.iter().filter(|k| **k != "scope_change") {
            s.reasons.insert(k, "unknown_status");
        }
        s.commitment_reliability = None;
        s.total_completion = None;
        s.carry_over_rate = None;
        s.unplanned_work_ratio = None;
        s.velocity = None;
        s.points_commitment_reliability = None;
    }
    s.facts = Some(f);
    s
}

/// The tracker's figures (spec S6 "Project"). `reasons` explains every `None`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct ProjectSdlc {
    pub predictability: Option<f64>,
    pub tokens_per_ticket: Option<f64>,
    pub prompts_per_ticket: Option<f64>,
    /// Tickets with at least one linked session: the per-ticket denominator.
    pub tickets_with_sessions: i64,
    /// Sessions in the union lacking a token field (`partial_tokens`).
    pub sessions_without_tokens: i64,
    pub reasons: std::collections::BTreeMap<&'static str, &'static str>,
}

/// Every non-removed sprint's figures on a tracker, by sprint id (S5, S6).
pub async fn sprint_sdlc(
    pg: &crate::db::pg_store::PgStore,
    tracker_id: uuid::Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<HashMap<uuid::Uuid, SprintSdlc>, String> {
    let inputs = pg.replay_inputs(tracker_id).await?;
    figures_by_sprint(&inputs, now, crate::sdlc::reads_history(pg, tracker_id).await?)
}

/// `history`: whether the tracker's adapter reads ticket history at all (§G8). Every
/// sprint figure is a replay of it, so without it each reads `unsupported_by_tracker`.
fn figures_by_sprint(
    inputs: &crate::db::pg_store::tracker_history::ReplayInputs,
    now: chrono::DateTime<chrono::Utc>,
    history: bool,
) -> Result<HashMap<uuid::Uuid, SprintSdlc>, String> {
    use crate::tracker::sprint::SprintState;
    let refs: Vec<SprintRef> = inputs
        .sprints
        .iter()
        .map(|s| SprintRef { external_id: s.external_id.clone(), seq: s.seq })
        .collect();
    inputs
        .sprints
        .iter()
        .map(|s| {
            let state = SprintState::parse(&s.state)
                .ok_or_else(|| format!("sprint {} has unknown state {}", s.id, s.state))?;
            // R7's window and reasons; `not_started` is named for the sprint (A7).
            let window = crate::tracker::usage::sprint_window(
                state,
                s.start_at,
                s.end_at,
                s.complete_at,
                now,
            )
            .map_err(|why| if why == "not_started" { "sprint_not_started" } else { why });
            let window = window.and_then(|(from, to)| {
                if !history {
                    Err("unsupported_by_tracker")
                } else if history_pending(&inputs.tasks, to.timestamp_millis()) {
                    Err("history_pending")
                } else {
                    Ok((from, to))
                }
            });
            let facts = window.map(|(from, to)| {
                let me = SprintRef { external_id: s.external_id.clone(), seq: s.seq };
                replay(
                    &me,
                    (from.timestamp_millis(), to.timestamp_millis()),
                    &refs,
                    &inputs.tasks,
                    &inputs.statuses,
                )
            });
            Ok((s.id, sprint_figures(facts, inputs.estimation)))
        })
        .collect()
}

/// Why predictability is absent: a tracker without history never has it (§G8).
fn predictability_reason(history: bool, why: &'static str) -> &'static str {
    if history { why } else { "unsupported_by_tracker" }
}

/// The tracker's project figures (S6): predictability over its last six closed
/// sprints, and tokens and prompts per ticket over the distinct union of its
/// tickets' sessions (a session shared by two tickets counts once).
pub async fn project_sdlc(
    pg: &crate::db::pg_store::PgStore,
    tracker_id: uuid::Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<ProjectSdlc, String> {
    let mut p = ProjectSdlc::default();
    let inputs = pg.replay_inputs(tracker_id).await?;
    let history = crate::sdlc::reads_history(pg, tracker_id).await?;
    let by_sprint = figures_by_sprint(&inputs, now, history)?;
    let closed: Vec<Option<f64>> = inputs
        .sprints
        .iter()
        .filter(|s| s.state == "closed")
        .map(|s| by_sprint.get(&s.id).and_then(|f| f.commitment_reliability))
        .collect();
    match predictability(&closed) {
        Ok(v) => p.predictability = Some(v),
        Err(why) => {
            p.reasons.insert("predictability", predictability_reason(history, why));
        }
    }

    let pairs = pg.ticket_sessions(tracker_id, None).await?;
    let tickets: BTreeSet<uuid::Uuid> = pairs.iter().map(|x| x.task_id).collect();
    let sessions: Vec<uuid::Uuid> =
        pairs.iter().map(|x| x.session_id).collect::<BTreeSet<_>>().into_iter().collect();
    p.tickets_with_sessions = tickets.len() as i64;
    if tickets.is_empty() {
        p.reasons.insert("tokens_per_ticket", "no_usage");
        p.reasons.insert("prompts_per_ticket", "no_usage");
        return Ok(p);
    }
    let n = tickets.len() as f64;
    let (tokens, missing) = pg.session_token_totals(&sessions).await?;
    p.sessions_without_tokens = missing;
    if missing > 0 {
        p.reasons.insert("tokens_per_ticket", "partial_tokens");
    } else {
        p.tokens_per_ticket = Some(tokens as f64 / n);
    }
    let hash = pg.command_catalogue().await?.hash;
    let signals = pg.session_signals_for(&sessions, &hash).await?;
    if let Some(why) = crate::sdlc::ticket::underived(signals.iter().map(|(_, s)| s)) {
        p.reasons.insert("prompts_per_ticket", why);
    } else {
        let prompts = signals
            .iter()
            .flat_map(|(_, s)| s.signals.iter())
            .filter(|x| x.kind == crate::sdlc::signals::SignalKind::Prompt)
            .count();
        p.prompts_per_ticket = Some(prompts as f64 / n);
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tracker_without_history_reads_unsupported_on_every_sprint_key() {
        // §G8: not "history_pending" (coming) — this tracker never supplies it.
        let inputs = crate::db::pg_store::tracker_history::ReplayInputs {
            sprints: vec![crate::db::pg_store::tracker_history::ReplaySprint {
                id: uuid::Uuid::nil(),
                external_id: "milestone:7".into(),
                seq: 7,
                state: "closed".into(),
                start_at: Some(chrono::DateTime::from_timestamp(0, 0).unwrap()),
                end_at: Some(chrono::DateTime::from_timestamp(86_400 * 14, 0).unwrap()),
                complete_at: Some(chrono::DateTime::from_timestamp(86_400 * 14, 0).unwrap()),
            }],
            tasks: vec![],
            statuses: Default::default(),
            estimation: Err("not_synced"),
        };
        let now = chrono::DateTime::from_timestamp(86_400 * 30, 0).unwrap();
        let f = &figures_by_sprint(&inputs, now, false).unwrap()[&uuid::Uuid::nil()];
        assert_eq!(f.reasons.len(), 7, "{:?}", f.reasons);
        assert!(f.reasons.values().all(|r| *r == "unsupported_by_tracker"), "{:?}", f.reasons);
        assert!(registered_reason("unsupported_by_tracker"));
        let with_history = &figures_by_sprint(&inputs, now, true).unwrap()[&uuid::Uuid::nil()];
        assert!(!with_history.reasons.values().any(|r| *r == "unsupported_by_tracker"));
    }

    #[test]
    fn predictability_without_history_is_unsupported() {
        assert_eq!(predictability_reason(false, "too_few_sprints"), "unsupported_by_tracker");
        assert_eq!(predictability_reason(true, "too_few_sprints"), "too_few_sprints");
    }

    fn registered_reason(code: &str) -> bool {
        crate::sdlc::registered_reasons().contains(code)
    }

    const DAY: i64 = 86_400_000;

    fn change(field: ChangeField, day: i64, from: &str, to: &str) -> Change {
        let v = |s: &str| (!s.is_empty()).then(|| s.to_string());
        Change { field, at: day * DAY, from: v(from), to: v(to) }
    }
    fn statuses() -> HashMap<String, String> {
        HashMap::from([("1".into(), "new".into()), ("3".into(), "done".into())])
    }
    fn sprints() -> Vec<SprintRef> {
        [("4", 4), ("5", 5), ("6", 6)]
            .into_iter()
            .map(|(id, seq)| SprintRef { external_id: id.into(), seq })
            .collect()
    }
    fn task(created: i64, now_in: &[&str], category: &str, changes: Vec<Change>) -> ReplayTask {
        ReplayTask {
            created_at: Some(created * DAY),
            current_sprints: now_in.iter().map(|s| s.to_string()).collect(),
            current_category: Some(category.into()),
            current_points: None,
            changes,
            history_gap_from: None,
        }
    }

    /// Spec gate 6: Sprint 5 runs day 0 to day 14.
    fn gate6() -> Vec<ReplayTask> {
        use ChangeField::{Sprint, Status};
        vec![
            // i1: in from day -1, done day 10.
            task(
                -5,
                &["5"],
                "done",
                vec![change(Sprint, -1, "", "5"), change(Status, 10, "1", "3")],
            ),
            // i2: added day 3, done day 12.
            task(
                -5,
                &["5"],
                "done",
                vec![change(Sprint, 3, "", "5"), change(Status, 12, "1", "3")],
            ),
            // i3: in from day -1, removed day 5.
            task(-5, &[], "new", vec![change(Sprint, -1, "", "5"), change(Sprint, 5, "5", "")]),
            // i4: in from day -1, not done, gains sprint 6 at the end.
            task(
                -5,
                &["5", "6"],
                "new",
                vec![change(Sprint, -1, "", "5"), change(Sprint, 14, "5", "5,6")],
            ),
            // i5: in sprints 4 and 5, not done, sent to the backlog: no change row.
            task(-30, &["4", "5"], "new", vec![change(Sprint, -1, "4", "4,5")]),
            // i6: created on day 2 directly in sprint 5: no change row.
            task(2, &["5"], "new", vec![]),
        ]
    }

    #[test]
    fn gate_six_sets() {
        let s5 = SprintRef { external_id: "5".into(), seq: 5 };
        let f = replay(&s5, (0, 14 * DAY), &sprints(), &gate6(), &statuses());
        assert_eq!(
            (
                f.committed,
                f.added,
                f.removed,
                f.completed_committed,
                f.completed_added,
                f.carried_over
            ),
            (4, 2, 1, 1, 1, 1)
        );
    }

    #[test]
    fn points_at_the_relevant_time() {
        use ChangeField::{Estimate, Sprint, Status};
        let s5 = SprintRef { external_id: "5".into(), seq: 5 };
        let tasks = vec![
            // Committed with 3 at the start, re-estimated to 5 mid-sprint, done.
            ReplayTask {
                current_points: Some(5.0),
                ..task(
                    -5,
                    &["5"],
                    "done",
                    vec![
                        change(Sprint, -1, "", "5"),
                        change(Estimate, 2, "3", "5"),
                        change(Status, 10, "1", "3"),
                    ],
                )
            },
            // Committed with junk points.
            ReplayTask {
                current_points: None,
                ..task(
                    -5,
                    &["5"],
                    "new",
                    vec![change(Sprint, -1, "", "5"), change(Estimate, -2, "", "abc")],
                )
            },
        ];
        let f = replay(&s5, (0, 14 * DAY), &sprints(), &tasks, &statuses());
        assert_eq!(
            (f.committed_points, f.committed_points_n, f.points_unparsed),
            (Some(3.0), 1, 1)
        );
        assert_eq!((f.completed_committed_points, f.completed_committed_points_n), (Some(5.0), 1));
        assert_eq!(
            (f.completed_added_points, f.completed_added_points_n),
            (None, 0),
            "a sum over nothing is null"
        );
    }

    #[test]
    fn predictability_over_the_last_six() {
        assert_eq!(
            predictability(&[Some(50.0), Some(100.0), None, Some(50.0), Some(100.0)]),
            Ok(25.0)
        );
        assert_eq!(predictability(&[Some(50.0)]), Err("too_few_sprints"));
        let seven = [Some(0.0), Some(0.0), Some(0.0), Some(0.0), Some(0.0), Some(0.0), Some(100.0)];
        assert_eq!(predictability(&seven), Ok(0.0), "only the newest six count");
    }

    #[test]
    fn gate_six_rates() {
        let s5 = SprintRef { external_id: "5".into(), seq: 5 };
        let f = replay(&s5, (0, 14 * DAY), &sprints(), &gate6(), &statuses());
        let s = sprint_figures(Ok(f), Ok(()));
        assert_eq!(
            (
                s.commitment_reliability,
                s.total_completion,
                s.scope_change,
                s.carry_over_rate,
                s.unplanned_work_ratio
            ),
            (Some(25.0), Some(50.0), Some(75.0), Some(25.0), Some(50.0))
        );
        assert_eq!(
            (s.velocity, s.reasons.get("velocity")),
            (None, Some(&"not_estimated")),
            "no points at all"
        );
    }

    #[test]
    fn sprint_reasons() {
        let s = sprint_figures(Err("not_started"), Ok(()));
        assert_eq!(s.reasons.get("commitment_reliability"), Some(&"not_started"));
        let empty = sprint_figures(Ok(SprintFacts::default()), Ok(()));
        assert_eq!(empty.reasons.get("commitment_reliability"), Some(&"no_commitment"));
        assert_eq!(empty.reasons.get("unplanned_work_ratio"), Some(&"nothing_completed"));
        let facts = || SprintFacts {
            committed: 2,
            completed_committed: 1,
            committed_points: Some(8.0),
            committed_points_n: 2,
            completed_committed_points: Some(3.0),
            completed_committed_points_n: 1,
            ..Default::default()
        };
        let s = sprint_figures(Ok(facts()), Ok(()));
        assert_eq!((s.velocity, s.points_commitment_reliability), (Some(3.0), Some(37.5)));
        let s = sprint_figures(Ok(facts()), Err("no_estimation_field"));
        assert_eq!((s.velocity, s.reasons.get("velocity")), (None, Some(&"no_estimation_field")));
        assert_eq!(s.reasons.get("points_commitment_reliability"), Some(&"no_estimation_field"));
    }

    #[test]
    fn predictability_skips_unmeasured_sprints_before_taking_six() {
        let mut r = vec![Some(50.0)];
        r.extend([None; 5]);
        r.push(Some(100.0));
        assert_eq!(predictability(&r), Ok(25.0));
    }

    /// A8: an estimated sprint that finished nothing has a velocity of 0.
    #[test]
    fn nothing_completed_is_zero_not_unestimated() {
        let f = SprintFacts {
            committed: 2,
            committed_points: Some(8.0),
            committed_points_n: 2,
            ..Default::default()
        };
        let s = sprint_figures(Ok(f), Ok(()));
        assert_eq!((s.velocity, s.points_commitment_reliability), (Some(0.0), Some(0.0)));
        assert_eq!(s.reasons.get("velocity"), None);

        let unestimated =
            SprintFacts { committed: 2, completed_committed: 1, ..Default::default() };
        let s = sprint_figures(Ok(unestimated), Ok(()));
        assert_eq!(s.reasons.get("velocity"), Some(&"not_estimated"), "done but no points");
        assert_eq!(s.reasons.get("points_commitment_reliability"), Some(&"not_estimated"));

        let zero = SprintFacts { committed_points: Some(0.0), committed: 1, ..Default::default() };
        let s = sprint_figures(Ok(zero), Ok(()));
        assert_eq!(s.reasons.get("points_commitment_reliability"), Some(&"no_commitment"));
    }

    #[test]
    fn a_status_the_board_does_not_list_is_named_not_counted_as_open() {
        use ChangeField::{Sprint, Status};
        let tasks = [task(
            -5,
            &["5"],
            "done",
            vec![change(Sprint, -1, "", "5"), change(Status, 10, "1", "9")],
        )];
        let f = replay(&sprints()[1], (0, 14 * DAY), &sprints(), &tasks, &statuses());
        assert_eq!((f.committed, f.completed_committed, f.unknown_status), (1, 0, 1));
        let s = sprint_figures(Ok(f), Ok(()));
        assert_eq!(s.commitment_reliability, None);
        assert_eq!(s.reasons.get("commitment_reliability"), Some(&"unknown_status"));
        assert_eq!(s.reasons.get("velocity"), Some(&"unknown_status"));
        assert_eq!(s.scope_change, Some(0.0), "membership does not need the status");
    }

    #[test]
    fn unread_history_before_the_end_holds_the_sprint() {
        let at = |gap: Option<i64>| {
            let mut t = task(-5, &["5"], "new", vec![]);
            t.history_gap_from = gap;
            t
        };
        assert!(!history_pending(&[at(None)], 14 * DAY), "read through its update");
        assert!(history_pending(&[at(Some(i64::MIN))], 14 * DAY), "never read");
        assert!(history_pending(&[at(Some(3 * DAY))], 14 * DAY), "unread from day 3");
        assert!(!history_pending(&[at(Some(20 * DAY))], 14 * DAY), "read past the end");
        let later = ReplayTask { created_at: Some(20 * DAY), ..at(Some(i64::MIN)) };
        assert!(!history_pending(&[later], 14 * DAY), "created after the sprint");
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;
    use crate::db::pg_store::PgStore;

    async fn exec(pg: &PgStore, sql: &str, t: uuid::Uuid) {
        sqlx_core::query::query(sql)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap_or_else(|e| panic!("{sql}: {e}"));
    }

    /// Sprint 100 (closed): A committed and done, B added and carried into 101.
    /// Sprint 101 (closed): C and D committed, C done; B added, not done.
    /// Sprint 102 is in the future. Points: A 3, C 5, B and D none.
    async fn seeded(pg: &PgStore) -> uuid::Uuid {
        let t = crate::tracker::history::test_tracker(pg).await;
        for sql in [
            "UPDATE sensei.trackers SET estimation_field = 'customfield_10016',
                    points_field = 'customfield_10016', terms_at = now() WHERE id = $1",
            "INSERT INTO sensei.tracker_statuses (tracker_id, status_id, name, category)
             VALUES ($1, '1', 'To Do', 'new'), ($1, '3', 'Done', 'done')",
            "INSERT INTO sensei.sprints (tracker_id, external_id, seq, name, state, start_at, end_at, complete_at)
             VALUES ($1, '100', 100, 'S100', 'closed', '2026-09-01Z', '2026-09-15Z', '2026-09-15Z'),
                    ($1, '101', 101, 'S101', 'closed', '2026-09-15Z', '2026-09-29Z', '2026-09-29Z'),
                    ($1, '102', 102, 'S102', 'future', NULL, NULL, NULL)",
            "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, issue_created_at, status_category, points)
             VALUES ($1, '1', 'A', '2026-08-01Z', 'done', 3), ($1, '2', 'B', '2026-08-01Z', 'new', NULL),
                    ($1, '3', 'C', '2026-08-01Z', 'done', 5), ($1, '4', 'D', '2026-08-01Z', 'new', NULL)",
            "UPDATE sensei.tracker_tasks SET issue_updated_at = '2026-10-01Z', history_through = '2026-10-01Z'
              WHERE tracker_id = $1",
            "INSERT INTO sensei.tracker_task_sprints (task_id, sprint_id)
             SELECT t.id, s.id FROM sensei.tracker_tasks t JOIN sensei.sprints s ON s.tracker_id = t.tracker_id
              WHERE t.tracker_id = $1 AND (t.key, s.external_id) IN
                    (('A', '100'), ('B', '100'), ('B', '101'), ('C', '101'), ('D', '101'))",
            "INSERT INTO sensei.tracker_task_changes (task_id, change_id, field, at, from_value, to_value)
             SELECT t.id, c.id, c.field::sensei.tracked_change_field, c.at::timestamptz, c.f, c.v
               FROM sensei.tracker_tasks t JOIN (VALUES
                    ('A', 'a1', 'status', '2026-09-10Z', '1', '3'),
                    ('B', 'b1', 'sprint', '2026-09-05Z', NULL, '100'),
                    ('B', 'b2', 'sprint', '2026-09-15T12:00Z', '100', '100,101'),
                    ('C', 'c1', 'status', '2026-09-20Z', '1', '3')) c(key, id, field, at, f, v)
                 ON c.key = t.key
              WHERE t.tracker_id = $1",
        ] {
            exec(pg, sql, t).await;
        }
        t
    }

    fn by_name(
        figures: &HashMap<uuid::Uuid, SprintSdlc>,
        inputs: &crate::db::pg_store::tracker_history::ReplayInputs,
        ext: &str,
    ) -> SprintSdlc {
        let id = inputs.sprints.iter().find(|s| s.external_id == ext).unwrap().id;
        figures[&id].clone()
    }

    #[tokio::test]
    async fn sprints_replay_from_the_stored_history() {
        let pg = PgStore::test_store().await;
        let t = seeded(&pg).await;
        let now = "2026-10-29T00:00:00Z".parse().unwrap();
        let inputs = pg.replay_inputs(t).await.unwrap();
        assert_eq!(
            inputs.sprints.iter().map(|s| s.seq).collect::<Vec<_>>(),
            [102, 101, 100],
            "newest first"
        );
        assert_eq!(inputs.estimation, Ok(()));
        let figures = sprint_sdlc(&pg, t, now).await.unwrap();
        assert_eq!(figures.len(), 3);

        let s100 = by_name(&figures, &inputs, "100");
        let f = s100.facts.as_ref().unwrap();
        assert_eq!(
            (f.committed, f.added, f.removed, f.completed_committed, f.completed_added),
            (1, 1, 0, 1, 0)
        );
        assert_eq!(f.carried_over, 1, "B moved on to 101 after the sprint closed");
        assert_eq!(s100.commitment_reliability, Some(100.0));
        assert_eq!(s100.scope_change, Some(100.0));
        assert_eq!(s100.velocity, Some(3.0));

        let s101 = by_name(&figures, &inputs, "101");
        let f = s101.facts.as_ref().unwrap();
        assert_eq!((f.committed, f.added, f.completed_committed), (2, 1, 1));
        assert_eq!(s101.commitment_reliability, Some(50.0));
        assert_eq!(s101.velocity, Some(5.0));
        assert_eq!(s101.points_commitment_reliability, Some(100.0), "D has no points");

        let s102 = by_name(&figures, &inputs, "102");
        assert_eq!(s102.commitment_reliability, None);
        assert_eq!(s102.reasons.get("commitment_reliability"), Some(&"sprint_not_started"));

        let p = project_sdlc(&pg, t, now).await.unwrap();
        assert_eq!(p.predictability, Some(25.0), "stddev of 50 and 100");
        assert_eq!(p.reasons.get("tokens_per_ticket"), Some(&"no_usage"));
    }

    #[tokio::test]
    async fn points_wait_for_the_board_terms() {
        let pg = PgStore::test_store().await;
        let t = seeded(&pg).await;
        exec(&pg, "UPDATE sensei.trackers SET terms_at = NULL WHERE id = $1", t).await;
        let now = "2026-10-29T00:00:00Z".parse().unwrap();
        let inputs = pg.replay_inputs(t).await.unwrap();
        let s100 = by_name(&sprint_sdlc(&pg, t, now).await.unwrap(), &inputs, "100");
        assert_eq!(s100.velocity, None);
        assert_eq!(s100.reasons.get("velocity"), Some(&"not_synced"));
        assert_eq!(s100.commitment_reliability, Some(100.0), "counts need no points");
    }

    #[tokio::test]
    async fn bad_rows_are_errors_not_figures() {
        let pg = PgStore::test_store().await;
        let now = chrono::Utc::now();
        let ghost = uuid::Uuid::new_v4();
        assert!(pg.replay_inputs(ghost).await.unwrap_err().contains("not found"));
        let t = seeded(&pg).await;
        exec(
            &pg,
            "UPDATE sensei.sprints SET state = 'paused' WHERE tracker_id = $1 AND seq = 102",
            t,
        )
        .await;
        assert!(sprint_sdlc(&pg, t, now).await.unwrap_err().contains("paused"));
        assert!(project_sdlc(&pg, t, now).await.unwrap_err().contains("paused"));
    }

    #[tokio::test]
    async fn a_ticket_with_unread_history_holds_the_sprints_it_could_touch() {
        let pg = PgStore::test_store().await;
        let t = seeded(&pg).await;
        let now = "2026-10-29T00:00:00Z".parse().unwrap();
        // D was updated on 09-20 and read only through 09-16: unread from then.
        exec(
            &pg,
            "UPDATE sensei.tracker_tasks SET issue_updated_at = '2026-09-20Z',
                    history_through = '2026-09-16Z' WHERE tracker_id = $1 AND key = 'D'",
            t,
        )
        .await;
        let inputs = pg.replay_inputs(t).await.unwrap();
        let figures = sprint_sdlc(&pg, t, now).await.unwrap();
        let s100 = by_name(&figures, &inputs, "100");
        let s101 = by_name(&figures, &inputs, "101");
        assert_eq!(s100.commitment_reliability, Some(100.0), "read past sprint 100's end");
        assert_eq!(s101.commitment_reliability, None);
        assert_eq!(s101.reasons.get("commitment_reliability"), Some(&"history_pending"));

        exec(
            &pg,
            "UPDATE sensei.tracker_tasks SET history_through = NULL WHERE tracker_id = $1",
            t,
        )
        .await;
        let figures = sprint_sdlc(&pg, t, now).await.unwrap();
        assert_eq!(
            by_name(&figures, &inputs, "100").reasons.get("velocity"),
            Some(&"history_pending"),
            "never read"
        );
    }

    #[tokio::test]
    async fn same_instant_changes_replay_in_change_id_order() {
        let pg = PgStore::test_store().await;
        let t = seeded(&pg).await;
        exec(
            &pg,
            "INSERT INTO sensei.tracker_task_changes (task_id, change_id, field, at, from_value, to_value)
             SELECT t.id, c.cid, 'sprint', '2026-09-20Z', c.f, c.v FROM sensei.tracker_tasks t,
                    (VALUES ('10', '101', '101,102'), ('9', NULL, '101')) c(cid, f, v)
              WHERE t.tracker_id = $1 AND t.key = 'D'",
            t,
        )
        .await;
        let inputs = pg.replay_inputs(t).await.unwrap();
        let d = inputs
            .tasks
            .iter()
            .find(|x| x.changes.iter().any(|c| c.to.as_deref() == Some("101,102")))
            .unwrap();
        assert_eq!(
            d.changes.iter().map(|c| c.from.as_deref()).collect::<Vec<_>>(),
            [None, Some("101")],
            "change 9 before change 10, as Jira numbered them"
        );
    }
}
