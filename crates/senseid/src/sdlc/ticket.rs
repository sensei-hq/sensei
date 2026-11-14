//! A ticket's SDLC figures (spec S6 "Ticket"), computed at read from its linked
//! sessions' derived signals, their R7 usage and the workspace settings.
//!
//! Pure: [`compute`] is a function of its input. Times are Unix milliseconds.
//! Every figure that can't be computed is `None` with a reason code from the
//! `sdlc_metrics` domain of `sensei.reason_codes`; a `0` means the inputs exist
//! and the count is zero.

use std::collections::BTreeMap;

use crate::sdlc::settings::MetricSettings;
use crate::sdlc::signals::{HoldReason, Signal};
use crate::tracker::usage::Usage;

/// One linked session's signals, and whether they reflect its current events
/// and catalogue (spec S6 "Derivation must be current").
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionSignals {
    pub current: bool,
    /// Derivation failed on these very events and catalogue: it will not be
    /// retried until they change, so the figures are not merely late (A9).
    pub failed: bool,
    pub signals: Vec<Signal>,
}

/// Why a set of linked sessions has no signal figures yet: one still waits for
/// derivation (`not_derived`), or one failed on its current inputs
/// (`derivation_failed`, A9). Waiting outranks failing: it may yet resolve.
pub fn underived<'a>(
    sessions: impl IntoIterator<Item = &'a SessionSignals>,
) -> Option<&'static str> {
    let (mut waiting, mut failed) = (false, false);
    for s in sessions {
        waiting |= !s.current && !s.failed;
        failed |= s.failed;
    }
    if waiting {
        Some("not_derived")
    } else if failed {
        Some("derivation_failed")
    } else {
        None
    }
}

/// The ticket's story points as the tracker states them (D6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Estimate {
    /// The ticket's history has not been read yet.
    NotSynced,
    /// The board estimates by count or not at all.
    NoField,
    NotEstimated,
    Points(f64),
    /// The tracker's adapter reads no history, so it never learns points (§G8).
    Unsupported,
}

/// What a ticket's figures are computed from.
#[derive(Debug, Clone)]
pub struct TicketInput<'a> {
    pub sessions: &'a [SessionSignals],
    pub usage: &'a Usage,
    pub settings: &'a MetricSettings,
    /// The issue's creation time; `None` until its history is read.
    pub issue_created_at: Option<i64>,
    pub estimate: Estimate,
}

/// The ticket's SDLC figures (spec S6). `reasons` explains every `None`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct TicketSdlc {
    pub worked_minutes: Option<f64>,
    pub prompts: Option<i64>,
    pub ai_token_cost: Option<f64>,
    pub cost_per_prompt_cycle: Option<f64>,
    pub escaped_task_defects: Option<i64>,
    pub compile_defects: Option<i64>,
    pub test_defects: Option<i64>,
    pub review_fixes: Option<i64>,
    pub review_defects: Option<i64>,
    pub total_defects: Option<i64>,
    pub testing_efficiency: Option<f64>,
    pub review_efficiency: Option<f64>,
    pub build_runs: Option<i64>,
    pub runs_unknown: Option<i64>,
    pub unclassified_failures: Option<i64>,
    pub build_failure_rate: Option<f64>,
    pub mean_build_recovery_minutes: Option<f64>,
    pub build_unrecovered: Option<i64>,
    #[serde(serialize_with = "rfc3339")]
    pub work_started_at: Option<i64>,
    #[serde(serialize_with = "rfc3339")]
    pub submitted_at: Option<i64>,
    pub submitted_via: Option<&'static str>,
    pub execution_hours: Option<f64>,
    pub lead_hours: Option<f64>,
    pub on_hold_minutes: Option<f64>,
    pub on_hold_count: Option<i64>,
    pub on_hold_by_reason: Option<BTreeMap<&'static str, f64>>,
    pub on_hold_ratio: Option<f64>,
    pub rework_minutes: Option<f64>,
    pub rework_cycles: Option<i64>,
    pub post_review_rework_ratio: Option<f64>,
    pub estimated_points: Option<f64>,
    pub expected_minutes: Option<f64>,
    pub time_variance_minutes: Option<f64>,
    pub effort_cost: Option<f64>,
    pub rework_cost: Option<f64>,
    pub reasons: BTreeMap<&'static str, &'static str>,
    /// What the dōjō merge sums (tracker-delivery-metrics D1); never served.
    #[serde(skip)]
    pub numerators: Numerators,
}

/// A ticket's raw counts and milliseconds behind its ratios, so figures merged
/// across people are recomputed from sums, never averaged (tracker-delivery-metrics
/// D1). Each is `None` exactly where its figure is unknown.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Numerators {
    pub worked_ms: Option<i64>,
    pub on_hold_ms: Option<i64>,
    pub execution_ms: Option<i64>,
    pub rework_ms: Option<i64>,
    pub build_ok: Option<i64>,
    pub build_fail: Option<i64>,
    pub recovery_ms_sum: Option<i64>,
    pub recovery_n: Option<i64>,
}

/// An instant in Unix milliseconds, as RFC 3339 like every other API timestamp.
fn rfc3339<S: serde::Serializer>(ms: &Option<i64>, ser: S) -> Result<S::Ok, S::Error> {
    match ms.and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis) {
        Some(t) => ser.serialize_str(&t.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
        None => ser.serialize_none(),
    }
}

/// Figures read from the signals (spec S6): all of them wait for derivation.
const SIGNAL_FIGURES: &[&str] = &[
    "worked_minutes",
    "prompts",
    "cost_per_prompt_cycle",
    "escaped_task_defects",
    "compile_defects",
    "test_defects",
    "review_fixes",
    "review_defects",
    "total_defects",
    "testing_efficiency",
    "review_efficiency",
    "build_runs",
    "runs_unknown",
    "unclassified_failures",
    "build_failure_rate",
    "mean_build_recovery_minutes",
    "build_unrecovered",
    "work_started_at",
    "submitted_at",
    "submitted_via",
    "execution_hours",
    "lead_hours",
    "on_hold_minutes",
    "on_hold_count",
    "on_hold_by_reason",
    "on_hold_ratio",
    "rework_minutes",
    "rework_cycles",
    "post_review_rework_ratio",
    "time_variance_minutes",
    "effort_cost",
    "rework_cost",
];

