//! Usage write-back (spec `docs/spec/daemon/tracker-metrics-writeback.md`): after a
//! successful sync, write each ticket's R7 usage to the tracker, for tickets
//! assigned to the token owner only. Provider-neutral: the adapter turns a
//! [`Report`] into its own format and sends it; every SQL statement lives in
//! `db::pg_store`.

use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::db::pg_store::trackers::{WriteTarget, WritebackTask};
use crate::tracker::runtime::{SYNC_RETRIES, TransportFactory};
use crate::tracker::secret::{KEYRING_TIMEOUT, SecretStore, bounded};
use crate::tracker::usage::Usage;
use crate::tracker::{AccountRef, GetTransport, TrackerAdapter, TrackerError, WriteTransport};

/// The whole pass, 429 sleeps included (W2).
pub const WRITEBACK_DEADLINE: Duration = Duration::from_secs(5 * 60);
/// 429 retries per write request (W2), the same budget as a sync's.
pub const WRITE_RETRIES: u8 = 3;
/// Consecutive `rate_limited` tasks that end a pass (W9).
pub const RATE_LIMITED_STOP: usize = 5;

/// A ticket's last write-back outcome — the `sensei.tracker_writeback_status` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackStatus {
    Ok,
    Forbidden,
    FieldInvalid,
    CommentRemoved,
    NotFound,
    RateLimited,
    Error,
}

impl WritebackStatus {
    /// In the enum's declared order.
    pub const ALL: [WritebackStatus; 7] = [
        WritebackStatus::Ok,
        WritebackStatus::Forbidden,
        WritebackStatus::FieldInvalid,
        WritebackStatus::CommentRemoved,
        WritebackStatus::NotFound,
        WritebackStatus::RateLimited,
        WritebackStatus::Error,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            WritebackStatus::Ok => "ok",
            WritebackStatus::Forbidden => "forbidden",
            WritebackStatus::FieldInvalid => "field_invalid",
            WritebackStatus::CommentRemoved => "comment_removed",
            WritebackStatus::NotFound => "not_found",
            WritebackStatus::RateLimited => "rate_limited",
            WritebackStatus::Error => "error",
        }
    }
    pub fn parse(s: &str) -> Option<WritebackStatus> {
        WritebackStatus::ALL.into_iter().find(|w| w.as_str() == s)
    }
}

/// The request a status came from (W9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOp {
    FindComment,
    CreateComment,
    UpdateComment,
    IssueCheck,
    WriteFields,
}

impl WriteOp {
    /// Every operation, for the exhaustive status table test.
    #[cfg(test)]
    pub const ALL: [WriteOp; 5] = [
        WriteOp::FindComment,
        WriteOp::CreateComment,
        WriteOp::UpdateComment,
        WriteOp::IssueCheck,
        WriteOp::WriteFields,
    ];
}

/// The W9 table: what a failed request leaves on the ticket. Total over every
/// operation and error. An update's `NotFound` is never final on its own: the pass
/// asks [`WriteOp::IssueCheck`] which object is gone (W5).
pub fn status_for(op: WriteOp, e: TrackerError) -> WritebackStatus {
    use TrackerError as E;
    match (op, e) {
        // The issue check only tells which object a 404 meant (W5).
        (WriteOp::IssueCheck, E::NotFound) => WritebackStatus::NotFound,
        (WriteOp::IssueCheck, _) => WritebackStatus::Error,
        // A mapped-field write: a 400 is the field the tracker refused (W6, W9).
        (WriteOp::WriteFields, E::BadRequest) => WritebackStatus::FieldInvalid,
        // The comment-list read (W9).
        (WriteOp::FindComment, E::Forbidden | E::Unauthorized) => WritebackStatus::Forbidden,
        (WriteOp::FindComment, E::NotFound) => WritebackStatus::NotFound,
        (WriteOp::FindComment, _) => WritebackStatus::Error,
        (_, E::Forbidden | E::Unauthorized) => WritebackStatus::Forbidden,
        (WriteOp::UpdateComment, E::NotFound) => WritebackStatus::Error,
        (_, E::NotFound) => WritebackStatus::NotFound,
        (_, E::RateLimited) => WritebackStatus::RateLimited,
        (
            _,
            E::BadRequest
            | E::HttpError
            | E::Transport
            | E::Decode
            | E::Redirect
            | E::Db
            | E::TokenMissing
            | E::CredentialStore
            | E::UnsupportedTracker,
        ) => WritebackStatus::Error,
    }
}

/// Whether a ticket in this status is skipped until something changes (W4).
pub fn is_sticky(status: WritebackStatus) -> bool {
    matches!(
        status,
        WritebackStatus::Forbidden
            | WritebackStatus::FieldInvalid
            | WritebackStatus::CommentRemoved
            | WritebackStatus::NotFound
    )
}

/// A usage metric a field can be mapped to — the `sensei.tracker_metric` enum (W3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrackerMetric {
    Sessions,
    ActiveHours,
    ClaudeHours,
    UserHours,
    WallHours,
    CycleHours,
    TokensFresh,
    CacheRead,
    CacheWrite,
    TokensOut,
    TokensReasoning,
    MeteredCost,
    ApiCost,
}

impl TrackerMetric {
    /// In the enum's declared order, the order fields are written and hashed in.
    pub const ALL: [TrackerMetric; 13] = [
        TrackerMetric::Sessions,
        TrackerMetric::ActiveHours,
        TrackerMetric::ClaudeHours,
        TrackerMetric::UserHours,
        TrackerMetric::WallHours,
        TrackerMetric::CycleHours,
        TrackerMetric::TokensFresh,
        TrackerMetric::CacheRead,
        TrackerMetric::CacheWrite,
        TrackerMetric::TokensOut,
        TrackerMetric::TokensReasoning,
        TrackerMetric::MeteredCost,
        TrackerMetric::ApiCost,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            TrackerMetric::Sessions => "sessions",
            TrackerMetric::ActiveHours => "active_hours",
            TrackerMetric::ClaudeHours => "claude_hours",
            TrackerMetric::UserHours => "user_hours",
            TrackerMetric::WallHours => "wall_hours",
            TrackerMetric::CycleHours => "cycle_hours",
            TrackerMetric::TokensFresh => "tokens_fresh",
            TrackerMetric::CacheRead => "cache_read",
            TrackerMetric::CacheWrite => "cache_write",
            TrackerMetric::TokensOut => "tokens_out",
            TrackerMetric::TokensReasoning => "tokens_reasoning",
            TrackerMetric::MeteredCost => "metered_cost",
            TrackerMetric::ApiCost => "api_cost",
        }
    }
    pub fn parse(s: &str) -> Option<TrackerMetric> {
        TrackerMetric::ALL.into_iter().find(|m| m.as_str() == s)
    }
}

/// The span from the first session's start to the last one's end; `None` when
/// either is unknown or it is negative, by however little (W3).
pub fn cycle_secs(u: &Usage) -> Option<f64> {
    let span = u.last_at? - u.first_at?;
    (span >= chrono::TimeDelta::zero()).then(|| span.as_seconds_f64())
}

/// What a field mapped to `metric` is written for this usage (W3): the comment's
/// numbers, as numbers.
pub fn metric_value(metric: TrackerMetric, u: &Usage) -> crate::tracker::FieldValue {
    use crate::tracker::FieldValue as V;
    let h = |secs: Option<f64>| secs.and_then(hours).map_or(V::Null, V::Dec);
    let n = |x: Option<i64>| x.map_or(V::Null, V::Int);
    let money = |x: Option<f64>| x.map(amount).map_or(V::Null, V::Dec);
    match metric {
        TrackerMetric::Sessions => V::Int(u.sessions),
        TrackerMetric::ActiveHours => h(u.active_secs),
        TrackerMetric::ClaudeHours => h(u.claude_secs),
        TrackerMetric::UserHours => h(u.user_secs),
        TrackerMetric::WallHours => h(u.wall_secs),
        TrackerMetric::CycleHours => h(cycle_secs(u)),
        TrackerMetric::TokensFresh => n(u.tokens_fresh),
        TrackerMetric::CacheRead => n(u.cache_read),
        TrackerMetric::CacheWrite => n(u.cache_write),
        TrackerMetric::TokensOut => n(u.tokens_out),
        TrackerMetric::TokensReasoning => n(u.tokens_reasoning),
        TrackerMetric::MeteredCost => money(u.metered_cost),
        TrackerMetric::ApiCost => money(u.api_cost),
    }
}

/// The fields surface's change key (W4): the pairs as written, in metric order.
pub fn fields_hash(pairs: &[(String, crate::tracker::FieldValue)]) -> String {
    use sha2::Digest;
    let lines = pairs.iter().map(|(r, v)| format!("{r}={}", v.canonical()));
    let payload: Vec<String> = std::iter::once("fields".to_string()).chain(lines).collect();
    hex::encode(sha2::Sha256::digest(payload.join("\n")))
}

/// How every report's title starts: what marks a comment as sensei's (W5 adoption).
pub const REPORT_TITLE_PREFIX: &str = "Sensei · AI usage";

/// What the comment surface writes: a title and its lines, provider-neutral.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub title: String,
    pub lines: Vec<String>,
}

/// Hours to 2 dp, half away from zero with the division last (W3); a negative
/// span is no value.
pub fn hours(secs: f64) -> Option<String> {
    (secs >= 0.0).then(|| two_dp((secs * 100.0 / 3600.0).round()))
}

/// `hundredths / 100` to two places, never `-0.00`.
fn two_dp(hundredths: f64) -> String {
    let x = hundredths / 100.0;
    format!("{:.2}", if x == 0.0 { 0.0 } else { x })
}

/// A decimal amount to 2 dp, half away from zero (W3).
pub fn amount(x: f64) -> String {
    two_dp((x * 100.0).round())
}