const MS_PER_MIN: f64 = 60_000.0;
const MS_PER_HOUR: f64 = 3_600_000.0;

/// The ticket's figures (spec S6 "Ticket"). Values are unrounded; write-back
/// rounds once (S9).
pub fn compute(input: &TicketInput) -> TicketSdlc {
    let mut t = TicketSdlc::default();
    let u = input.usage;
    match u.api_cost {
        Some(c) => t.ai_token_cost = Some(c),
        None => {
            t.reasons.insert("ai_token_cost", "not_priced");
        }
    }
    match input.estimate {
        Estimate::Points(p) => t.estimated_points = Some(p),
        Estimate::NotSynced => {
            t.reasons.insert("estimated_points", "not_synced");
        }
        Estimate::NoField => {
            t.reasons.insert("estimated_points", "no_estimation_field");
        }
        Estimate::NotEstimated => {
            t.reasons.insert("estimated_points", "not_estimated");
        }
        Estimate::Unsupported => {
            t.reasons.insert("estimated_points", "unsupported_by_tracker");
        }
    }
    match (t.estimated_points, input.settings.minutes_per_point) {
        (Some(p), Some(m)) => t.expected_minutes = Some(p * m),
        (None, _) => {
            let why = t.reasons["estimated_points"];
            t.reasons.insert("expected_minutes", why);
        }
        (Some(_), None) => {
            t.reasons.insert("expected_minutes", "no_calibration");
        }
    }

    let blocked =
        if input.sessions.is_empty() { Some("no_sessions") } else { underived(input.sessions) };
    if let Some(why) = blocked {
        for key in SIGNAL_FIGURES {
            t.reasons.insert(key, why);
        }
        return t;
    }
    from_signals(input, &mut t);
    t
}

/// The signal-based figures, once every session is current.
fn from_signals(input: &TicketInput, t: &mut TicketSdlc) {
    use crate::sdlc::signals::SignalKind as K;
    let threshold = i64::from(input.settings.idle_threshold_minutes) * 60_000;

    // Signals merged across sessions by time; ties by session order, then sequence.
    let mut timeline: Vec<(i64, usize, usize, &Signal)> = input
        .sessions
        .iter()
        .enumerate()
        .flat_map(|(si, s)| {
            s.signals
                .iter()
                .enumerate()
                .filter(|(_, x)| x.kind != K::Slice)
                .map(move |(k, x)| (x.at, si, k, x))
        })
        .collect();
    timeline.sort_by_key(|&(at, si, k, _)| (at, si, k));
    let order: Vec<&Signal> = timeline.into_iter().map(|(.., x)| x).collect();
    let count = |kind: K| order.iter().filter(|s| s.kind == kind).count() as i64;

    // Slices merged by the threshold per session, then their union across sessions.
    let union = union_slices(input.sessions, threshold);
    if union.is_empty() {
        t.reasons.insert("worked_minutes", "no_activity");
    } else {
        let worked_ms: i64 = union.iter().map(|u| u.end - u.start).sum();
        t.numerators.worked_ms = Some(worked_ms);
        t.worked_minutes = Some(worked_ms as f64 / MS_PER_MIN);
    }

    let prompts = count(K::Prompt);
    t.prompts = Some(prompts);
    let u = input.usage;
    t.cost_per_prompt_cycle = match u.api_cost {
        None => why(t, "cost_per_prompt_cycle", "not_priced"),
        Some(_) if u.api_cost_n < u.sessions || u.api_cost_partial > 0 => {
            why(t, "cost_per_prompt_cycle", "partial_cost")
        }
        Some(_) if prompts == 0 => why(t, "cost_per_prompt_cycle", "no_prompts"),
        Some(c) => Some(c / prompts as f64),
    };

    // Escapes (D7, D11): a tick, then a failure before the next tick or PR, later
    // recovered in that failure's own category (A5). One escape per tick.
    let is_fail = |k: K| matches!(k, K::BuildFail | K::TestFail);
    let is_ok = |k: K| matches!(k, K::BuildOk | K::TestOk);
    let mut triggers = std::collections::HashSet::new();
    let mut escaped = 0;
    for (i, s) in order.iter().enumerate().filter(|(_, s)| s.kind == K::TaskTick) {
        let Some(f) = order[i + 1..]
            .iter()
            .take_while(|x| !matches!(x.kind, K::TaskTick | K::PrCreated))
            .position(|x| is_fail(x.kind))
            .map(|p| i + 1 + p)
        else {
            continue;
        };
        let _ = s;
        let cat = order[f].category;
        let Some(o) = order[f + 1..]
            .iter()
            .position(|x| is_ok(x.kind) && x.category == cat)
            .map(|p| f + 1 + p)
        else {
            continue;
        };
        escaped += 1;
        triggers.extend((f..o).filter(|&j| is_fail(order[j].kind) && order[j].category == cat));
    }
    let untriggered = |kind: K| {
        order.iter().enumerate().filter(|(j, s)| s.kind == kind && !triggers.contains(j)).count()
            as i64
    };
    let compile = untriggered(K::BuildFail);
    let test = untriggered(K::TestFail);
    let review_fixes = count(K::ReviewFix);
    let review_defects = review_fixes + escaped;
    let total = compile + test + review_defects;
    t.escaped_task_defects = Some(escaped);
    t.compile_defects = Some(compile);
    t.test_defects = Some(test);
    t.review_fixes = Some(review_fixes);
    t.review_defects = Some(review_defects);
    t.total_defects = Some(total);
    if total > 0 {
        t.testing_efficiency = Some((compile + test) as f64 / total as f64 * 100.0);
        t.review_efficiency = Some(review_defects as f64 / total as f64 * 100.0);
    } else {
        why::<f64>(t, "testing_efficiency", "no_defects");
        why::<f64>(t, "review_efficiency", "no_defects");
    }

    let build_fail = count(K::BuildFail);
    let build_ok = count(K::BuildOk);
    t.numerators.build_ok = Some(build_ok);
    t.numerators.build_fail = Some(build_fail);
    let build_runs = build_ok + build_fail;
    t.build_runs = Some(build_runs);
    let unmatched = |categorised: bool| {
        order
            .iter()
            .filter(|s| s.kind == K::UnmatchedCall && s.category.is_some() == categorised)
            .count() as i64
    };
    t.runs_unknown = Some(count(K::RunUnknown) + unmatched(true));
    t.unclassified_failures = Some(count(K::UnclassifiedFail) + unmatched(false));
    t.build_failure_rate = if build_runs > 0 {
        Some(build_fail as f64 / build_runs as f64 * 100.0)
    } else {
        why(t, "build_failure_rate", "no_runs")
    };
    let mut gaps = Vec::new();
    let mut unrecovered = 0;
    for (f, s) in order.iter().enumerate().filter(|(_, s)| s.kind == K::BuildFail) {
        match order[f + 1..].iter().find(|x| x.kind == K::BuildOk && x.category == s.category) {
            Some(ok) => gaps.push(ok.at - s.at),
            None => unrecovered += 1,
        }
    }
    t.build_unrecovered = Some(unrecovered);
    let recovery_ms_sum: i64 = gaps.iter().sum();
    t.numerators.recovery_ms_sum = Some(recovery_ms_sum);
    t.numerators.recovery_n = Some(gaps.len() as i64);
    t.mean_build_recovery_minutes = if gaps.is_empty() {
        why(t, "mean_build_recovery_minutes", "no_recovery")
    } else {
        Some(recovery_ms_sum as f64 / gaps.len() as f64 / MS_PER_MIN)
    };

    // Timing: work starts at the first code edit; submission is the first PR (else
    // push) at or after it.
    let started = order.iter().filter(|s| s.kind == K::CodeEdit).map(|s| s.at).min();
    let first_at = |kind: K, from: i64| {
        order.iter().filter(|s| s.kind == kind && s.at >= from).map(|s| s.at).min()
    };
    let submitted = started.and_then(|w| {
        first_at(K::PrCreated, w)
            .map(|at| (at, "pr"))
            .or_else(|| first_at(K::Pushed, w).map(|at| (at, "push")))
    });
    match started {
        Some(w) => t.work_started_at = Some(w),
        None => {
            why::<i64>(t, "work_started_at", "not_started");
        }
    }
    let not_submitted = if started.is_none() { "not_started" } else { "not_submitted" };
    match submitted {
        Some((at, via)) => {
            t.submitted_at = Some(at);
            t.submitted_via = Some(via);
        }
        None => {
            why::<i64>(t, "submitted_at", not_submitted);
            why::<&str>(t, "submitted_via", not_submitted);
        }
    }
    let window = started.zip(submitted.map(|(at, _)| at));
    t.numerators.execution_ms = window.map(|(w, s)| s - w);
    t.execution_hours = match window {
        Some((w, s)) => Some((s - w) as f64 / MS_PER_HOUR),
        None => why(t, "execution_hours", not_submitted),
    };
    t.lead_hours = match (input.issue_created_at, submitted) {
        (None, _) => why(t, "lead_hours", "not_synced"),
        (_, None) => why(t, "lead_hours", not_submitted),
        (Some(c), Some((s, _))) if s < c => why(t, "lead_hours", "created_after_submit"),
        (Some(c), Some((s, _))) => Some((s - c) as f64 / MS_PER_HOUR),
    };

    // Holds: gaps longer than the threshold between the union's intervals, inside
    // the window. A slice with no reason holds as `unknown` (A5).
    match window {
        None => {
            for key in ["on_hold_minutes", "on_hold_count", "on_hold_by_reason", "on_hold_ratio"] {
                t.reasons.insert(key, not_submitted);
            }
        }
        Some((w, s)) => {
            let clipped: Vec<&Interval> =
                union.iter().filter(|u| u.end >= w && u.start <= s).collect();
            let mut held_ms = 0;
            let mut holds = 0;
            let mut by_reason: BTreeMap<&'static str, f64> = BTreeMap::new();
            for pair in clipped.windows(2) {
                let (before, after) = (pair[0], pair[1]);
                let gap = after.start.min(s) - before.end.max(w);
                if gap > threshold {
                    let m = gap as f64 / MS_PER_MIN;
                    held_ms += gap;
                    holds += 1;
                    *by_reason
                        .entry(before.reason.unwrap_or(HoldReason::Unknown).as_str())
                        .or_default() += m;
                }
            }
            let minutes = held_ms as f64 / MS_PER_MIN;
            t.numerators.on_hold_ms = Some(held_ms);
            t.on_hold_minutes = Some(minutes);
            t.on_hold_count = Some(holds);
            t.on_hold_by_reason = Some(by_reason);
            t.on_hold_ratio = if s > w {
                Some(minutes / ((s - w) as f64 / MS_PER_MIN) * 100.0)
            } else {
                why(t, "on_hold_ratio", "no_window")
            };
        }
    }

    // Rework after submission, over the union (A5), and the push cycles it took.
    match submitted {
        None => {
            for key in
                ["rework_minutes", "rework_cycles", "post_review_rework_ratio", "rework_cost"]
            {
                t.reasons.insert(key, not_submitted);
            }
        }
        Some((s, _)) => {
            let edits_after: Vec<i64> =
                order.iter().filter(|x| x.kind == K::CodeEdit && x.at >= s).map(|x| x.at).collect();
            let rework_ms: i64 = union
                .iter()
                .filter_map(|u| {
                    edits_after
                        .iter()
                        .find(|&&at| at >= u.start && at <= u.end)
                        .map(|&at| u.end - at)
                })
                .sum();
            t.numerators.rework_ms = Some(rework_ms);
            let rework = rework_ms as f64 / MS_PER_MIN;
            t.rework_minutes = Some(rework);
            let mut cycles = 0;
            let mut edited = false;
            for x in order.iter().filter(|x| x.at >= s) {
                match x.kind {
                    K::CodeEdit => edited = true,
                    K::Pushed if x.at > s => {
                        if edited {
                            cycles += 1;
                        }
                        edited = false;
                    }
                    _ => {}
                }
            }
            t.rework_cycles = Some(cycles);
            t.post_review_rework_ratio = match t.worked_minutes {
                Some(wm) if wm > 0.0 => Some(rework / wm * 100.0),
                _ => why(t, "post_review_rework_ratio", "no_activity"),
            };
            t.rework_cost = match input.settings.hourly_rate {
                Some(rate) => Some(rate * rework / 60.0),
                None => why(t, "rework_cost", "no_rate"),
            };
        }
    }

    t.time_variance_minutes = match (t.worked_minutes, t.expected_minutes) {
        (Some(wm), Some(e)) => Some(wm - e),
        (None, _) => why(t, "time_variance_minutes", "no_activity"),
        (_, None) => {
            let why_e = t.reasons["expected_minutes"];
            why(t, "time_variance_minutes", why_e)
        }
    };
    t.effort_cost = match (input.settings.hourly_rate, t.worked_minutes) {
        (None, _) => why(t, "effort_cost", "no_rate"),
        (_, None) => why(t, "effort_cost", "no_activity"),
        (Some(rate), Some(wm)) => Some(rate * wm / 60.0),
    };
}