/// The comment for one ticket's usage (W5).
pub fn render(u: &Usage, updated_at: chrono::DateTime<chrono::Utc>) -> Report {
    const MISSING: &str = "—";
    // A value, then its coverage mark when only some sessions carried it (W5).
    let marked = |value: Option<String>, n: i64| match value {
        None => MISSING.to_string(),
        Some(v) if 0 < n && n < u.sessions => format!("{v} [{n} of {} sessions]", u.sessions),
        Some(v) => v,
    };
    let h = |secs: Option<f64>, n: i64| marked(secs.and_then(hours).map(|v| format!("{v} h")), n);
    let count = |x: Option<i64>, n: i64| marked(x.map(|x| x.to_string()), n);
    let plural = |n: i64| if n == 1 { "session" } else { "sessions" };

    let mut sessions = format!("Sessions: {}", u.sessions);
    if u.shared > 0 {
        sessions.push_str(&format!(" ({} shared with other tickets)", u.shared));
    }
    let cycle = cycle_secs(u)
        .and_then(hours)
        .map(|v| format!("{v} h"))
        .unwrap_or_else(|| MISSING.to_string());
    let mut api = marked(u.api_cost.map(|c| format!("${}", amount(c))), u.api_cost_n);
    if u.api_cost.is_some() && u.api_cost_partial > 0 {
        let n = u.api_cost_partial;
        api.push_str(&format!(" (partly priced for {n} {})", plural(n)));
    }
    Report {
        title: format!(
            "{REPORT_TITLE_PREFIX} from the assignee's sessions (updated {} UTC)",
            updated_at.format("%Y-%m-%d %H:%M")
        ),
        lines: vec![
            sessions,
            format!(
                "Active time: {} (Claude {}, you {})",
                h(u.active_secs, u.active_n),
                h(u.claude_secs, u.claude_n),
                h(u.user_secs, u.user_n)
            ),
            format!("Wall-clock time: {}", h(u.wall_secs, u.wall_n)),
            format!("Cycle time: {cycle}"),
            format!(
                "Tokens: fresh {} · cache read {} · cache write {} · out {} · reasoning {}",
                count(u.tokens_fresh, u.tokens_fresh_n),
                count(u.cache_read, u.cache_read_n),
                count(u.cache_write, u.cache_write_n),
                count(u.tokens_out, u.tokens_out_n),
                count(u.tokens_reasoning, u.tokens_reasoning_n)
            ),
            format!(
                "Cost: metered {} · API {api}",
                marked(u.metered_cost.map(amount), u.metered_cost_n)
            ),
        ],
    }
}

/// The change key of a comment: its lines, never its title (which carries the
/// "updated" time).
pub fn comment_hash(r: &Report) -> String {
    use sha2::Digest;
    let payload = std::iter::once("comment").chain(r.lines.iter().map(String::as_str));
    hex::encode(sha2::Sha256::digest(payload.collect::<Vec<_>>().join("\n")))
}

/// How a pass ended (the board shows it through `writeback_error`; this is for tests and logs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassOutcome {
    /// Nothing to do: the tracker is gone, write-back is off, or the owner is unknown.
    Skipped,
    /// Every candidate was looked at.
    Done { written: usize, unchanged: usize, failed: usize },
    /// The pass ended early with this `writeback_error` code.
    Stopped(&'static str),
    /// The token changed under the pass: it recorded nothing more.
    Superseded,
}

/// After a successful sync (W2): take the tracker's guard now and run the pass in
/// the background, so the sync's caller never waits for it. `None` when the guard
/// is busy (no pass this round).
pub fn spawn_after_sync(
    pg: &PgStore,
    rt: &crate::tracker::runtime::TrackerRuntime,
    tracker_id: uuid::Uuid,
    secret_service: &str,
    adapter: &'static dyn TrackerAdapter,
) -> Option<tokio::task::JoinHandle<PassOutcome>> {
    let guard = rt.locks.try_lock(tracker_id)?;
    let (pg, io, service) = (pg.clone(), PassIo::of(rt), secret_service.to_string());
    Some(tokio::spawn(async move {
        run(&pg, &io, tracker_id, &service, adapter, WRITEBACK_DEADLINE, guard).await
    }))
}

/// What a pass reaches outside the database: the keyring and the transports.
#[derive(Clone)]
pub struct PassIo {
    pub secrets: Arc<dyn SecretStore>,
    pub transports: Arc<dyn TransportFactory>,
}

impl PassIo {
    pub fn of(rt: &crate::tracker::runtime::TrackerRuntime) -> Self {
        PassIo { secrets: rt.secrets.clone(), transports: rt.transports.clone() }
    }
}

/// One write-back pass over a tracker, holding its guard (W2–W5, W9).
/// Whether `task` may still hold a value sensei wrote to a field retired at
/// `retired_at` (W6): nothing has landed on it since, and it was tried at all. A
/// ticket last tried before then may have taken the write even if it failed.
pub(crate) fn clear_pending(
    task: &WritebackTask,
    retired_at: chrono::DateTime<chrono::Utc>,
) -> bool {
    match task.written_at {
        Some(w) => w < retired_at,
        None => task.tried_at.is_some(),
    }
}

pub async fn run(
    pg: &PgStore,
    io: &PassIo,
    tracker_id: uuid::Uuid,
    secret_service: &str,
    adapter: &'static dyn TrackerAdapter,
    deadline: Duration,
    _guard: tokio::sync::OwnedMutexGuard<()>,
) -> PassOutcome {
    // Read at pass start: the sync may have just found the owner (W1). The sync's
    // token binds every record, this failure's included (W2).
    let row = match pg.tracker_by_id(tracker_id).await {
        Ok(Some(row)) if row.secret_service == secret_service => row,
        Ok(Some(_)) => return PassOutcome::Superseded,
        Ok(None) => return PassOutcome::Skipped,
        Err(_) => {
            let pass = Pass { pg, tracker_id, service: secret_service.to_string(), adapter };
            return pass.stop(TrackerError::Db.code()).await;
        }
    };
    let Some(owner) = row.owner_ref.clone().filter(|_| row.writeback_enabled) else {
        return PassOutcome::Skipped;
    };
    let pass = Pass { pg, tracker_id, service: row.secret_service.clone(), adapter };
    let in_flight = std::sync::Mutex::new(None);
    let body = pass.run(io.secrets.clone(), io.transports.as_ref(), &row, &owner, &in_flight);
    match tokio::time::timeout(deadline, body).await {
        Ok(outcome) => outcome,
        Err(_) => {
            // The request in flight may or may not have landed; the next pass's
            // find_comment adopts a comment that did (W2).
            let task = *in_flight.lock().unwrap();
            if let Some(task) = task
                && let Err(e) = pg
                    .record_write_status(task, &pass.service, WritebackStatus::Error.as_str())
                    .await
            {
                let _ = e;
                tracing::warn!(tracker_id = %tracker_id, task_id = %task, code = TrackerError::Db.code(), "write-back: could not record the deadline");
            }
            pass.stop(TrackerError::Transport.code()).await
        }
    }
}

/// One pass's context: every write it records is bound to `service` (W2).
struct Pass<'a> {
    pg: &'a PgStore,
    tracker_id: uuid::Uuid,
    service: String,
    adapter: &'static dyn TrackerAdapter,
}