/// Record why a figure is null, and return the null.
fn why<T>(t: &mut TicketSdlc, key: &'static str, reason: &'static str) -> Option<T> {
    t.reasons.insert(key, reason);
    None
}

/// A stretch of work in milliseconds, and the reason it ended.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Interval {
    start: i64,
    end: i64,
    reason: Option<HoldReason>,
    session: usize,
}

/// Each session's base slices merged where they are no more than `threshold`
/// apart (a merged slice keeps its last base reason that exists, A4), then the
/// union across sessions. An interval ending where another does keeps the reason
/// of the later session that has one.
fn union_slices(sessions: &[SessionSignals], threshold: i64) -> Vec<Interval> {
    use crate::sdlc::signals::SignalKind as K;
    let mut merged: Vec<Interval> = Vec::new();
    for (si, s) in sessions.iter().enumerate() {
        let mut base: Vec<&Signal> = s.signals.iter().filter(|x| x.kind == K::Slice).collect();
        base.sort_by_key(|x| x.at);
        let mut own: Vec<Interval> = Vec::new();
        for b in base {
            let end = b.until.unwrap_or(b.at);
            match own.last_mut() {
                Some(last) if b.at - last.end <= threshold => {
                    last.end = last.end.max(end);
                    if b.reason.is_some() {
                        last.reason = b.reason;
                    }
                }
                _ => own.push(Interval { start: b.at, end, reason: b.reason, session: si }),
            }
        }
        merged.extend(own);
    }
    merged.sort_by_key(|m| (m.start, m.session));
    let mut union: Vec<Interval> = Vec::new();
    for m in merged {
        match union.last_mut() {
            Some(u) if m.start <= u.end => {
                if m.end > u.end || (m.end == u.end && m.reason.is_some()) {
                    u.reason = m.reason.or(u.reason);
                    u.session = m.session;
                }
                u.end = u.end.max(m.end);
            }
            _ => union.push(m),
        }
    }
    union
}

/// Every ticket's figures among `task_ids` on a tracker (spec S6), from R7's
/// ticket session sets and `usage` (already computed by the caller).
pub async fn ticket_figures(
    pg: &crate::db::pg_store::PgStore,
    tracker_id: uuid::Uuid,
    workspace_id: uuid::Uuid,
    task_ids: &[uuid::Uuid],
    usage: &std::collections::HashMap<uuid::Uuid, Usage>,
) -> Result<std::collections::HashMap<uuid::Uuid, TicketSdlc>, String> {
    let pairs = pg.ticket_sessions(tracker_id, Some(task_ids)).await?;
    ticket_figures_of(pg, tracker_id, workspace_id, task_ids, &pairs, usage).await
}

/// Every ticket's figures among `task_ids` over exactly these (ticket, session)
/// pairs, and `usage` computed over the same pairs.
pub async fn ticket_figures_of(
    pg: &crate::db::pg_store::PgStore,
    tracker_id: uuid::Uuid,
    workspace_id: uuid::Uuid,
    task_ids: &[uuid::Uuid],
    pairs: &[crate::db::pg_store::tracker_usage::TicketSession],
    usage: &std::collections::HashMap<uuid::Uuid, Usage>,
) -> Result<std::collections::HashMap<uuid::Uuid, TicketSdlc>, String> {
    let catalogue = pg.command_catalogue().await?;
    let settings = pg.metric_settings(workspace_id).await?;
    let mut ids: Vec<uuid::Uuid> = pairs.iter().map(|p| p.session_id).collect();
    ids.sort();
    ids.dedup();
    let signals = pg.session_signals_for(&ids, &catalogue.hash).await?;
    let reads_history = crate::sdlc::reads_history(pg, tracker_id).await?;
    let history = pg.ticket_history_facts(task_ids).await?;
    let none = Usage::default();
    Ok(task_ids
        .iter()
        .map(|task| {
            let linked: std::collections::HashSet<uuid::Uuid> =
                pairs.iter().filter(|p| p.task_id == *task).map(|p| p.session_id).collect();
            let sessions: Vec<SessionSignals> = signals
                .iter()
                .filter(|(id, _)| linked.contains(id))
                .map(|(_, s)| s.clone())
                .collect();
            let facts = history.get(task);
            let estimate = match facts {
                _ if !reads_history => Estimate::Unsupported,
                None => Estimate::NotSynced,
                Some(f) if !f.learned => Estimate::NotSynced,
                Some(f) if !f.has_field => Estimate::NoField,
                Some(f) => f.points.map_or(Estimate::NotEstimated, Estimate::Points),
            };
            let figures = compute(&TicketInput {
                sessions: &sessions,
                usage: usage.get(task).unwrap_or(&none),
                settings: &settings,
                issue_created_at: facts.and_then(|f| f.created_at).map(|t| t.timestamp_millis()),
                estimate,
            });
            (*task, figures)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sdlc::classify::Category;
    use crate::sdlc::signals::SignalKind::{self, *};

    const MIN: i64 = 60_000;

    fn sig(kind: SignalKind, minute: i64) -> Signal {
        Signal {
            kind,
            at: minute * MIN,
            until: None,
            category: None,
            reason: None,
            reference: None,
        }
    }
    fn run(kind: SignalKind, minute: i64, c: Category) -> Signal {
        Signal { category: Some(c), ..sig(kind, minute) }
    }
    fn slice(from: i64, to: i64, reason: Option<HoldReason>) -> Signal {
        Signal { until: Some(to * MIN), reason, ..sig(Slice, from) }
    }
    fn current(signals: Vec<Signal>) -> SessionSignals {
        SessionSignals { current: true, failed: false, signals }
    }
    fn settings(t: i32) -> MetricSettings {
        MetricSettings { idle_threshold_minutes: t, ..Default::default() }
    }
    fn input<'a>(
        sessions: &'a [SessionSignals],
        usage: &'a Usage,
        s: &'a MetricSettings,
    ) -> TicketInput<'a> {
        TicketInput {
            sessions,
            usage,
            settings: s,
            issue_created_at: None,
            estimate: Estimate::NotSynced,
        }
    }

    /// Spec gate 2, fixture A, as F2 derives it.
    fn fixture_a() -> SessionSignals {
        current(vec![
            sig(CodeEdit, 0),
            run(BuildFail, 1, Category::Build),
            slice(0, 2, Some(HoldReason::BuildFailure)),
            sig(CodeEdit, 30),
            run(BuildOk, 31, Category::Build),
            sig(PrCreated, 32),
            slice(30, 32, Some(HoldReason::Unknown)),
            sig(CodeEdit, 50),
            slice(50, 50, Some(HoldReason::Unknown)),
            sig(Pushed, 52),
            slice(52, 53, Some(HoldReason::SessionEnded)),
        ])
    }

    #[test]
    fn fixture_a_at_fifteen_minutes() {
        let sessions = [fixture_a()];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(
            (t.compile_defects, t.build_runs, t.build_failure_rate),
            (Some(1), Some(2), Some(50.0))
        );
        assert_eq!(t.mean_build_recovery_minutes, Some(30.0));
        assert_eq!(
            (t.work_started_at, t.submitted_at, t.submitted_via),
            (Some(0), Some(32 * MIN), Some("pr"))
        );
        assert_eq!(t.execution_hours, Some(32.0 / 60.0));
        assert_eq!((t.on_hold_count, t.on_hold_minutes), (Some(1), Some(28.0)));
        assert_eq!(t.on_hold_by_reason, Some(BTreeMap::from([("build_failure", 28.0)])));
        assert_eq!((t.rework_minutes, t.rework_cycles), (Some(3.0), Some(1)));
        assert_eq!(t.worked_minutes, Some(7.0));
        assert_eq!(t.on_hold_ratio, Some(28.0 / 32.0 * 100.0));
    }

    /// tracker-delivery-metrics D1: the ✚ numerators and the milliseconds the merge
    /// sums, so the dōjō recomputes the ratios rather than averaging them.
    #[test]
    fn fixture_a_exposes_the_numerators_the_merge_sums() {
        let sessions = [fixture_a()];
        let (u, s) = (Usage::default(), settings(15));
        let n = compute(&input(&sessions, &u, &s)).numerators;
        assert_eq!(
            n,
            Numerators {
                worked_ms: Some(7 * MIN),
                on_hold_ms: Some(28 * MIN),
                execution_ms: Some(32 * MIN),
                rework_ms: Some(3 * MIN),
                build_ok: Some(1),
                build_fail: Some(1),
                recovery_ms_sum: Some(30 * MIN),
                recovery_n: Some(1),
            }
        );
    }

    #[test]
    fn numerators_are_unknown_where_their_figures_are() {
        // No sessions: nothing was measured, so nothing is 0.
        let (u, s) = (Usage::default(), settings(15));
        assert_eq!(compute(&input(&[], &u, &s)).numerators, Numerators::default());
        // Worked but never submitted: no window, no rework, but runs are counted.
        let sessions = [current(vec![
            sig(CodeEdit, 0),
            run(BuildFail, 1, Category::Build),
            slice(0, 2, None),
        ])];
        let n = compute(&input(&sessions, &u, &s)).numerators;
        assert_eq!((n.worked_ms, n.build_ok, n.build_fail), (Some(2 * MIN), Some(0), Some(1)));
        assert_eq!((n.recovery_ms_sum, n.recovery_n), (Some(0), Some(0)));
        assert_eq!((n.execution_ms, n.on_hold_ms, n.rework_ms), (None, None, None));
    }

    #[test]
    fn the_numerators_never_reach_the_api() {
        let sessions = [fixture_a()];
        let (u, s) = (Usage::default(), settings(15));
        let json = serde_json::to_value(compute(&input(&sessions, &u, &s))).unwrap();
        assert!(json.get("numerators").is_none() && json.get("worked_ms").is_none());
    }

    #[test]
    fn the_threshold_is_applied_at_read() {
        let sessions = [fixture_a()];
        let u = Usage::default();
        let at_one = compute(&input(&sessions, &u, &settings(1)));
        assert_eq!(at_one.on_hold_count, Some(1));
        let at_thirty = compute(&input(&sessions, &u, &settings(30)));
        assert_eq!((at_thirty.on_hold_count, at_thirty.on_hold_minutes), (Some(0), Some(0.0)));
        assert_eq!(
            at_thirty.worked_minutes,
            Some(53.0),
            "every gap (28, 18, 2) is within 30: one slice [0,53]"
        );
    }

    #[test]
    fn a_merged_slice_takes_its_last_reason_that_exists() {
        // A4: [build fail], then a tail-only base slice; merged at 15, the hold after
        // them is the build failure's.
        let sessions = [current(vec![
            sig(CodeEdit, 0),
            run(BuildFail, 0, Category::Build),
            slice(0, 0, Some(HoldReason::BuildFailure)),
            slice(3, 3, None),
            sig(CodeEdit, 40),
            sig(PrCreated, 41),
            slice(40, 41, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(t.on_hold_by_reason, Some(BTreeMap::from([("build_failure", 37.0)])));
    }

    /// Spec gate 3, fixture B: a tick in one session escapes in another (D11).
    #[test]
    fn an_escape_across_sessions() {
        let tick = Signal { reference: Some("tasks.md: a".into()), ..sig(TaskTick, 0) };
        let sessions = [
            current(vec![tick, slice(0, 0, Some(HoldReason::Unknown))]),
            current(vec![
                run(TestFail, 60, Category::Test),
                run(TestOk, 70, Category::Test),
                slice(60, 70, Some(HoldReason::Unknown)),
            ]),
        ];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(
            (t.escaped_task_defects, t.test_defects, t.review_defects, t.total_defects),
            (Some(1), Some(0), Some(1), Some(1))
        );
    }

    /// Spec gate 4, fixture E.
    #[test]
    fn trigger_fails_are_the_escape_category_only() {
        let sessions = [current(vec![
            sig(TaskTick, 0),
            run(TestFail, 1, Category::Test),
            run(BuildFail, 2, Category::Build),
            run(BuildOk, 3, Category::Build),
            run(TestOk, 4, Category::Test),
            slice(0, 4, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(
            (t.escaped_task_defects, t.compile_defects, t.total_defects),
            (Some(1), Some(1), Some(2))
        );
        assert_eq!(t.testing_efficiency, Some(50.0));
        assert_eq!(t.review_efficiency, Some(50.0));
    }

    #[test]
    fn back_to_back_sessions_under_the_threshold_are_no_hold() {
        let sessions = [
            current(vec![sig(CodeEdit, 0), slice(0, 10, Some(HoldReason::SessionEnded))]),
            current(vec![sig(PrCreated, 20), slice(12, 20, Some(HoldReason::Unknown))]),
        ];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(
            (t.on_hold_count, t.on_hold_minutes, t.worked_minutes),
            (Some(0), Some(0.0), Some(18.0))
        );
    }

    #[test]
    fn a_session_not_derived_nulls_every_signal_figure() {
        let sessions = [fixture_a(), SessionSignals { current: false, ..Default::default() }];
        let u = Usage { api_cost: Some(1.5), api_cost_n: 2, sessions: 2, ..Default::default() };
        let s = settings(15);
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!((t.prompts, t.compile_defects, t.worked_minutes), (None, None, None));
        assert_eq!(t.reasons.get("compile_defects"), Some(&"not_derived"));
        assert_eq!(t.ai_token_cost, Some(1.5), "usage figures don't wait for derivation");
    }

    #[test]
    fn lead_time_needs_the_issues_creation_and_is_never_negative() {
        let sessions = [fixture_a()];
        let (u, s) = (Usage::default(), settings(15));
        let mut i = input(&sessions, &u, &s);
        assert_eq!(
            (compute(&i).lead_hours, compute(&i).reasons.get("lead_hours")),
            (None, Some(&"not_synced"))
        );
        i.issue_created_at = Some(-60 * MIN);
        assert!((compute(&i).lead_hours.unwrap() - (32.0 / 60.0 + 1.0)).abs() < 1e-9);
        i.issue_created_at = Some(40 * MIN);
        let late = compute(&i);
        assert_eq!(
            (late.lead_hours, late.reasons.get("lead_hours")),
            (None, Some(&"created_after_submit"))
        );
    }

    #[test]
    fn a_push_before_any_code_edit_is_not_a_submission() {
        let sessions = [current(vec![
            sig(Pushed, 0),
            sig(CodeEdit, 5),
            sig(Pushed, 9),
            slice(0, 9, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!((t.submitted_at, t.submitted_via), (Some(9 * MIN), Some("push")));
        assert_eq!(t.execution_hours, Some(4.0 / 60.0));
    }

    #[test]
    fn costs_need_a_rate_and_a_full_price() {
        let sessions = [fixture_a()];
        let u = Usage { api_cost: Some(2.0), api_cost_n: 1, sessions: 1, ..Default::default() };
        let mut s = settings(15);
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!((t.effort_cost, t.reasons.get("effort_cost")), (None, Some(&"no_rate")));
        assert_eq!(
            (t.cost_per_prompt_cycle, t.reasons.get("cost_per_prompt_cycle")),
            (None, Some(&"no_prompts"))
        );
        s.hourly_rate = Some(60.0);
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(t.effort_cost, Some(7.0), "60/h × 7 worked minutes");
        assert_eq!(t.rework_cost, Some(3.0));
        let partial = Usage {
            api_cost: Some(2.0),
            api_cost_n: 1,
            sessions: 1,
            api_cost_partial: 1,
            ..Default::default()
        };
        let with_prompt = [current(vec![sig(Prompt, 0), slice(0, 0, None)])];
        let t = compute(&input(&with_prompt, &partial, &s));
        assert_eq!(t.reasons.get("cost_per_prompt_cycle"), Some(&"partial_cost"));
    }

    #[test]
    fn estimates_and_calibration() {
        let sessions = [fixture_a()];
        let u = Usage::default();
        let mut s = settings(15);
        let mut i = input(&sessions, &u, &s);
        i.estimate = Estimate::Points(2.0);
        assert_eq!(
            (compute(&i).estimated_points, compute(&i).reasons.get("expected_minutes")),
            (Some(2.0), Some(&"no_calibration"))
        );
        s.minutes_per_point = Some(3.0);
        let i = TicketInput {
            settings: &s,
            estimate: Estimate::Points(2.0),
            ..input(&sessions, &u, &s)
        };
        let t = compute(&i);
        assert_eq!((t.expected_minutes, t.time_variance_minutes), (Some(6.0), Some(1.0)));
        for (e, why) in [
            (Estimate::NoField, "no_estimation_field"),
            (Estimate::NotEstimated, "not_estimated"),
            (Estimate::NotSynced, "not_synced"),
        ] {
            let i = TicketInput { estimate: e, ..input(&sessions, &u, &s) };
            assert_eq!(compute(&i).reasons.get("estimated_points"), Some(&why));
        }
    }

    #[test]
    fn unknown_and_unclassified_runs_are_counted() {
        let sessions = [current(vec![
            run(RunUnknown, 0, Category::Test),
            Signal { category: Some(Category::Build), ..sig(UnmatchedCall, 1) },
            sig(UnmatchedCall, 2),
            sig(UnclassifiedFail, 3),
            slice(0, 3, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!((t.runs_unknown, t.unclassified_failures), (Some(2), Some(2)));
        assert_eq!((t.build_runs, t.build_failure_rate), (Some(0), None));
        assert_eq!(t.reasons.get("build_failure_rate"), Some(&"no_runs"));
    }

    #[test]
    fn no_sessions() {
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&[], &u, &s));
        assert_eq!((t.worked_minutes, t.prompts), (None, None));
        assert_eq!(t.reasons.get("worked_minutes"), Some(&"no_sessions"));
    }

    // ---- pre-build audit (A5) ----

    #[test]
    fn a_hold_with_no_reason_counts_as_unknown() {
        let sessions = [current(vec![
            sig(Prompt, 0),
            slice(0, 0, None),
            sig(CodeEdit, 40),
            sig(Pushed, 41),
            slice(40, 41, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        // The window starts at the first code edit (40), so the chat turn's hold is outside it.
        assert_eq!(t.on_hold_by_reason, Some(BTreeMap::new()));
        let sessions = [current(vec![
            sig(CodeEdit, 0),
            slice(0, 0, None),
            sig(CodeEdit, 40),
            sig(Pushed, 41),
            slice(40, 41, Some(HoldReason::Unknown)),
        ])];
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(t.on_hold_by_reason, Some(BTreeMap::from([("unknown", 40.0)])));
        assert_eq!(t.on_hold_minutes, Some(40.0));
    }

    #[test]
    fn an_escape_is_judged_on_the_category_itself() {
        let sessions = [current(vec![
            sig(TaskTick, 0),
            run(TestFail, 1, Category::E2e),
            run(TestOk, 2, Category::Test),
            slice(0, 2, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!((t.escaped_task_defects, t.test_defects), (Some(0), Some(1)));
    }

    #[test]
    fn rework_is_summed_over_the_union() {
        let sessions = [
            current(vec![
                sig(CodeEdit, 0),
                sig(PrCreated, 1),
                slice(0, 1, None),
                sig(CodeEdit, 45),
                slice(40, 60, None),
            ]),
            current(vec![sig(CodeEdit, 55), slice(50, 70, None)]),
        ];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(
            t.rework_minutes,
            Some(25.0),
            "one interval [40,70], from its first edit after submission"
        );
        assert!(t.post_review_rework_ratio.unwrap() <= 100.0);
    }

    /// A9: a failed derivation is not a lag. It is named apart, so write-back
    /// does not wait on it for ever; a session still waiting outranks it.
    #[test]
    fn a_failed_derivation_is_named_apart_from_one_still_waiting() {
        let waiting = SessionSignals { current: false, ..Default::default() };
        let failed = SessionSignals { failed: true, ..Default::default() };
        let done = current(vec![]);
        assert_eq!(underived([&done]), None);
        assert_eq!(underived([&done, &failed]), Some("derivation_failed"));
        assert_eq!(underived([&failed, &waiting]), Some("not_derived"));
        let sessions = [failed];
        let t = compute(&input(&sessions, &Usage::default(), &settings(15)));
        assert_eq!(t.compile_defects, None);
        assert_eq!(t.reasons.get("compile_defects"), Some(&"derivation_failed"));
    }

    #[test]
    fn an_estimate_the_tracker_cannot_supply_reads_unsupported() {
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&TicketInput { estimate: Estimate::Unsupported, ..input(&[], &u, &s) });
        assert_eq!(t.estimated_points, None);
        assert_eq!(t.reasons.get("estimated_points"), Some(&"unsupported_by_tracker"));
    }

    #[test]
    fn every_reason_is_a_registered_code() {
        let registered = crate::sdlc::registered_reasons();
        let u = Usage::default();
        let s = settings(15);
        let a = [fixture_a()];
        let stale = [SessionSignals { current: false, ..Default::default() }];
        let broken = [SessionSignals { failed: true, ..Default::default() }];
        let idle = [current(vec![sig(Prompt, 0)])];
        let mut seen = std::collections::BTreeSet::new();
        for (sessions, estimate, created) in [
            (&[][..], Estimate::NotSynced, None),
            (&stale[..], Estimate::NoField, None),
            (&a[..], Estimate::NotEstimated, Some(40 * MIN)),
            (&a[..], Estimate::Points(1.0), None),
            (&idle[..], Estimate::NotSynced, None),
            (&broken[..], Estimate::NotSynced, None),
            (&a[..], Estimate::Unsupported, None),
        ] {
            let t = compute(&TicketInput {
                estimate,
                issue_created_at: created,
                ..input(sessions, &u, &s)
            });
            seen.extend(t.reasons.values().copied());
        }
        let missing: Vec<_> = seen.iter().filter(|c| !registered.contains(**c)).collect();
        assert!(missing.is_empty(), "unregistered reason codes: {missing:?}");
        assert!(seen.len() >= 12, "the inputs above exercise most codes: {seen:?}");
    }

    // ---- post-implementation review (F4) ----

    fn fix(minute: i64) -> Signal {
        Signal { reference: Some("x/a.rs".into()), ..sig(ReviewFix, minute) }
    }

    #[test]
    fn review_fixes_count_and_split_the_defects() {
        let sessions = [current(vec![
            sig(CodeEdit, 0),
            fix(1),
            fix(2),
            run(BuildFail, 3, Category::Build),
            slice(0, 3, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!(
            (t.review_fixes, t.review_defects, t.total_defects),
            (Some(2), Some(2), Some(3))
        );
        assert!((t.review_efficiency.unwrap() - 200.0 / 3.0).abs() < 1e-9);
        assert!((t.testing_efficiency.unwrap() - 100.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn recovery_pairs_within_a_category_and_averages() {
        let sessions = [current(vec![
            run(BuildFail, 0, Category::Typecheck),
            run(BuildOk, 1, Category::Build),
            run(BuildFail, 2, Category::Build),
            run(BuildOk, 6, Category::Build),
            run(BuildOk, 10, Category::Typecheck),
            run(BuildFail, 11, Category::Build),
            slice(0, 11, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        // typecheck 0 -> 10 (10 min), build 2 -> 6 (4 min); the build fail at 11 never recovers.
        assert_eq!((t.mean_build_recovery_minutes, t.build_unrecovered), (Some(7.0), Some(1)));
        let none = [current(vec![run(BuildFail, 0, Category::Build), slice(0, 0, None)])];
        let t = compute(&input(&none, &u, &s));
        assert_eq!(
            (t.mean_build_recovery_minutes, t.reasons.get("mean_build_recovery_minutes")),
            (None, Some(&"no_recovery"))
        );
    }

    #[test]
    fn rework_cycles_need_an_edit_between_pushes() {
        let sessions = [current(vec![
            sig(CodeEdit, 0),
            sig(Pushed, 1),
            sig(Pushed, 2),
            sig(CodeEdit, 3),
            sig(Pushed, 4),
            sig(CodeEdit, 5),
            sig(Pushed, 6),
            sig(Pushed, 7),
            slice(0, 7, Some(HoldReason::Unknown)),
        ])];
        let (u, s) = (Usage::default(), settings(15));
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!((t.submitted_at, t.rework_cycles), (Some(MIN), Some(2)));
    }

    #[test]
    fn dependent_figures_inherit_their_input_reason() {
        let sessions = [fixture_a()];
        let u = Usage::default();
        let s = MetricSettings { minutes_per_point: Some(3.0), ..settings(15) };
        for (e, why) in [
            (Estimate::NoField, "no_estimation_field"),
            (Estimate::NotEstimated, "not_estimated"),
            (Estimate::NotSynced, "not_synced"),
        ] {
            let t = compute(&TicketInput { estimate: e, ..input(&sessions, &u, &s) });
            assert_eq!(t.reasons.get("time_variance_minutes"), Some(&why));
        }
        let s = settings(15);
        let t =
            compute(&TicketInput { estimate: Estimate::Points(9.0), ..input(&sessions, &u, &s) });
        assert_eq!(t.reasons.get("time_variance_minutes"), Some(&"no_calibration"));
        let s = MetricSettings { minutes_per_point: Some(3.0), ..settings(15) };
        let t =
            compute(&TicketInput { estimate: Estimate::Points(9.0), ..input(&sessions, &u, &s) });
        assert_eq!(t.time_variance_minutes, Some(7.0 - 27.0), "under the estimate is negative");
    }

    #[test]
    fn prompts_and_cost_per_prompt() {
        let sessions = [current(vec![
            sig(Prompt, 0),
            sig(Prompt, 1),
            sig(Prompt, 2),
            sig(Prompt, 3),
            slice(0, 3, None),
        ])];
        let u = Usage { api_cost: Some(2.0), api_cost_n: 2, sessions: 2, ..Default::default() };
        let s = settings(15);
        let t = compute(&input(&sessions, &u, &s));
        assert_eq!((t.prompts, t.cost_per_prompt_cycle), (Some(4), Some(0.5)));
        let short = Usage { api_cost: Some(2.0), api_cost_n: 1, sessions: 2, ..Default::default() };
        assert_eq!(
            compute(&input(&sessions, &short, &s)).reasons.get("cost_per_prompt_cycle"),
            Some(&"partial_cost")
        );
        let unpriced = Usage { sessions: 1, ..Default::default() };
        let t = compute(&input(&sessions, &unpriced, &s));
        assert_eq!((t.ai_token_cost, t.reasons.get("ai_token_cost")), (None, Some(&"not_priced")));
        assert_eq!(t.reasons.get("cost_per_prompt_cycle"), Some(&"not_priced"));
    }

    #[test]
    fn escape_boundaries() {
        let (u, s) = (Usage::default(), settings(15));
        // A PR between the tick and the failure ends the tick's window.
        let cut = [current(vec![
            sig(TaskTick, 0),
            sig(PrCreated, 1),
            run(TestFail, 2, Category::Test),
            run(TestOk, 3, Category::Test),
            slice(0, 3, None),
        ])];
        let t = compute(&input(&cut, &u, &s));
        assert_eq!((t.escaped_task_defects, t.test_defects), (Some(0), Some(1)));
        // A failure that never recovers is an ordinary defect, not an escape.
        let stuck =
            [current(vec![sig(TaskTick, 0), run(TestFail, 1, Category::Test), slice(0, 1, None)])];
        let t = compute(&input(&stuck, &u, &s));
        assert_eq!((t.escaped_task_defects, t.test_defects), (Some(0), Some(1)));
        // Two ticks, one failure: only the later tick's window holds it.
        let two = [current(vec![
            sig(TaskTick, 0),
            sig(TaskTick, 1),
            run(TestFail, 2, Category::Test),
            run(TestFail, 3, Category::Test),
            run(TestOk, 4, Category::Test),
            slice(0, 4, None),
        ])];
        let t = compute(&input(&two, &u, &s));
        assert_eq!((t.escaped_task_defects, t.test_defects), (Some(1), Some(0)));
    }

    #[test]
    fn every_null_has_a_reason() {
        let u = Usage::default();
        let s = settings(15);
        let edited = [current(vec![sig(CodeEdit, 0), slice(0, 0, None)])];
        let empty = [current(vec![])];
        for sessions in [&[][..], &edited[..], &empty[..], &[fixture_a()][..]] {
            let t = compute(&input(sessions, &u, &s));
            let v = serde_json::to_value(&t).unwrap();
            for (key, value) in v.as_object().unwrap() {
                if value.is_null() {
                    assert!(t.reasons.contains_key(key.as_str()), "{key} is null with no reason");
                }
            }
        }
        let t = compute(&input(&empty, &u, &s));
        assert_eq!(t.reasons.get("worked_minutes"), Some(&"no_activity"));
        let t = compute(&input(&edited, &u, &s));
        assert_eq!(t.reasons.get("submitted_via"), Some(&"not_submitted"));
        let t = compute(&input(&[], &u, &s));
        assert_eq!(t.reasons.get("submitted_via"), Some(&"no_sessions"));
    }

    #[test]
    fn instants_serialise_as_rfc3339() {
        let sessions = [fixture_a()];
        let (u, s) = (Usage::default(), settings(15));
        let v = serde_json::to_value(compute(&input(&sessions, &u, &s))).unwrap();
        assert_eq!(v["work_started_at"], serde_json::json!("1970-01-01T00:00:00Z"));
        assert_eq!(v["submitted_at"], serde_json::json!("1970-01-01T00:32:00Z"));
    }
}