impl Pass<'_> {
    /// End the pass early, recording why on the tracker.
    async fn stop(&self, code: &'static str) -> PassOutcome {
        match self.pg.set_writeback_error(self.tracker_id, &self.service, Some(code)).await {
            Ok(true) => PassOutcome::Stopped(code),
            Ok(false) => PassOutcome::Superseded,
            Err(_) => {
                tracing::warn!(tracker_id = %self.tracker_id, code, "write-back: could not record why it stopped");
                PassOutcome::Stopped(code)
            }
        }
    }

    async fn run(
        &self,
        secrets: Arc<dyn SecretStore>,
        transports: &dyn TransportFactory,
        row: &crate::db::pg_store::trackers::TrackerRow,
        owner: &AccountRef,
        in_flight: &std::sync::Mutex<Option<uuid::Uuid>>,
    ) -> PassOutcome {
        // The credential, once, for both transports (W2).
        let (service, account) = (self.service.clone(), row.account.clone());
        let token =
            match bounded(secrets, KEYRING_TIMEOUT, move |s| s.get(&service, &account)).await {
                Ok(t) => t,
                Err(e) => return self.stop(e.code()).await,
            };
        let auth = self.adapter.auth(&row.account, &token);
        let transports_built =
            transports.transport(&row.base_url, auth.clone(), SYNC_RETRIES).and_then(|get| {
                Ok((get, transports.write_transport(&row.base_url, auth, WRITE_RETRIES)?))
            });
        let (get, write) = match transports_built {
            Ok(both) => both,
            Err(e) => return self.stop(e.code()).await,
        };
        match self.pg.set_writeback_error(self.tracker_id, &self.service, None).await {
            Ok(true) => {}
            Ok(false) => return PassOutcome::Superseded,
            Err(_) => return self.stop(TrackerError::Db.code()).await,
        }
        let Ok(tasks) = self.pg.writeback_candidates(self.tracker_id).await else {
            return self.stop(TrackerError::Db.code()).await;
        };
        let Ok(rows) = self.pg.tracker_fields(self.tracker_id).await else {
            return self.stop(TrackerError::Db.code()).await;
        };
        // Mapped fields in metric order make the fields surface (W6); retired ones
        // are cleared on each ticket written before they were retired.
        let mut mapped: Vec<(TrackerMetric, String)> = rows
            .iter()
            .filter_map(|r| {
                Some((TrackerMetric::parse(r.metric.as_deref()?)?, r.field_ref.clone()))
            })
            .collect();
        mapped.sort();
        let mut retired: Vec<(String, chrono::DateTime<chrono::Utc>)> = rows
            .iter()
            .filter(|r| r.metric.is_none())
            .filter_map(|r| Some((r.field_ref.clone(), r.retired_at?)))
            .collect();
        let on_fields = !mapped.is_empty();
        let ids: Vec<uuid::Uuid> = tasks.iter().map(|t| t.id).collect();
        let Ok(mut usages) =
            crate::tracker::usage::ticket_usages(self.pg, self.tracker_id, Some(&ids)).await
        else {
            return self.stop(TrackerError::Db.code()).await;
        };

        let (mut written, mut unchanged, mut failed, mut rate_limited) = (0, 0, 0, 0);
        for task in tasks {
            let status = task.status.as_deref().and_then(WritebackStatus::parse);
            // A deleted comment holds back the comment surface only (W4).
            let sticky = status.is_some_and(|s| {
                is_sticky(s) && !(on_fields && s == WritebackStatus::CommentRemoved)
            });
            if sticky {
                continue;
            }
            // A ticket with no candidate sessions is the genuinely empty set (W4); it is
            // written only if something was written to it before.
            let usage = usages.remove(&task.id).unwrap_or_default();
            if usage.sessions == 0 && task.written_at.is_none() {
                continue;
            }
            let clears: Vec<(String, crate::tracker::FieldValue)> = retired
                .iter()
                .filter(|(_, retired_at)| clear_pending(&task, *retired_at))
                .map(|(r, _)| (r.clone(), crate::tracker::FieldValue::Null))
                .collect();
            let report = render(&usage, chrono::Utc::now());
            let pairs: Vec<(String, crate::tracker::FieldValue)> =
                mapped.iter().map(|(m, r)| (r.clone(), metric_value(*m, &usage))).collect();
            let hash = if on_fields { fields_hash(&pairs) } else { comment_hash(&report) };
            // Unchanged only if that hash is what last landed (a failed write keeps the old
            // one) and no retired field is left to clear.
            if task.hash.as_deref() == Some(hash.as_str())
                && status == Some(WritebackStatus::Ok)
                && clears.is_empty()
            {
                unchanged += 1;
                continue;
            }
            match self.pg.writeback_target(task.id, &self.service).await {
                Ok(WriteTarget::Go) => {}
                Ok(WriteTarget::Reassigned) => continue,
                Ok(WriteTarget::Stop) => return PassOutcome::Superseded,
                Err(_) => return self.stop(TrackerError::Db.code()).await,
            }
            *in_flight.lock().unwrap() = Some(task.id);
            let clear_refs: Vec<String> = clears.iter().map(|(r, _)| r.clone()).collect();
            let write_fields = |fields: Vec<(String, crate::tracker::FieldValue)>| {
                let (write, ext) = (write.as_ref(), task.external_id.as_str());
                async move { self.adapter.write_fields(write, ext, &fields).await }
            };
            // A retired field the tracker refuses to clear (deleted, say) is dropped rather
            // than holding the ticket back for good (W6).
            let (result, unclearable) = if on_fields {
                match write_fields(pairs.iter().cloned().chain(clears).collect()).await {
                    Err(TrackerError::BadRequest) if !clear_refs.is_empty() => {
                        match write_fields(pairs).await {
                            Ok(()) => (Ok(None), true),
                            Err(e) => (Err(status_for(WriteOp::WriteFields, e)), false),
                        }
                    }
                    other => (
                        other.map(|()| None).map_err(|e| status_for(WriteOp::WriteFields, e)),
                        false,
                    ),
                }
            } else {
                match self.write_comment(get.as_ref(), write.as_ref(), &task, owner, &report).await
                {
                    Ok(comment_ref) if clears.is_empty() => (Ok(Some(comment_ref)), false),
                    Ok(comment_ref) => match write_fields(clears).await {
                        Ok(()) => (Ok(Some(comment_ref)), false),
                        Err(TrackerError::BadRequest) => (Ok(Some(comment_ref)), true),
                        Err(e) => (Err(status_for(WriteOp::WriteFields, e)), false),
                    },
                    Err(status) => (Err(status), false),
                }
            };
            *in_flight.lock().unwrap() = None;
            if unclearable {
                tracing::warn!(tracker_id = %self.tracker_id, task_id = %task.id, "write-back: a retired field could not be cleared; dropped");
                match self
                    .pg
                    .delete_retired_fields(self.tracker_id, &self.service, &clear_refs)
                    .await
                {
                    Ok(_) => retired.retain(|(r, _)| !clear_refs.contains(r)),
                    Err(_) => return self.stop(TrackerError::Db.code()).await,
                }
            }
            let recorded = match &result {
                Ok(comment_ref) => {
                    self.pg
                        .record_write_ok(task.id, &self.service, &hash, comment_ref.as_deref())
                        .await
                }
                Err(status) => {
                    self.pg.record_write_status(task.id, &self.service, status.as_str()).await
                }
            };
            match recorded {
                Ok(true) => {}
                Ok(false) => return PassOutcome::Superseded,
                Err(_) => {
                    tracing::warn!(tracker_id = %self.tracker_id, task_id = %task.id, code = TrackerError::Db.code(), "write-back: could not record a write");
                    return self.stop(TrackerError::Db.code()).await;
                }
            }
            match result {
                Ok(_) => {
                    written += 1;
                    rate_limited = 0;
                }
                Err(status) => {
                    failed += 1;
                    tracing::warn!(tracker_id = %self.tracker_id, task_id = %task.id, status = status.as_str(), "write-back: write failed");
                    if status == WritebackStatus::RateLimited {
                        rate_limited += 1;
                        if rate_limited >= RATE_LIMITED_STOP {
                            return self.stop(TrackerError::RateLimited.code()).await;
                        }
                    } else {
                        rate_limited = 0;
                    }
                }
            }
        }
        PassOutcome::Done { written, unchanged, failed }
    }

    /// Create or update the ticket's comment (W5); the comment's id, or the status
    /// a failure leaves.
    async fn write_comment(
        &self,
        get: &dyn GetTransport,
        write: &dyn WriteTransport,
        task: &WritebackTask,
        owner: &AccountRef,
        report: &Report,
    ) -> Result<String, WritebackStatus> {
        let ext = task.external_id.as_str();
        let comment_ref = match &task.comment_ref {
            Some(r) => r.clone(),
            None => match self.adapter.find_comment(get, ext, owner).await {
                Ok(Some(r)) => r,
                Ok(None) => {
                    return self
                        .adapter
                        .create_comment(write, ext, report)
                        .await
                        .map_err(|e| status_for(WriteOp::CreateComment, e));
                }
                Err(e) => return Err(status_for(WriteOp::FindComment, e)),
            },
        };
        match self.adapter.update_comment(write, ext, &comment_ref, report).await {
            Ok(()) => Ok(comment_ref),
            // Which object is gone: the comment, or the whole ticket (W5)?
            Err(TrackerError::NotFound) => match self.adapter.issue_exists(get, ext).await {
                Ok(true) => Err(WritebackStatus::CommentRemoved),
                Ok(false) => Err(WritebackStatus::NotFound),
                Err(e) => Err(status_for(WriteOp::IssueCheck, e)),
            },
            Err(e) => Err(status_for(WriteOp::UpdateComment, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&chrono::Utc)
    }

    /// W5 golden example 1.
    fn golden_1() -> Usage {
        Usage {
            sessions: 3,
            shared: 1,
            active_secs: Some(7380.0),
            active_n: 3,
            claude_secs: Some(4680.0),
            claude_n: 3,
            user_secs: Some(2700.0),
            user_n: 3,
            wall_secs: Some(14400.0),
            wall_n: 3,
            first_at: Some(at("2026-10-20T10:00:00Z")),
            last_at: Some(at("2026-10-22T10:00:00Z")),
            tokens_fresh: Some(120000),
            tokens_fresh_n: 3,
            cache_read: Some(900000),
            cache_read_n: 3,
            cache_write: Some(30000),
            cache_write_n: 3,
            tokens_out: Some(45000),
            tokens_out_n: 3,
            tokens_reasoning: None,
            tokens_reasoning_n: 0,
            metered_cost: None,
            metered_cost_n: 0,
            api_cost: Some(4.317),
            api_cost_n: 3,
            api_cost_partial: 1,
            ..Default::default()
        }
    }

    /// W5 golden example 2: the coverage marks.
    fn golden_2() -> Usage {
        Usage {
            sessions: 3,
            shared: 0,
            claude_n: 2,
            wall_n: 1,
            cache_read_n: 2,
            cache_write: None,
            cache_write_n: 0,
            metered_cost: Some(1.5),
            metered_cost_n: 1,
            api_cost: Some(30.0),
            api_cost_n: 2,
            api_cost_partial: 2,
            ..golden_1()
        }
    }

    const UPDATED: &str = "2026-10-26T14:30:00Z";

    #[test]
    fn statuses_are_exactly_the_ddl_enum_labels() {
        let labels = crate::tracker::test_support::ddl_enum_labels("tracker_writeback_status");
        let ours: Vec<&str> = WritebackStatus::ALL.iter().map(|w| w.as_str()).collect();
        assert_eq!(labels, ours);
        for w in WritebackStatus::ALL {
            assert_eq!(WritebackStatus::parse(w.as_str()), Some(w));
        }
    }

    #[test]
    fn golden_1_renders_exactly() {
        let r = render(&golden_1(), at(UPDATED));
        assert_eq!(
            r.title,
            "Sensei · AI usage from the assignee's sessions (updated 2026-10-26 14:30 UTC)"
        );
        assert_eq!(
            r.lines,
            [
                "Sessions: 3 (1 shared with other tickets)",
                "Active time: 2.05 h (Claude 1.30 h, you 0.75 h)",
                "Wall-clock time: 4.00 h",
                "Cycle time: 48.00 h",
                "Tokens: fresh 120000 · cache read 900000 · cache write 30000 · out 45000 · reasoning —",
                "Cost: metered — · API $4.32 (partly priced for 1 session)",
            ]
        );
    }

    #[test]
    fn golden_2_renders_the_coverage_marks_exactly() {
        assert_eq!(
            render(&golden_2(), at(UPDATED)).lines,
            [
                "Sessions: 3",
                "Active time: 2.05 h (Claude 1.30 h [2 of 3 sessions], you 0.75 h)",
                "Wall-clock time: 4.00 h [1 of 3 sessions]",
                "Cycle time: 48.00 h",
                "Tokens: fresh 120000 · cache read 900000 [2 of 3 sessions] · cache write — · out 45000 · reasoning —",
                "Cost: metered 1.50 [1 of 3 sessions] · API $30.00 [2 of 3 sessions] (partly priced for 2 sessions)",
            ]
        );
    }

    #[test]
    fn a_ticket_with_no_sessions_renders_every_value_as_missing() {
        assert_eq!(
            render(&Usage::default(), at(UPDATED)).lines,
            [
                "Sessions: 0",
                "Active time: — (Claude —, you —)",
                "Wall-clock time: —",
                "Cycle time: —",
                "Tokens: fresh — · cache read — · cache write — · out — · reasoning —",
                "Cost: metered — · API —",
            ]
        );
    }

    #[test]
    fn an_unpriced_api_cost_is_missing_and_carries_no_note() {
        let u = Usage {
            sessions: 1,
            api_cost: None,
            api_cost_n: 1,
            api_cost_partial: 1,
            ..Default::default()
        };
        assert_eq!(render(&u, at(UPDATED)).lines[5], "Cost: metered — · API —");
    }

    #[test]
    fn rounding_is_half_away_from_zero_with_the_division_last() {
        assert_eq!(hours(3618.0).as_deref(), Some("1.01"));
        assert_eq!(hours(522.0).as_deref(), Some("0.15"));
        assert_eq!(hours(7380.0).as_deref(), Some("2.05"));
        assert_eq!(hours(0.0).as_deref(), Some("0.00"));
        assert_eq!(hours(-0.0).as_deref(), Some("0.00"), "never -0.00");
        assert_eq!(hours(-1.0), None, "hours elapsed are never negative");
        assert_eq!(amount(4.317), "4.32");
        assert_eq!(amount(0.125), "0.13");
        assert_eq!(amount(-0.0), "0.00");
    }

    #[test]
    fn a_cycle_that_ends_before_it_starts_is_missing() {
        let u = Usage {
            sessions: 1,
            first_at: Some(at("2026-10-22T10:00:00Z")),
            last_at: Some(at("2026-10-22T09:59:59Z")),
            ..Default::default()
        };
        assert_eq!(render(&u, at(UPDATED)).lines[3], "Cycle time: —");
    }

    #[test]
    fn the_hash_follows_the_lines_and_ignores_the_updated_time() {
        let a = render(&golden_1(), at(UPDATED));
        let later = render(&golden_1(), at("2026-10-27T09:00:00Z"));
        assert_eq!(comment_hash(&a), comment_hash(&later), "the updated time is not a change");
        let h = comment_hash(&a);
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        // Same sums, different coverage: the comment's text changes, so must the hash.
        let fewer = Usage { tokens_reasoning: Some(0), tokens_reasoning_n: 2, ..golden_1() };
        let all = Usage { tokens_reasoning: Some(0), tokens_reasoning_n: 3, ..golden_1() };
        assert_ne!(
            comment_hash(&render(&fewer, at(UPDATED))),
            comment_hash(&render(&all, at(UPDATED)))
        );
    }

    #[test]
    fn every_failure_of_every_operation_has_a_status() {
        use TrackerError as E;
        use WritebackStatus as S;
        for op in WriteOp::ALL {
            for e in TrackerError::ALL {
                let want = match (op, e) {
                    // The issue check only tells which object a 404 meant (W5).
                    (WriteOp::IssueCheck, E::NotFound) => S::NotFound,
                    (WriteOp::IssueCheck, _) => S::Error,
                    // The comment-list read (W9).
                    (WriteOp::FindComment, E::Forbidden | E::Unauthorized) => S::Forbidden,
                    (WriteOp::FindComment, E::NotFound) => S::NotFound,
                    (WriteOp::FindComment, _) => S::Error,
                    // The writes.
                    (
                        WriteOp::CreateComment | WriteOp::UpdateComment,
                        E::Forbidden | E::Unauthorized,
                    ) => S::Forbidden,
                    (WriteOp::CreateComment | WriteOp::UpdateComment, E::RateLimited) => {
                        S::RateLimited
                    }
                    (WriteOp::CreateComment, E::NotFound) => S::NotFound,
                    // An update's 404 goes to the issue check first (W5).
                    (WriteOp::UpdateComment, E::NotFound) => S::Error,
                    (WriteOp::CreateComment | WriteOp::UpdateComment, _) => S::Error,
                    // A mapped-field write (W6, W9): a 400 is the field the tracker refused.
                    (WriteOp::WriteFields, E::BadRequest) => S::FieldInvalid,
                    (WriteOp::WriteFields, E::NotFound) => S::NotFound,
                    (WriteOp::WriteFields, E::Forbidden | E::Unauthorized) => S::Forbidden,
                    (WriteOp::WriteFields, E::RateLimited) => S::RateLimited,
                    (WriteOp::WriteFields, _) => S::Error,
                };
                assert_eq!(status_for(op, e), want, "{op:?} × {e:?}");
            }
        }
    }

    #[test]
    fn metrics_are_exactly_the_ddl_enum_labels() {
        let labels = crate::tracker::test_support::ddl_enum_labels("tracker_metric");
        let ours: Vec<&str> = TrackerMetric::ALL.iter().map(|m| m.as_str()).collect();
        assert_eq!(labels, ours);
        for m in TrackerMetric::ALL {
            assert_eq!(TrackerMetric::parse(m.as_str()), Some(m));
        }
        assert_eq!(TrackerMetric::parse("active_secs"), None);
    }

    #[test]
    fn a_field_carries_the_comments_number_as_a_number() {
        use crate::tracker::FieldValue as V;
        let u = golden_1();
        let dec = |s: &str| V::Dec(s.into());
        let want = [
            (TrackerMetric::Sessions, V::Int(3)),
            (TrackerMetric::ActiveHours, dec("2.05")),
            (TrackerMetric::ClaudeHours, dec("1.30")),
            (TrackerMetric::UserHours, dec("0.75")),
            (TrackerMetric::WallHours, dec("4.00")),
            (TrackerMetric::CycleHours, dec("48.00")),
            (TrackerMetric::TokensFresh, V::Int(120000)),
            (TrackerMetric::CacheRead, V::Int(900000)),
            (TrackerMetric::CacheWrite, V::Int(30000)),
            (TrackerMetric::TokensOut, V::Int(45000)),
            (TrackerMetric::TokensReasoning, V::Null),
            (TrackerMetric::MeteredCost, V::Null),
            (TrackerMetric::ApiCost, dec("4.32")),
        ];
        for (m, v) in want {
            assert_eq!(metric_value(m, &u), v, "{m:?}");
        }
        assert_eq!(metric_value(TrackerMetric::Sessions, &Usage::default()), V::Int(0));
        assert_eq!(metric_value(TrackerMetric::ApiCost, &Usage::default()), V::Null);
        assert_eq!(V::Dec("2.05".into()).to_json(), serde_json::json!(2.05));
        assert_eq!(V::Int(120000).to_json(), serde_json::json!(120000));
        assert_eq!(V::Null.to_json(), serde_json::Value::Null);
        assert_eq!(
            (V::Dec("2.05".into()).canonical(), V::Int(3).canonical(), V::Null.canonical()),
            ("2.05".to_string(), "3".to_string(), "null".to_string())
        );
    }

    #[test]
    fn the_fields_hash_is_the_pairs_as_written() {
        use crate::tracker::FieldValue as V;
        let pairs = |v: V| vec![("customfield_10050".to_string(), v)];
        use sha2::Digest;
        let expected = hex::encode(sha2::Sha256::digest("fields\ncustomfield_10050=2.05"));
        assert_eq!(fields_hash(&pairs(V::Dec("2.05".into()))), expected);
        assert_ne!(fields_hash(&pairs(V::Null)), fields_hash(&pairs(V::Dec("0.00".into()))));
        assert_ne!(
            fields_hash(&pairs(V::Dec("2.05".into()))),
            comment_hash(&Report {
                title: String::new(),
                lines: vec!["customfield_10050=2.05".into()]
            }),
            "a surface never matches the other"
        );
    }

    #[test]
    fn a_cycle_that_ends_a_moment_before_it_starts_is_missing() {
        let t = at("2026-10-22T10:00:00Z");
        let u = Usage {
            sessions: 1,
            first_at: Some(t),
            last_at: Some(t - chrono::Duration::microseconds(500)),
            ..Default::default()
        };
        assert_eq!(cycle_secs(&u), None);
        assert_eq!(render(&u, at(UPDATED)).lines[3], "Cycle time: —");
        assert_eq!(metric_value(TrackerMetric::CycleHours, &u), crate::tracker::FieldValue::Null);
        let same = Usage { last_at: Some(t), ..u };
        assert_eq!(cycle_secs(&same), Some(0.0));
    }

    #[test]
    fn only_the_statuses_a_retry_cannot_fix_are_sticky() {
        use WritebackStatus as S;
        let sticky: Vec<_> = S::ALL.into_iter().filter(|s| is_sticky(*s)).collect();
        assert_eq!(sticky, [S::Forbidden, S::FieldInvalid, S::CommentRemoved, S::NotFound]);
    }
}

#[cfg(test)]
mod pass_tests {
    use super::*;
    use crate::tracker::connect::connect;
    use crate::tracker::connect::tests::workspace;
    use crate::tracker::jira::{JiraAdapter, adf};
    use crate::tracker::runtime::TrackerRuntime;
    use crate::tracker::runtime::test_support::{FakeFactory, fake_runtime};
    use crate::tracker::secret::test_support::MemoryStore;
    use crate::tracker::sync::{SYNC_DEADLINE, SyncError, sync_tracker};
    use crate::tracker::test_support::{
        FakeBoard, RecordingWrites, forget_owner, repo_in, session_with,
    };
    use crate::tracker::{ApiToken, ConnectParams, TrackerLevel, WriteMethod};
    use serde_json::json;

    /// A connected, synced board with one session linked to all three tickets.
    struct Fixture {
        pg: PgStore,
        rt: Arc<TrackerRuntime>,
        store: Arc<MemoryStore>,
        factory: Arc<FakeFactory>,
        t: uuid::Uuid,
        site: String,
        repo: uuid::Uuid,
        board: FakeBoard,
    }

    fn standard(assignees: [Option<&str>; 3]) -> FakeBoard {
        FakeBoard::standard().with_assignees(assignees).with_comment_reads()
    }

    async fn fixture_with(board: FakeBoard, served: FakeBoard) -> Fixture {
        let pg = PgStore::test_store().await;
        let (rt, store, factory) = fake_runtime();
        let site = format!("https://t{}.atlassian.net", uuid::Uuid::new_v4().simple());
        let p = workspace(&pg).await;
        let params = ConnectParams {
            base_url: site.clone(),
            board_ref: "42".into(),
            account: "dev@example.com".into(),
            token: ApiToken("t".into()),
            project_key: None,
        };
        let t =
            connect(&pg, rt.secrets.clone(), &JiraAdapter, &board, p, TrackerLevel::Sprint, params)
                .await
                .unwrap()
                .tracker_id;
        let repo = repo_in(&pg, p).await;
        session_with(&pg, Some(repo), &["fix/AUTH-1", "fix/AUTH-2", "fix/AUTH-3"], &[]).await;
        sync_tracker(&pg, &rt.locks, t, &JiraAdapter, &board, SYNC_DEADLINE).await.unwrap();
        factory.serve(&site, served);
        Fixture { pg, rt, store, factory, t, site, repo, board }
    }

    async fn fixture(assignees: [Option<&str>; 3]) -> Fixture {
        fixture_with(standard(assignees), standard(assignees)).await
    }

    impl Fixture {
        fn writes(&self, w: RecordingWrites) -> Arc<RecordingWrites> {
            self.factory.serve_writes(&self.site, w)
        }
        async fn pass_within(&self, deadline: Duration) -> PassOutcome {
            let guard = self.rt.locks.try_lock(self.t).expect("the guard is free");
            let service = self.pg.tracker_by_id(self.t).await.unwrap().unwrap().secret_service;
            run(&self.pg, &PassIo::of(&self.rt), self.t, &service, &JiraAdapter, deadline, guard)
                .await
        }
        async fn pass(&self) -> PassOutcome {
            self.pass_within(WRITEBACK_DEADLINE).await
        }
        /// Link one more session to `key` and re-sync, so its usage changes.
        async fn new_session(&self, key: &str) {
            session_with(&self.pg, Some(self.repo), &[&format!("fix/{key}")], &[]).await;
            sync_tracker(
                &self.pg,
                &self.rt.locks,
                self.t,
                &JiraAdapter,
                &self.board,
                SYNC_DEADLINE,
            )
            .await
            .unwrap();
        }
        /// `(external_id, writeback_status, writeback_comment_ref, hash set)`, by external id.
        async fn state(&self) -> Vec<(String, Option<String>, Option<String>, bool)> {
            sqlx_core::query_as::query_as(
                "SELECT external_id, writeback_status::text, writeback_comment_ref, writeback_hash IS NOT NULL
                   FROM sensei.tracker_tasks WHERE tracker_id = $1 ORDER BY 1",
            )
            .bind(self.t)
            .fetch_all(self.pg.pool())
            .await
            .unwrap()
        }
        async fn status_of(&self, ext: &str) -> Option<String> {
            self.state().await.into_iter().find(|r| r.0 == ext).and_then(|r| r.1)
        }
        async fn writeback_error(&self) -> Option<String> {
            sqlx_core::query_as::query_as::<_, (Option<String>,)>(
                "SELECT writeback_error::text FROM sensei.trackers WHERE id = $1",
            )
            .bind(self.t)
            .fetch_one(self.pg.pool())
            .await
            .unwrap()
            .0
        }
        async fn sql(&self, q: &str) {
            sqlx_core::query::query(q).bind(self.t).execute(self.pg.pool()).await.unwrap();
        }
        async fn usage_of(&self, ext: &str) -> Usage {
            let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
                "SELECT id FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = $2",
            )
            .bind(self.t)
            .bind(ext)
            .fetch_one(self.pg.pool())
            .await
            .unwrap();
            crate::tracker::usage::ticket_usages(&self.pg, self.t, Some(&[id]))
                .await
                .unwrap()
                .remove(&id)
                .unwrap_or_default()
        }
    }

    fn comment_path(ext: &str) -> String {
        format!("/rest/api/3/issue/{ext}/comment")
    }

    /// The bullet texts of a sent ADF body.
    fn lines_of(body: &serde_json::Value) -> Vec<String> {
        body["body"]["content"][1]["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["content"][0]["content"][0]["text"].as_str().unwrap().to_string())
            .collect()
    }

    #[tokio::test]
    async fn only_the_owners_ticket_is_written_and_its_write_is_recorded() {
        let f = fixture([Some("A1"), Some("B2"), None]).await;
        let w = f.writes(RecordingWrites::default());
        assert_eq!(f.pass().await, PassOutcome::Done { written: 1, unchanged: 0, failed: 0 });
        let sent = w.sent();
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert_eq!(
            (sent[0].method, sent[0].path.as_str()),
            (WriteMethod::Post, comment_path("10001").as_str())
        );
        let usage = f.usage_of("10001").await;
        assert_eq!(usage.sessions, 1, "the fixture's session is linked");
        assert_eq!(lines_of(&sent[0].body), render(&usage, chrono::Utc::now()).lines);
        let title = sent[0].body["body"]["content"][0]["content"][0]["text"].as_str().unwrap();
        assert!(
            title.starts_with("Sensei · AI usage from the assignee's sessions (updated "),
            "{title}"
        );
        assert_eq!(
            f.state().await,
            vec![
                ("10001".into(), Some("ok".into()), Some("20000".into()), true),
                ("10002".into(), None, None, false),
                ("10003".into(), None, None, false),
            ]
        );
        assert_eq!(f.factory.write_retries.lock().unwrap().clone(), vec![WRITE_RETRIES]);
    }

    #[tokio::test]
    async fn an_unchanged_ticket_sends_nothing_and_a_change_edits_the_same_comment() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        assert_eq!(f.pass().await, PassOutcome::Done { written: 0, unchanged: 1, failed: 0 });
        assert_eq!(w.sent().len(), 1, "no request for an unchanged ticket");
        f.new_session("AUTH-1").await;
        f.pass().await;
        let sent = w.sent();
        assert_eq!(sent.len(), 2);
        assert_eq!(
            (sent[1].method, sent[1].path.as_str()),
            (WriteMethod::Put, "/rest/api/3/issue/10001/comment/20000")
        );
        assert_eq!(sent[1].query, [("notifyUsers".to_string(), "false".to_string())]);
        assert!(
            lines_of(&sent[1].body)[0].starts_with("Sessions: 2"),
            "{:?}",
            lines_of(&sent[1].body)
        );
    }

    #[tokio::test]
    async fn a_report_the_owner_already_posted_is_adopted_not_duplicated() {
        let earlier = json!({"comments": [{"id": "77", "author": {"accountId": "A1"},
            "body": {"type": "doc", "version": 1, "content": [{"type": "paragraph", "content": [
                {"type": "text", "text": "Sensei · AI usage from the assignee's sessions (updated 2026-10-20 08:00 UTC)"}]}]}}]});
        let served = standard([Some("A1"), None, None]).with(&comment_path("10001"), 0, earlier);
        let f = fixture_with(standard([Some("A1"), None, None]), served).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        let sent = w.sent();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            (sent[0].method, sent[0].path.as_str()),
            (WriteMethod::Put, "/rest/api/3/issue/10001/comment/77")
        );
        assert_eq!(f.state().await[0].2.as_deref(), Some("77"));
    }

    #[tokio::test]
    async fn a_ticket_that_lost_its_sessions_is_rewritten_as_empty() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE repo_folder_id = $1")
            .bind(f.repo)
            .execute(f.pg.pool())
            .await
            .unwrap();
        f.pass().await;
        let sent = w.sent();
        assert_eq!(sent.len(), 2, "the stale figures are replaced");
        assert_eq!(lines_of(&sent[1].body), render(&Usage::default(), chrono::Utc::now()).lines);
    }

    #[tokio::test]
    async fn a_deleted_comment_stops_writes_and_a_deleted_ticket_is_not_found() {
        for (issue_check, want) in
            [(None, "comment_removed"), (Some(TrackerError::NotFound), "not_found")]
        {
            let mut served = standard([Some("A1"), None, None]);
            if let Some(e) = issue_check {
                served = served.fail("/rest/api/3/issue/10001", e);
            }
            let f = fixture_with(standard([Some("A1"), None, None]), served).await;
            let w = f.writes(RecordingWrites::default().respond(
                WriteMethod::Put,
                "/rest/api/3/issue/10001/comment/20000",
                404,
                1,
            ));
            f.pass().await;
            f.new_session("AUTH-1").await;
            f.pass().await;
            assert_eq!(f.status_of("10001").await.as_deref(), Some(want));
            f.new_session("AUTH-1").await;
            f.pass().await;
            assert_eq!(w.sent().len(), 2, "{want} is sticky: nothing is posted again");
        }
    }

    #[tokio::test]
    async fn every_failed_write_leaves_its_status_and_only_sticky_ones_wait() {
        for (code, want, sticky) in [
            (401, "forbidden", true),
            (403, "forbidden", true),
            (404, "not_found", true),
            (400, "error", false),
            (500, "error", false),
            (302, "error", false),
        ] {
            let f = fixture([Some("A1"), None, None]).await;
            let w = f.writes(RecordingWrites::default().respond(
                WriteMethod::Post,
                &comment_path("10001"),
                code,
                1,
            ));
            assert_eq!(
                f.pass().await,
                PassOutcome::Done { written: 0, unchanged: 0, failed: 1 },
                "{code}"
            );
            assert_eq!(f.status_of("10001").await.as_deref(), Some(want), "{code}");
            assert!(!f.state().await[0].3, "{code}: the hash is not stored on a failure");
            f.pass().await;
            assert_eq!(w.sent().len(), if sticky { 1 } else { 2 }, "{code}");
        }
    }

    #[tokio::test]
    async fn a_failed_comment_list_read_leaves_its_status_and_posts_nothing() {
        for (e, want) in
            [(TrackerError::Forbidden, "forbidden"), (TrackerError::HttpError, "error")]
        {
            let served = standard([Some("A1"), None, None]).fail(&comment_path("10001"), e);
            let f = fixture_with(standard([Some("A1"), None, None]), served).await;
            let w = f.writes(RecordingWrites::default());
            f.pass().await;
            assert_eq!(f.status_of("10001").await.as_deref(), Some(want));
            assert!(w.sent().is_empty(), "never post without first looking for an earlier report");
        }
    }

    #[tokio::test]
    async fn a_429_spends_the_write_budget_then_rate_limits() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default().respond_after(
            WriteMethod::Post,
            &comment_path("10001"),
            429,
            Some("0"),
            4,
        ));
        f.pass().await;
        assert_eq!(w.sent().len(), 4, "one try plus WRITE_RETRIES");
        assert_eq!(f.status_of("10001").await.as_deref(), Some("rate_limited"));
    }

    #[tokio::test]
    async fn five_rate_limited_tickets_in_a_row_end_the_pass() {
        use crate::tracker::test_support::assigned_issue;
        let issues: Vec<_> = (1..=6)
            .map(|n| assigned_issue(&format!("1000{n}"), &format!("AUTH-{n}"), Some("A1")))
            .collect();
        let mut board = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/issue",
            0,
            json!({"startAt": 0, "maxResults": 6, "total": 6, "issues": issues}),
        );
        for n in 1..=6 {
            board = board.with(&comment_path(&format!("1000{n}")), 0, json!({"comments": []}));
        }
        let served = FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/issue",
            0,
            json!({"startAt": 0, "maxResults": 6, "total": 6, "issues": []}),
        );
        let served = (1..=6).fold(served, |b, n| {
            b.with(&comment_path(&format!("1000{n}")), 0, json!({"comments": []}))
        });
        let f = fixture_with(board, served).await;
        for n in 4..=6 {
            session_with(&f.pg, Some(f.repo), &[&format!("fix/AUTH-{n}")], &[]).await;
        }
        sync_tracker(&f.pg, &f.rt.locks, f.t, &JiraAdapter, &f.board, SYNC_DEADLINE).await.unwrap();
        let w = (1..=6).fold(RecordingWrites::default(), |w, n| {
            w.respond_after(
                WriteMethod::Post,
                &comment_path(&format!("1000{n}")),
                429,
                Some("0"),
                4,
            )
        });
        let w = f.writes(w);
        assert_eq!(f.pass().await, PassOutcome::Stopped("rate_limited"));
        assert_eq!(w.sent().len(), 5 * 4, "the sixth ticket is never tried");
        assert_eq!(f.writeback_error().await.as_deref(), Some("rate_limited"));
        let untouched = f.state().await.into_iter().filter(|r| r.1.is_none()).count();
        assert_eq!(untouched, 1);
    }

    #[tokio::test]
    async fn a_failed_write_never_touches_the_sync_and_the_others_are_written() {
        let f = fixture([Some("A1"), Some("A1"), None]).await;
        let w = f.writes(RecordingWrites::default().respond(
            WriteMethod::Post,
            &comment_path("10001"),
            403,
            1,
        ));
        f.pass().await;
        assert_eq!(f.status_of("10001").await.as_deref(), Some("forbidden"));
        assert_eq!(f.status_of("10002").await.as_deref(), Some("ok"));
        assert_eq!(w.count(WriteMethod::Post), 2);
        let row = f.pg.tracker_by_id(f.t).await.unwrap().unwrap();
        assert_eq!(row.last_error, None);
        assert_eq!(f.writeback_error().await, None);
    }

    /// A hook that runs `sql` (bound to the tracker id) once, on the first send.
    fn once(pg: &PgStore, t: uuid::Uuid, sql: &'static str) -> crate::tracker::test_support::Hook {
        let pool = pg.pool().clone();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Box::new(move |_path| {
            let (pool, done) = (pool.clone(), done.clone());
            Box::pin(async move {
                if !done.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    for q in sql.split(';').filter(|q| !q.trim().is_empty()) {
                        sqlx_core::query::query(q).bind(t).execute(&pool).await.unwrap();
                    }
                }
            })
        })
    }

    #[tokio::test]
    async fn switching_off_or_disconnecting_mid_pass_stops_the_next_write() {
        for sql in [
            "UPDATE sensei.trackers SET writeback_enabled = false WHERE id = $1",
            "DELETE FROM sensei.trackers WHERE id = $1",
        ] {
            let f = fixture([Some("A1"), Some("A1"), Some("A1")]).await;
            let w = f.writes(RecordingWrites::default().on_send(once(&f.pg, f.t, sql)));
            f.pass().await;
            assert_eq!(w.sent().len(), 1, "{sql}");
        }
    }

    #[tokio::test]
    async fn a_reconnect_during_a_write_leaves_nothing_of_the_old_token() {
        let f = fixture([Some("A1"), Some("A1"), None]).await;
        let w = f.writes(RecordingWrites::default().on_send(once(
            &f.pg,
            f.t,
            "UPDATE sensei.trackers SET secret_service = secret_service || '.new', writeback_error = NULL WHERE id = $1;
             UPDATE sensei.tracker_tasks SET writeback_comment_ref = NULL, writeback_hash = NULL,
                    writeback_status = NULL, assigned_to_me = NULL WHERE tracker_id = $1",
        )));
        assert_eq!(f.pass().await, PassOutcome::Superseded);
        assert_eq!(w.sent().len(), 1);
        assert!(
            f.state().await.iter().all(|r| r.1.is_none() && r.2.is_none() && !r.3),
            "{:?}",
            f.state().await
        );
        assert_eq!(f.writeback_error().await, None);
    }

    #[tokio::test]
    async fn a_write_that_cannot_be_recorded_stops_the_pass_and_says_so() {
        let f = fixture([Some("A1"), Some("A1"), None]).await;
        f.sql(
            "CREATE FUNCTION pg_temp_refuse() RETURNS trigger LANGUAGE plpgsql AS
                 $$ BEGIN RAISE EXCEPTION 'record refused'; END $$",
        )
        .await;
        f.sql(
            "CREATE TRIGGER refuse BEFORE UPDATE OF writeback_hash ON sensei.tracker_tasks
                 FOR EACH ROW EXECUTE FUNCTION pg_temp_refuse()",
        )
        .await;
        let w = f.writes(RecordingWrites::default());
        assert_eq!(f.pass().await, PassOutcome::Stopped("db"));
        assert_eq!(w.sent().len(), 1, "no second write after the first could not be recorded");
        assert_eq!(f.writeback_error().await.as_deref(), Some("db"));
    }

    #[tokio::test]
    async fn the_deadline_fails_the_write_in_flight_and_the_rest_wait_their_turn() {
        let f = fixture([Some("A1"), Some("A1"), Some("A1")]).await;
        let w = f.writes(RecordingWrites::default().delay(Duration::from_secs(3)));
        assert_eq!(
            f.pass_within(Duration::from_millis(500)).await,
            PassOutcome::Stopped("transport")
        );
        assert_eq!(w.sent().len(), 1);
        let first = w.sent()[0].path.clone();
        let state = f.state().await;
        let failed: Vec<_> = state.iter().filter(|r| r.1.as_deref() == Some("error")).collect();
        assert_eq!(failed.len(), 1, "the write in flight");
        assert!(!failed[0].3, "a write that may not have landed records no hash");
        assert_eq!(
            state.iter().filter(|r| r.1.is_none()).count(),
            2,
            "the others keep their state"
        );
        assert_eq!(f.writeback_error().await.as_deref(), Some("transport"));
        let next = f.writes(RecordingWrites::default());
        f.pass().await;
        assert_ne!(next.sent()[0].path, first, "never-attempted tickets go first");
        assert_eq!(f.writeback_error().await, None, "a pass that runs clears it");
    }

    #[tokio::test]
    async fn a_busy_guard_means_no_pass_this_round() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        let _held = f.rt.locks.try_lock(f.t).unwrap();
        assert!(spawn_after_sync(&f.pg, &f.rt, f.t, "any", &JiraAdapter).is_none());
        assert!(w.sent().is_empty());
    }

    #[tokio::test]
    async fn the_card_counts_only_live_tickets_assigned_to_the_owner() {
        let f = fixture([Some("A1"), Some("A1"), Some("B2")]).await;
        f.sql("UPDATE sensei.tracker_tasks SET writeback_status = 'ok' WHERE tracker_id = $1 AND external_id = '10001'").await;
        f.sql("UPDATE sensei.tracker_tasks SET writeback_status = 'forbidden' WHERE tracker_id = $1 AND external_id = '10003'").await;
        f.sql("INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, assigned_to_me, writeback_status, removed_at)
               VALUES ($1, '10009', 'AUTH-9', true, 'error', now())")
            .await;
        assert_eq!(f.pg.writeback_counts(f.t).await, Ok((2, 0)));
        f.sql("UPDATE sensei.tracker_tasks SET writeback_status = 'error' WHERE tracker_id = $1 AND external_id = '10001'").await;
        assert_eq!(f.pg.writeback_counts(f.t).await, Ok((2, 1)));
    }

    #[tokio::test]
    async fn a_credential_the_pass_cannot_read_is_recorded_apart_and_cleared_later() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        *f.store.fail_get.lock().unwrap() = true;
        assert_eq!(f.pass().await, PassOutcome::Stopped("credential_store"));
        assert!(w.sent().is_empty());
        assert_eq!(f.writeback_error().await.as_deref(), Some("credential_store"));
        assert_eq!(f.pg.tracker_by_id(f.t).await.unwrap().unwrap().last_error, None);
        *f.store.fail_get.lock().unwrap() = false;
        f.pass().await;
        assert_eq!(w.sent().len(), 1);
        assert_eq!(f.writeback_error().await, None);
    }

    #[tokio::test]
    async fn no_pass_without_an_owner_or_with_write_back_off() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        forget_owner(&f.pg, f.t).await;
        assert_eq!(f.pass().await, PassOutcome::Skipped);
        f.sql(
            "UPDATE sensei.trackers SET owner_ref = 'A1', writeback_enabled = false WHERE id = $1",
        )
        .await;
        assert_eq!(f.pass().await, PassOutcome::Skipped);
        assert!(w.sent().is_empty());
    }

    #[tokio::test]
    async fn the_sync_never_waits_for_its_pass_and_holds_the_guard_through_it() {
        let f = fixture([Some("A1"), None, None]).await;
        f.factory.serve(&f.site, standard([Some("A1"), None, None]));
        let w = f.writes(RecordingWrites::default().delay(Duration::from_secs(3)));
        let row = f.pg.tracker_by_id(f.t).await.unwrap().unwrap();
        let started = std::time::Instant::now();
        assert!(crate::tracker::runtime::sync_stored(&f.pg, &f.rt, &row).await.is_ok());
        assert!(started.elapsed() < Duration::from_secs(1), "took {:?}", started.elapsed());
        let row = f.pg.tracker_by_id(f.t).await.unwrap().unwrap();
        assert_eq!(
            crate::tracker::runtime::sync_stored(&f.pg, &f.rt, &row).await,
            Err(SyncError::Busy),
            "the running pass holds the tracker's guard"
        );
        for _ in 0..50 {
            if w.count(WriteMethod::Post) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(w.count(WriteMethod::Post), 1);
    }

    #[tokio::test]
    async fn an_owner_found_by_a_sync_is_written_by_that_syncs_pass() {
        let f = fixture([Some("A1"), None, None]).await;
        forget_owner(&f.pg, f.t).await;
        let w = f.writes(RecordingWrites::default());
        let row = f.pg.tracker_by_id(f.t).await.unwrap().unwrap();
        crate::tracker::runtime::sync_stored(&f.pg, &f.rt, &row).await.unwrap();
        for _ in 0..50 {
            if w.count(WriteMethod::Post) == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(w.count(WriteMethod::Post), 1);
    }

    #[tokio::test]
    async fn write_back_is_on_unless_switched_off_and_no_owner_id_reaches_a_debug_print() {
        let f = fixture([Some("A1"), None, None]).await;
        let (on,): (bool,) = sqlx_core::query_as::query_as(
            "SELECT writeback_enabled FROM sensei.trackers WHERE id = $1",
        )
        .bind(f.t)
        .fetch_one(f.pg.pool())
        .await
        .unwrap();
        assert!(on, "on by default (W8)");
        let row = f.pg.tracker_by_id(f.t).await.unwrap().unwrap();
        assert!(
            row.owner_ref.is_some(),
            "the owner is on record, so the check below means something"
        );
        assert!(!format!("{row:?}").contains("A1"), "{row:?}");
    }

    #[tokio::test]
    async fn switching_write_back_off_and_on_posts_a_new_comment_where_one_was_deleted() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default().respond(
            WriteMethod::Put,
            "/rest/api/3/issue/10001/comment/20000",
            404,
            1,
        ));
        f.pass().await;
        f.new_session("AUTH-1").await;
        f.pass().await;
        assert_eq!(f.status_of("10001").await.as_deref(), Some("comment_removed"));
        assert_eq!(f.pg.set_writeback_enabled(f.t, false).await, Ok(true));
        assert_eq!(f.pg.set_writeback_enabled(f.t, true).await, Ok(true));
        assert_eq!(
            f.state().await[0],
            ("10001".into(), None, None, false),
            "the removed comment is forgotten"
        );
        f.pass().await;
        let posts: Vec<_> =
            w.sent().into_iter().filter(|s| s.method == WriteMethod::Post).collect();
        assert_eq!(posts.len(), 2, "a fresh comment, after the one the user deleted");
        assert_eq!(f.status_of("10001").await.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn switching_write_back_on_clears_only_what_was_waiting_for_it() {
        let f = fixture([Some("A1"), Some("A1"), Some("A1")]).await;
        f.sql(
            "UPDATE sensei.tracker_tasks SET writeback_comment_ref = '9', writeback_hash = 'h',
                    writeback_status = CASE external_id WHEN '10001' THEN 'forbidden'::sensei.tracker_writeback_status
                                                       WHEN '10002' THEN 'ok'
                                                       ELSE 'error' END
              WHERE tracker_id = $1",
        )
        .await;
        f.sql(
            "INSERT INTO sensei.tracker_tasks (tracker_id, external_id, key, assigned_to_me,
                    writeback_comment_ref, writeback_hash, writeback_status)
             VALUES ($1, '10007', 'AUTH-7', true, '9', 'h', 'not_found'),
                    ($1, '10008', 'AUTH-8', true, '9', 'h', 'field_invalid')",
        )
        .await;
        // Switching on while already on changes nothing.
        f.pg.set_writeback_enabled(f.t, true).await.unwrap();
        assert_eq!(f.state().await[0].1.as_deref(), Some("forbidden"), "it was already on");
        f.pg.set_writeback_enabled(f.t, false).await.unwrap();
        assert_eq!(
            f.state().await[0].1.as_deref(),
            Some("forbidden"),
            "switching off clears nothing"
        );
        f.pg.set_writeback_enabled(f.t, true).await.unwrap();
        assert_eq!(
            f.state().await,
            vec![
                ("10001".into(), None, Some("9".into()), true),
                ("10002".into(), Some("ok".into()), Some("9".into()), true),
                ("10003".into(), Some("error".into()), Some("9".into()), true),
                ("10007".into(), None, Some("9".into()), true),
                ("10008".into(), None, Some("9".into()), true),
            ],
            "every sticky status is cleared; ok and a retryable error stay"
        );
        f.pg.set_writeback_enabled(f.t, true).await.unwrap();
        assert_eq!(f.pg.set_writeback_enabled(uuid::Uuid::new_v4(), true).await, Ok(false));
    }

    // ── Mapped fields (W6, W7) ─────────────────────────────────────────────

    impl Fixture {
        async fn map(&self, mapping: &[(&str, &str)]) {
            let rows: Vec<(String, String)> =
                mapping.iter().map(|(m, r)| (m.to_string(), r.to_string())).collect();
            self.pg.replace_tracker_fields(self.t, &rows).await.unwrap();
        }
        /// Give the fixture's sessions 7380 s of active time (2.05 h).
        async fn active(&self) {
            sqlx_core::query::query(
                "UPDATE activity.sessions SET duration = interval '7380 seconds' WHERE repo_folder_id = $1",
            )
            .bind(self.repo)
            .execute(self.pg.pool())
            .await
            .unwrap();
        }
    }

    const ISSUE_1: &str = "/rest/api/3/issue/10001";

    #[tokio::test]
    async fn a_mapped_field_gets_the_number_in_one_request_and_no_comment() {
        let f = fixture([Some("A1"), None, None]).await;
        f.active().await;
        f.map(&[("active_hours", "customfield_10050")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        let sent = w.sent();
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert_eq!((sent[0].method, sent[0].path.as_str()), (WriteMethod::Put, ISSUE_1));
        assert_eq!(sent[0].body, json!({"fields": {"customfield_10050": 2.05}}));
        assert_eq!(f.status_of("10001").await.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn a_change_to_an_unmapped_metric_sends_nothing() {
        let f = fixture([Some("A1"), None, None]).await;
        f.map(&[("metered_cost", "customfield_10051")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        assert_eq!(
            w.sent()[0].body,
            json!({"fields": {"customfield_10051": null}}),
            "no value is null, never 0"
        );
        f.new_session("AUTH-1").await;
        f.pass().await;
        assert_eq!(w.sent().len(), 1, "sessions changed, the mapped metric did not");
    }

    #[tokio::test]
    async fn a_ticket_that_lost_its_sessions_gets_null_in_its_fields() {
        let f = fixture([Some("A1"), None, None]).await;
        f.active().await;
        f.map(&[("active_hours", "customfield_10050"), ("sessions", "customfield_10052")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE repo_folder_id = $1")
            .bind(f.repo)
            .execute(f.pg.pool())
            .await
            .unwrap();
        f.pass().await;
        assert_eq!(
            w.sent()[1].body,
            json!({"fields": {"customfield_10052": 0, "customfield_10050": null}}),
            "fields follow the metric enum order"
        );
    }

    #[tokio::test]
    async fn a_refused_field_waits_for_the_mapping_to_change() {
        let f = fixture([Some("A1"), None, None]).await;
        f.map(&[("sessions", "customfield_10052")]).await;
        let w = f.writes(RecordingWrites::default().respond(WriteMethod::Put, ISSUE_1, 400, 1));
        f.pass().await;
        assert_eq!(f.status_of("10001").await.as_deref(), Some("field_invalid"));
        f.new_session("AUTH-1").await;
        f.pass().await;
        assert_eq!(w.sent().len(), 1, "field_invalid is sticky");
        f.map(&[("sessions", "customfield_10053")]).await;
        f.pass().await;
        assert_eq!(f.status_of("10001").await.as_deref(), Some("ok"), "a mapping change clears it");
        assert_eq!(w.sent().len(), 2);
    }

    #[tokio::test]
    async fn a_field_write_to_a_ticket_that_is_gone_is_not_found() {
        let f = fixture([Some("A1"), None, None]).await;
        f.map(&[("sessions", "customfield_10052")]).await;
        f.writes(RecordingWrites::default().respond(WriteMethod::Put, ISSUE_1, 404, 1));
        f.pass().await;
        assert_eq!(f.status_of("10001").await.as_deref(), Some("not_found"));
    }

    #[tokio::test]
    async fn a_deleted_comment_does_not_hold_back_the_fields() {
        let f = fixture([Some("A1"), None, None]).await;
        f.sql("UPDATE sensei.tracker_tasks SET writeback_status = 'comment_removed' WHERE tracker_id = $1").await;
        f.map(&[("sessions", "customfield_10052")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        assert_eq!(w.sent().len(), 1, "comment_removed is sticky on the comment surface only");
        assert_eq!(w.sent()[0].path, ISSUE_1);
    }

    #[tokio::test]
    async fn an_unmapped_field_is_cleared_once_then_left_alone() {
        let f = fixture([Some("A1"), None, None]).await;
        f.active().await;
        f.map(&[("active_hours", "customfield_10050")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        f.map(&[]).await;
        f.pass().await;
        let sent = w.sent();
        let methods: Vec<_> = sent.iter().map(|s| (s.method, s.path.as_str())).collect();
        assert_eq!(
            methods,
            [
                (WriteMethod::Put, ISSUE_1),
                (WriteMethod::Post, "/rest/api/3/issue/10001/comment"),
                (WriteMethod::Put, ISSUE_1),
            ],
            "the comment, then the retired field cleared"
        );
        assert_eq!(sent[2].body, json!({"fields": {"customfield_10050": null}}));
        f.pass().await;
        assert_eq!(w.sent().len(), 3, "cleared once; nothing changed since");
    }

    #[tokio::test]
    async fn a_remapped_metric_moves_and_its_old_field_is_cleared_in_the_same_request() {
        let f = fixture([Some("A1"), None, None]).await;
        f.active().await;
        f.map(&[("active_hours", "customfield_10050")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        f.map(&[("active_hours", "customfield_10051")]).await;
        f.pass().await;
        assert_eq!(w.sent().len(), 2);
        assert_eq!(
            w.sent()[1].body,
            json!({"fields": {"customfield_10051": 2.05, "customfield_10050": null}})
        );
        f.pass().await;
        assert_eq!(w.sent().len(), 2);
    }

    #[tokio::test]
    async fn switching_back_to_the_comment_rewrites_a_ticket_that_lost_its_sessions() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        f.map(&[("sessions", "customfield_10052")]).await;
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE repo_folder_id = $1")
            .bind(f.repo)
            .execute(f.pg.pool())
            .await
            .unwrap();
        f.pass().await;
        f.map(&[]).await;
        f.pass().await;
        let comment_puts: Vec<_> = w
            .sent()
            .into_iter()
            .filter(|s| s.path == "/rest/api/3/issue/10001/comment/20000")
            .collect();
        assert_eq!(comment_puts.len(), 1, "{:?}", w.sent());
        assert_eq!(lines_of(&comment_puts[0].body)[0], "Sessions: 0");
    }

    #[tokio::test]
    async fn a_malformed_ticket_id_never_reaches_a_field_write() {
        let f = fixture([Some("A1"), None, None]).await;
        f.sql("UPDATE sensei.tracker_tasks SET external_id = 'abc' WHERE tracker_id = $1 AND external_id = '10001'").await;
        f.map(&[("sessions", "customfield_10052")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        assert!(w.sent().is_empty());
        assert_eq!(
            f.status_of("abc").await.as_deref(),
            Some("error"),
            "not field_invalid: no field was refused"
        );
    }

    #[tokio::test]
    async fn a_retired_field_is_cleared_on_tickets_tried_before_it_was_retired_only() {
        let f = fixture([Some("A1"), Some("A1"), None]).await;
        f.map(&[("sessions", "customfield_10052")]).await;
        f.writes(RecordingWrites::default());
        // 10001 is written; 10002's write fails, so it was tried but never recorded ok.
        f.writes(RecordingWrites::default().respond(
            WriteMethod::Put,
            "/rest/api/3/issue/10002",
            500,
            1,
        ));
        f.pass().await;
        assert_eq!(f.status_of("10002").await.as_deref(), Some("error"));
        // A ticket never tried at all: 10003, assigned now.
        f.sql("UPDATE sensei.tracker_tasks SET assigned_to_me = true WHERE tracker_id = $1 AND external_id = '10003'").await;
        f.map(&[]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        let clears: Vec<_> = w
            .sent()
            .into_iter()
            .filter(|s| s.body == json!({"fields": {"customfield_10052": null}}))
            .map(|s| s.path)
            .collect();
        assert!(
            clears.contains(&"/rest/api/3/issue/10001".to_string()),
            "written before: {clears:?}"
        );
        assert!(
            clears.contains(&"/rest/api/3/issue/10002".to_string()),
            "tried before, may have landed: {clears:?}"
        );
        assert!(!clears.iter().any(|p| p.contains("10003")), "never tried: nothing to clear");
    }

    #[tokio::test]
    async fn a_retired_field_the_tracker_refuses_to_clear_is_dropped_not_a_poison() {
        let f = fixture([Some("A1"), None, None]).await;
        f.map(&[("sessions", "customfield_10052")]).await;
        f.writes(RecordingWrites::default());
        f.pass().await;
        f.map(&[]).await;
        // The comment lands; clearing the retired field is refused (deleted in Jira).
        let w = f.writes(RecordingWrites::default().respond(WriteMethod::Put, ISSUE_1, 400, 1));
        f.pass().await;
        assert_eq!(f.status_of("10001").await.as_deref(), Some("ok"), "the comment landed");
        assert!(f.state().await[0].2.is_some(), "and its id is kept");
        assert!(
            f.pg.tracker_fields(f.t).await.unwrap().is_empty(),
            "the unclearable field is dropped"
        );
        f.new_session("AUTH-1").await;
        f.pass().await;
        assert_eq!(w.count(WriteMethod::Put), 2, "the comment update only; no more clears");
    }

    #[tokio::test]
    async fn a_refused_clear_does_not_hold_back_the_mapped_fields() {
        let f = fixture([Some("A1"), None, None]).await;
        f.map(&[("sessions", "customfield_10052")]).await;
        f.writes(RecordingWrites::default());
        f.pass().await;
        f.map(&[("sessions", "customfield_10053")]).await;
        // The combined write (10053 value + 10052 null) is refused; alone, 10053 is fine.
        let w = f.writes(RecordingWrites::default().respond(WriteMethod::Put, ISSUE_1, 400, 1));
        f.pass().await;
        assert_eq!(w.sent().len(), 2);
        assert_eq!(w.sent()[1].body, json!({"fields": {"customfield_10053": 1}}));
        assert_eq!(f.status_of("10001").await.as_deref(), Some("ok"));
        assert_eq!(
            f.pg.tracker_fields(f.t)
                .await
                .unwrap()
                .into_iter()
                .map(|r| r.field_ref)
                .collect::<Vec<_>>(),
            ["customfield_10053"],
            "the retired field it could not clear is dropped"
        );
    }

    #[tokio::test]
    async fn saving_the_same_mapping_again_rewrites_every_ticket() {
        let f = fixture([Some("A1"), None, None]).await;
        f.map(&[("sessions", "customfield_10052")]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        f.map(&[("sessions", "customfield_10052")]).await;
        f.pass().await;
        assert_eq!(
            w.sent().len(),
            2,
            "a saved mapping clears every hash, so each ticket is written again"
        );
        assert_eq!(
            w.sent()[1].body,
            json!({"fields": {"customfield_10052": 1}}),
            "and nothing is cleared"
        );
    }

    /// Give the tracker's tickets an explicit pass order (oldest attempt first).
    async fn order(f: &Fixture, exts: &[&str]) {
        for (i, ext) in exts.iter().enumerate() {
            sqlx_core::query::query(
                "UPDATE sensei.tracker_tasks SET writeback_tried_at = now() - make_interval(hours => 100 - $3)
                  WHERE tracker_id = $1 AND external_id = $2",
            )
            .bind(f.t)
            .bind(*ext)
            .bind(i as i32)
            .execute(f.pg.pool())
            .await
            .unwrap();
        }
    }

    #[tokio::test]
    async fn a_ticket_that_failed_goes_behind_the_ones_not_yet_tried() {
        let f = fixture([Some("A1"), Some("A1"), Some("A1")]).await;
        f.writes(RecordingWrites::default());
        f.pass().await;
        // All three changed; 10001 was tried longest ago.
        session_with(&f.pg, Some(f.repo), &["fix/AUTH-1", "fix/AUTH-2", "fix/AUTH-3"], &[]).await;
        sync_tracker(&f.pg, &f.rt.locks, f.t, &JiraAdapter, &f.board, SYNC_DEADLINE).await.unwrap();
        order(&f, &["10001", "10002", "10003"]).await;
        let slow = f.writes(RecordingWrites::default().delay(Duration::from_secs(3)));
        f.pass_within(Duration::from_millis(500)).await;
        assert!(
            slow.sent()[0].path.starts_with("/rest/api/3/issue/10001/comment/"),
            "{:?}",
            slow.sent()
        );
        let next = f.writes(RecordingWrites::default());
        f.pass().await;
        assert!(
            next.sent()[0].path.starts_with("/rest/api/3/issue/10002/comment/"),
            "the ticket that just failed waits behind the others: {:?}",
            next.sent()
        );
    }

    #[tokio::test]
    async fn a_ticket_whose_last_write_failed_is_written_even_if_its_hash_matches() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        f.sql("UPDATE sensei.tracker_tasks SET writeback_status = 'error' WHERE tracker_id = $1 AND external_id = '10001'")
            .await;
        f.pass().await;
        assert_eq!(
            w.count(WriteMethod::Put),
            1,
            "a failed write is retried, whatever the hash says"
        );
        assert_eq!(f.status_of("10001").await.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn a_tracker_the_pass_cannot_read_is_recorded_as_db() {
        let f = fixture([Some("A1"), None, None]).await;
        let w = f.writes(RecordingWrites::default());
        let service = f.pg.tracker_by_id(f.t).await.unwrap().unwrap().secret_service;
        f.sql(
            "ALTER TABLE sensei.trackers RENAME COLUMN writeback_enabled TO writeback_enabled_gone",
        )
        .await;
        let guard = f.rt.locks.try_lock(f.t).unwrap();
        let outcome =
            run(&f.pg, &PassIo::of(&f.rt), f.t, &service, &JiraAdapter, WRITEBACK_DEADLINE, guard)
                .await;
        assert_eq!(outcome, PassOutcome::Stopped("db"));
        assert_eq!(f.writeback_error().await.as_deref(), Some("db"));
        assert!(w.sent().is_empty());
    }

    #[tokio::test]
    async fn a_reconnect_during_a_failed_write_records_nothing() {
        let f = fixture([Some("A1"), Some("A1"), None]).await;
        let w = f.writes(
            RecordingWrites::default()
                .respond(WriteMethod::Post, &comment_path("10001"), 403, 1)
                .respond(WriteMethod::Post, &comment_path("10002"), 403, 1)
                .on_send(once(
                    &f.pg,
                    f.t,
                    "UPDATE sensei.trackers SET secret_service = secret_service || '.new' WHERE id = $1;
                     UPDATE sensei.tracker_tasks SET writeback_status = NULL WHERE tracker_id = $1",
                )),
        );
        assert_eq!(f.pass().await, PassOutcome::Superseded);
        assert_eq!(w.sent().len(), 1);
        assert!(f.state().await.iter().all(|r| r.1.is_none()), "{:?}", f.state().await);
    }

    #[tokio::test]
    async fn a_ticket_with_no_sessions_and_nothing_written_is_left_alone() {
        let f = fixture([Some("A1"), Some("A1"), Some("A1")]).await;
        f.sql(
            "DELETE FROM sensei.session_task_links WHERE task_id IN
               (SELECT id FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = '10002')",
        )
        .await;
        let w = f.writes(RecordingWrites::default());
        f.pass().await;
        assert_eq!(w.sent().len(), 2, "{:?}", w.sent());
        assert!(w.sent().iter().all(|s| !s.path.contains("10002")));
    }

    #[tokio::test]
    async fn the_rate_limit_count_is_of_tickets_in_a_row() {
        use crate::tracker::test_support::assigned_issue;
        let issues: Vec<_> = (1..=6)
            .map(|n| assigned_issue(&format!("1000{n}"), &format!("AUTH-{n}"), Some("A1")))
            .collect();
        let reads = |b: FakeBoard| {
            (1..=6).fold(b, |b, n| {
                b.with(&comment_path(&format!("1000{n}")), 0, json!({"comments": []}))
            })
        };
        let board = reads(FakeBoard::standard().with(
            "/rest/agile/1.0/board/42/issue",
            0,
            json!({"startAt": 0, "maxResults": 6, "total": 6, "issues": issues}),
        ));
        let f = fixture_with(board, reads(FakeBoard::standard())).await;
        for n in 4..=6 {
            session_with(&f.pg, Some(f.repo), &[&format!("fix/AUTH-{n}")], &[]).await;
        }
        sync_tracker(&f.pg, &f.rt.locks, f.t, &JiraAdapter, &f.board, SYNC_DEADLINE).await.unwrap();
        order(&f, &["10001", "10002", "10003", "10004", "10005", "10006"]).await;
        // 429, 429, 429, 429, ok, 429: five rate-limited tickets, never five in a row.
        let w = [1, 2, 3, 4, 6].iter().fold(RecordingWrites::default(), |w, n| {
            w.respond_after(
                WriteMethod::Post,
                &comment_path(&format!("1000{n}")),
                429,
                Some("0"),
                4,
            )
        });
        f.writes(w);
        assert_eq!(f.pass().await, PassOutcome::Done { written: 1, unchanged: 0, failed: 5 });
        assert_eq!(f.writeback_error().await, None);
    }

    #[tokio::test]
    async fn a_ticket_reassigned_mid_pass_is_skipped_and_the_pass_goes_on() {
        let f = fixture([Some("A1"), Some("A1"), Some("A1")]).await;
        order(&f, &["10001", "10002", "10003"]).await;
        let w = f.writes(RecordingWrites::default().on_send(once(
            &f.pg,
            f.t,
            "UPDATE sensei.tracker_tasks SET assigned_to_me = false WHERE tracker_id = $1 AND external_id = '10002'",
        )));
        assert_eq!(f.pass().await, PassOutcome::Done { written: 2, unchanged: 0, failed: 0 });
        let paths: Vec<_> = w.sent().into_iter().map(|s| s.path).collect();
        assert_eq!(paths, [comment_path("10001"), comment_path("10003")]);
    }

    #[tokio::test]
    async fn every_pass_record_is_bound_to_the_token_it_ran_on() {
        let f = fixture([Some("A1"), None, None]).await;
        let (task,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "SELECT id FROM sensei.tracker_tasks WHERE tracker_id = $1 AND external_id = '10001'",
        )
        .bind(f.t)
        .fetch_one(f.pg.pool())
        .await
        .unwrap();
        let good = f.pg.tracker_by_id(f.t).await.unwrap().unwrap().secret_service;
        let stale = format!("{good}.stale");
        assert_eq!(f.pg.record_write_ok(task, &stale, "h", Some("1")).await, Ok(false));
        assert_eq!(f.pg.record_write_status(task, &stale, "error").await, Ok(false));
        assert_eq!(f.pg.set_writeback_error(f.t, &stale, Some("db")).await, Ok(false));
        assert_eq!(f.pg.writeback_target(task, &stale).await, Ok(WriteTarget::Stop));
        assert_eq!(f.state().await[0], ("10001".into(), None, None, false));
        assert_eq!(f.writeback_error().await, None);
        assert_eq!(f.pg.writeback_target(task, &good).await, Ok(WriteTarget::Go));
        f.sql("UPDATE sensei.trackers SET writeback_enabled = false WHERE id = $1").await;
        assert_eq!(f.pg.writeback_target(task, &good).await, Ok(WriteTarget::Stop));
    }

    #[test]
    fn the_adapter_and_the_pass_share_one_report_format() {
        // The bullets the pass sends are the rendered lines, through the adapter's document.
        let r = render(&Usage::default(), chrono::Utc::now());
        assert_eq!(lines_of(&json!({"body": adf(&r)})), r.lines);
    }
}
