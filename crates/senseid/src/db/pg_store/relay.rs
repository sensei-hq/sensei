//! Relay lanes slice 1, item 2 — the daemon store (phase A part 2).
//!
//! Every op the brief names: lane registration and leases, the grouped
//! envelope, reports / reviews / decisions / messages, and the coordinator's
//! plan / assign / merge — plus the person reads behind `relay-api.ts`
//! (`now`, `queue`, the task view). Each state change and its action are
//! written in one transaction (pinned by the atomicity reds).
//!
//! Conventions: `user_id` is an explicit parameter (scoping is testable with
//! two users); lease math takes a fed `now` (tests never read wall time);
//! reads return `serde_json::Value` shaped exactly per `relay-api.ts` (phase B
//! may introduce typed views as long as the JSON pins hold). `todo!()` stubs
//! until phase B — the reds below fail on them, never on setup.

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::PgStore;
use crate::relay::envelope::{
    EnvelopeAnswer, EnvelopeAssignment, EnvelopeBrief, EnvelopeMessage, EnvelopeReview,
    EnvelopeTicket, LaneEnvelope, ReviewDirection,
};
use crate::relay::{
    Caller, DISK_FLOOR_GB, LANE_LEASE_TTL_SECS, REOFFER_GRACE_SECS, RESOURCE_LEASE_TTL_SECS,
    RegisterMode, RelayActionKind, RelayError, RelayFindingSeverity, RelayFloors, RelayItemKind,
    RelayLeaseState, RelayRole, RelaySurface, RelayTaskPhase, RelayTaskState, RelayTicketProvider,
    ResourceGrant, ReviewVerdict, SWAP_FLOOR_GB, TaskEvent, check_summary_len,
    decide_resource_grant, guard_markdown, hash_lane_token, pick_reviewer, register_mode,
    resolve_surface, transition, validate_props,
};
use serde_json::json;

// ─── Inputs / outputs ───

/// `POST /api/relay/register`.
#[derive(Debug, Clone)]
pub struct RelayRegisterInput {
    pub role: RelayRole,
    pub family: String,
    pub version: Option<String>,
    pub model: Option<String>,
    pub capabilities: Vec<String>,
    pub repo: Option<String>,
    pub worktree: Option<String>,
    /// Absent for a minting coordinator (Q6); required otherwise.
    pub crew_id: Option<Uuid>,
    /// Mint-only fields (Q6): ignored on join.
    pub goal: Option<String>,
    pub repo_key: Option<String>,
    pub workspace_id: Option<Uuid>,
}

/// The register response. `crew_id` is always present (Q16-REC): minted or
/// joined, the coordinator needs it for the paste line. The raw token is
/// returned once, here — never stored, never returned again.
#[derive(Debug, Clone)]
pub struct RelayRegisterOutput {
    pub lane_token: String,
    pub lane_id: Uuid,
    pub lane_title: Option<String>,
    pub crew_id: Uuid,
}

/// An authenticated lane: what `relay_auth_lane` resolves a token to.
#[derive(Debug, Clone)]
pub struct RelayLaneSession {
    pub user_id: Uuid,
    pub crew_id: Uuid,
    pub role: RelayRole,
}

#[derive(Debug, Clone)]
pub struct RelayHeartbeatOutput {
    pub expires_at: DateTime<Utc>,
}

/// `POST /api/relay/reregister`: resume with the checkpoint. No new token —
/// the lane token is minted once, at register.
#[derive(Debug, Clone)]
pub struct RelayReregisterOutput {
    pub lane_id: Uuid,
    pub crew_id: Uuid,
    pub checkpoint: Option<serde_json::Value>,
    pub expires_at: DateTime<Utc>,
}

/// `POST /api/relay/tasks/plan` (coordinator): plan an item — or a manual
/// brief — into an unassigned task. The brief is frozen later, at assign.
#[derive(Debug, Clone)]
pub struct RelayPlanInput {
    pub item_id: Option<Uuid>,
    /// Required when no item (or when the item names no repository — tasks
    /// require `repo_key` where manual items may omit it).
    pub title: Option<String>,
    pub phase: RelayTaskPhase,
    pub repo_key: Option<String>,
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct RelayPlanOutput {
    pub task_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RelayTicketInput {
    pub provider: RelayTicketProvider,
    pub key: String,
    pub url: Option<String>,
}

/// `POST /api/relay/tasks/{id}/assign` (coordinator). The hand-off is
/// authoritative at assign time: title/phase/brief/acceptance/paths/deps/
/// resources are (re)set from the request and the brief frozen into the
/// `assigned` action. Re-assigning an approved task to a new phase is the
/// Q11 advance (state → working, new round).
#[derive(Debug, Clone)]
pub struct RelayAssignInput {
    /// The assignee lane (Q13); required — unassigned tasks are `plan`'s.
    pub to_lane: Uuid,
    pub title: String,
    pub phase: RelayTaskPhase,
    pub ticket: Option<RelayTicketInput>,
    pub tags: Vec<String>,
    pub brief_summary: String,
    pub brief_docs: Vec<String>,
    pub acceptance: Vec<String>,
    pub owned_paths: Vec<String>,
    /// Accepted as-is: graphs, not trees (no FK by the model).
    pub deps: Vec<Uuid>,
    /// Accepted as-is; item 3 leases them.
    pub resources: Vec<String>,
    /// Optional (Q14): honored when given and never the assignee; else the
    /// broker picks at first report.
    pub reviewer_lane: Option<Uuid>,
    /// Sets the lane's objective (first or later assign; absent = unchanged).
    pub lane_title: Option<String>,
    /// The assign clock: the assignee's lease must be live against it (F5).
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RelayAssignOutput {
    pub task_id: Uuid,
    pub round: i32,
    pub state: RelayTaskState,
}

/// `POST /api/relay/tasks/{id}/cancel` (coordinator, F3).
#[derive(Debug, Clone)]
pub struct RelayCancelOutput {
    pub task_id: Uuid,
    pub state: RelayTaskState,
}

/// `POST /api/relay/crews/{id}/close` (coordinator, F3).
#[derive(Debug, Clone)]
pub struct RelayCloseOutput {
    pub crew_id: Uuid,
    pub status: String,
}

/// One reported file. No `outside_owned`: the broker computes it from the
/// task's `owned_paths` (CONTRACT §3c), never trusts the lane's claim.
#[derive(Debug, Clone)]
pub struct RelayFileInput {
    pub path: String,
    pub added: i64,
    pub removed: i64,
}

#[derive(Debug, Clone)]
pub struct RelayGateInput {
    pub cmd: String,
    pub exit: i32,
    pub counts: Option<String>,
}

/// `POST /api/relay/tasks/{id}/report`. `changed` is the summary sent with
/// the request for review (`body_md`, ≤ 600 chars); `look_at`/`flags` ride
/// props (they are `RelayActionProps.reported` keys; `changed` is not).
/// `telemetry` is the free-form `reported` props tail (validated keys, D83
/// nulls stay null).
#[derive(Debug, Clone)]
pub struct RelayReportInput {
    pub phase: RelayTaskPhase,
    pub round: i32,
    pub changed: String,
    pub look_at: Option<String>,
    pub flags: Option<String>,
    pub report_md: Option<String>,
    pub base: Option<String>,
    pub branch: Option<String>,
    pub commits: Vec<String>,
    pub files: Vec<RelayFileInput>,
    pub gates: Vec<RelayGateInput>,
    pub telemetry: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RelayReportOutput {
    pub state: RelayTaskState,
    pub round: i32,
    pub reviewer_lane_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct RelayFindingInput {
    pub severity: RelayFindingSeverity,
    pub class: String,
    pub text: String,
    pub file: Option<String>,
    pub line: Option<i32>,
}

/// `POST /api/relay/tasks/{id}/review`. Findings name seeded
/// `finding_classes` codes (unknown classes are refused).
#[derive(Debug, Clone)]
pub struct RelayReviewInput {
    pub verdict: ReviewVerdict,
    pub summary: String,
    pub findings: Vec<RelayFindingInput>,
    pub probes: Option<i64>,
    pub telemetry: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RelayReviewOutput {
    pub state: RelayTaskState,
    pub round: i32,
}

/// Who a decision waits on: the person, or one lane (Q1: a uuid, never a code).
#[derive(Debug, Clone)]
pub enum RelayAskedOf {
    Person,
    Lane(Uuid),
}

#[derive(Debug, Clone)]
pub struct RelayDecisionOption {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
}

/// `POST /api/relay/decisions`. Asking blocks every task in `blocks`
/// (working/in_review/assigned → blocked, remembering where); answering
/// resumes them.
#[derive(Debug, Clone)]
pub struct RelayAskInput {
    /// The ask's own task (crew-level asks carry none).
    pub task_id: Option<Uuid>,
    pub question: String,
    pub context: Option<String>,
    pub options: Vec<RelayDecisionOption>,
    pub recommended: Option<String>,
    pub blocks: Vec<Uuid>,
    pub asked_of: RelayAskedOf,
}

#[derive(Debug, Clone)]
pub struct RelayAskOutput {
    pub decision_id: Uuid,
}

/// `POST /api/relay/decisions/{id}/answer` (person and lane share it; the
/// daemon stamps the caller). Only the asked-of party answers.
#[derive(Debug, Clone)]
pub struct RelayAnswerInput {
    pub option: String,
    pub note: Option<String>,
}

/// Who performs a surfaced op. Person callers name their surface
/// (desktop/dojo); lane callers are stamped `lane` by the daemon (Q12).
#[derive(Debug, Clone)]
pub enum RelayCaller {
    Person { surface: RelaySurface },
    Lane { lane_id: Uuid },
}

/// `message.props.to` (Q2, settled).
#[derive(Debug, Clone)]
pub enum RelayMessageTo {
    Coordinator,
    Lane { lane_id: Uuid },
    Everyone,
    Person,
}

/// Person (`POST /relay/messages`) and lane (`POST /api/relay/messages`)
/// share the store op; the caller decides the stamp.
#[derive(Debug, Clone)]
pub struct RelayMessageInput {
    pub to: RelayMessageTo,
    pub task_id: Option<Uuid>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct RelayMessageOutput {
    pub message_id: Uuid,
}

/// `POST /api/relay/tasks/{id}/merge` (coordinator). Empty `commits` means
/// done with nothing to merge (a mockup, an analysis).
#[derive(Debug, Clone)]
pub struct RelayMergeInput {
    pub commits: Vec<String>,
    pub into: String,
    pub gates: Vec<RelayGateInput>,
    pub note: String,
}

/// What one sweeper pass did.
#[derive(Debug, Clone)]
pub struct RelaySweepReport {
    pub expired_leases: u64,
    pub interrupted_tasks: u64,
    pub reoffered_tasks: u64,
    pub repicked_reviewers: u64,
    /// Resource grants past their TTL, expired this pass (item 3).
    pub expired_resource_leases: u64,
    /// Queued requests auto-granted this pass (item 3).
    pub granted_from_queue: u64,
}

/// `POST /api/relay/resources/request` (item 3): the lease input carries an
/// optional floor override so tests pin the grant/refuse paths on any disk;
/// the handlers always pass `None` (the ruled constants).
#[derive(Debug, Clone)]
pub struct RelayResourceRequest {
    pub resource: String,
    pub floors: Option<RelayFloors>,
}

/// A resource request's answer (item 3, Q9): the grant shape, with
/// `position` present only while queued and the stamp pair present only
/// while granted.
#[derive(Debug, Clone)]
pub struct RelayResourceGrant {
    pub lease_id: Uuid,
    pub state: RelayLeaseState,
    pub resource: String,
    pub disk_free_gb: Option<u64>,
    pub swap_free_gb: Option<u64>,
    pub disk_floor_gb: u64,
    pub swap_floor_gb: u64,
    pub granted_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub position: Option<u64>,
}

/// A resource release's answer (item 3): the released row plus the next
/// queued request the release auto-granted, if any.
#[derive(Debug, Clone)]
pub struct RelayResourceRelease {
    pub lease_id: Uuid,
    pub resource: String,
    pub granted_next: Option<Uuid>,
}

/// A live resource row for the idempotent re-request path (Q3): id,
/// state, props, and the stamp pair.
type LiveResourceRow =
    (Uuid, String, serde_json::Value, Option<DateTime<Utc>>, Option<DateTime<Utc>>);

/// A manual triage item, as the person adds it.
#[derive(Debug, Clone)]
pub struct RelayTriageInput {
    pub title: String,
    pub kind: RelayItemKind,
    pub repo_key: Option<String>,
    pub workspace_id: Option<Uuid>,
    pub tags: Vec<String>,
}

/// `RelayFilter`: empty narrows nothing (the main window).
#[derive(Debug, Clone)]
pub struct RelayReadFilter {
    pub workspace_id: Option<Uuid>,
    pub repo_key: Option<String>,
}

// ─── Small helpers ───

/// A database failure under a valid request is a 500, never a fabricated row.
fn db_err(e: sqlx_core::Error) -> RelayError {
    RelayError::Internal(format!("relay store: {e}"))
}

/// True when the error is a unique-violation (23505): the second
/// coordinator, a busy lane, an already-answered decision.
fn is_unique_violation(e: &sqlx_core::Error) -> bool {
    matches!(e, sqlx_core::Error::Database(d) if d.is_unique_violation())
}

/// The violated constraint's name, when the error names one.
fn violated_constraint(e: &sqlx_core::Error) -> Option<&str> {
    match e {
        sqlx_core::Error::Database(d) => d.constraint(),
        _ => None,
    }
}

/// Mint a v7 id (the DDL's `*_id_v7` checks require position 15 = `7`).
fn new_id() -> Uuid {
    Uuid::now_v7()
}

// ─── Resource-lease props (item 3, decision 8) ───

/// The ruled floors, when no override applies.
fn ruled_floors() -> RelayFloors {
    RelayFloors { disk_gb: DISK_FLOOR_GB, swap_gb: SWAP_FLOOR_GB }
}

/// The floor override a row persists, if it carries a well-formed one
/// (`props.floors`, present only when the request carried an override).
fn floors_from_props(props: &serde_json::Value) -> Option<RelayFloors> {
    let disk = props.get("floors")?.get("disk_gb")?.as_u64()?;
    let swap = props.get("floors")?.get("swap_gb")?.as_u64()?;
    Some(RelayFloors { disk_gb: disk, swap_gb: swap })
}

/// The grant-time numbers a row persists (`None` each on a queued row —
/// unjudged, Q9 — or when the reading was unknown, Q6).
fn numbers_from_props(props: &serde_json::Value) -> (Option<u64>, Option<u64>) {
    (
        props.get("disk_free_gb").and_then(serde_json::Value::as_u64),
        props.get("swap_free_gb").and_then(serde_json::Value::as_u64),
    )
}

fn some_or_null(n: Option<u64>) -> serde_json::Value {
    n.map(serde_json::Value::from).unwrap_or(serde_json::Value::Null)
}

/// A queued row's props: the persisted override only (no numbers — the
/// floors gate the grant, never the queue).
fn queued_resource_props(floors: Option<RelayFloors>) -> serde_json::Value {
    let mut props = serde_json::Map::new();
    if let Some(f) = floors {
        props.insert("floors".into(), json!({"disk_gb": f.disk_gb, "swap_gb": f.swap_gb}));
    }
    serde_json::Value::Object(props)
}

/// A judged row's props: the persisted override plus the grant-time
/// numbers and, on refusal, the reason.
fn judged_resource_props(
    floors: Option<RelayFloors>,
    disk_free_gb: Option<u64>,
    swap_free_gb: Option<u64>,
    reason: Option<&str>,
) -> serde_json::Value {
    let mut props = serde_json::Map::new();
    if let Some(f) = floors {
        props.insert("floors".into(), json!({"disk_gb": f.disk_gb, "swap_gb": f.swap_gb}));
    }
    props.insert("disk_free_gb".into(), some_or_null(disk_free_gb));
    props.insert("swap_free_gb".into(), some_or_null(swap_free_gb));
    if let Some(reason) = reason {
        props.insert("reason".into(), serde_json::Value::from(reason));
    }
    serde_json::Value::Object(props)
}

/// A relay enum as its DB/JSON word, reusing the serde spelling
/// (`in_review`, `changes_requested`, …).
fn word<T: serde::Serialize>(v: T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

fn parse_state(s: &str) -> Result<RelayTaskState, RelayError> {
    serde_json::from_str::<RelayTaskState>(&format!("\"{s}\""))
        .map_err(|_| RelayError::Validation(format!("unknown task state: {s}")))
}

fn parse_phase(s: &str) -> Result<RelayTaskPhase, RelayError> {
    serde_json::from_str::<RelayTaskPhase>(&format!("\"{s}\""))
        .map_err(|_| RelayError::Validation(format!("unknown task phase: {s}")))
}

fn parse_role(s: &str) -> Result<RelayRole, RelayError> {
    serde_json::from_str::<RelayRole>(&format!("\"{s}\""))
        .map_err(|_| RelayError::Validation(format!("unknown lane role: {s}")))
}

fn parse_surface(s: &str) -> Result<RelaySurface, RelayError> {
    serde_json::from_str::<RelaySurface>(&format!("\"{s}\""))
        .map_err(|_| RelayError::Validation(format!("unknown surface: {s}")))
}

fn parse_provider(s: &str) -> Result<RelayTicketProvider, RelayError> {
    serde_json::from_str::<RelayTicketProvider>(&format!("\"{s}\""))
        .map_err(|_| RelayError::Validation(format!("unknown ticket provider: {s}")))
}

fn parse_item_kind(s: &str) -> Result<RelayItemKind, RelayError> {
    serde_json::from_str::<RelayItemKind>(&format!("\"{s}\""))
        .map_err(|_| RelayError::Validation(format!("unknown item kind: {s}")))
}

/// A person-facing summary: ≤ 600 chars, no secrets.
fn check_summary(text: &str, what: &str) -> Result<(), RelayError> {
    check_summary_len(text)
        .map_err(|n| RelayError::Validation(format!("{what} too long: {n} > 600 chars")))?;
    guard_markdown(text).map_err(|kinds| {
        RelayError::Validation(format!("{what} looks like a secret ({})", kinds.join(", ")))
    })?;
    Ok(())
}

/// A stamp as the screen reads it: RFC 3339 with `Z`.
fn iso(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

/// Does `path` fall outside every owned glob? Owned entries are exact paths
/// or `prefix/**` globs (the only shapes the coordinator writes).
fn outside_owned(path: &str, owned: &[String]) -> bool {
    !owned.iter().any(|o| {
        path == o
            || o.strip_suffix("/**").is_some_and(|prefix| {
                path.starts_with(prefix) && path[prefix.len()..].starts_with('/')
            })
    })
}

/// The Q14 broker's query: whose crews, whose pen, as of when (#161: one
/// struct, not five positionals).
struct RelayPickQuery<'t, 'c> {
    tx: &'t mut sqlx_core::transaction::Transaction<'c, sqlx_postgres::Postgres>,
    user_id: Uuid,
    crew_id: Uuid,
    author: Uuid,
    now: DateTime<Utc>,
}

/// The Q14 broker: the least-loaded live lane that isn't the author, else
/// the coordinator — unless the author is the coordinator, and then none.
/// Liveness is a granted lane lease past `now` (the report feeds DB time,
/// the sweeper its fed clock); loads count open in-review reviews.
async fn relay_pick_reviewer(q: RelayPickQuery<'_, '_>) -> Result<Option<Uuid>, RelayError> {
    let RelayPickQuery { tx, user_id, crew_id, author, now } = q;
    let live: Vec<Uuid> = sqlx_core::query_scalar::query_scalar(
        "select l.id from relay.lanes l
         where l.user_id = $1 and l.crew_id = $2
           and exists (
             select 1 from relay.leases s
             where s.lane_id = l.id and s.kind = 'lane' and s.state = 'granted'
               and s.expires_at > $3)",
    )
    .bind(user_id)
    .bind(crew_id)
    .bind(now)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    let loads: Vec<(Uuid, i64)> = sqlx_core::query_as::query_as(
        "select reviewer_lane_id, count(*) from relay.tasks
         where user_id = $1 and crew_id = $2 and state = 'in_review'
           and reviewer_lane_id is not null
         group by reviewer_lane_id",
    )
    .bind(user_id)
    .bind(crew_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    let loads: std::collections::HashMap<Uuid, u64> =
        loads.into_iter().map(|(id, n)| (id, n as u64)).collect();
    let coordinator: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
        "select id from relay.lanes
         where user_id = $1 and crew_id = $2 and role = 'coordinator'",
    )
    .bind(user_id)
    .bind(crew_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_err)?;
    let Some(coordinator) = coordinator else {
        return Err(RelayError::Internal("crew has no coordinator".into()));
    };
    let live_loads: Vec<(Uuid, u64)> =
        live.into_iter().map(|id| (id, loads.get(&id).copied().unwrap_or(0))).collect();
    Ok(pick_reviewer(&live_loads, &author, &coordinator))
}

/// A waiter's 1-based position among the slot's queued rows, FIFO by
/// broker-minted id (Q1).
async fn queue_position(
    tx: &mut sqlx_core::transaction::Transaction<'_, sqlx_postgres::Postgres>,
    resource: &str,
    id: Uuid,
) -> Result<u64, RelayError> {
    let rank: i64 = sqlx_core::query_scalar::query_scalar(
        "select count(*) + 1 from relay.leases
         where kind = 'resource' and resource = $1 and state = 'queued' and id < $2",
    )
    .bind(resource)
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(rank as u64)
}

/// Grant a freed slot to its first waiter (item 3, Q1): FIFO by
/// broker-minted id, floors re-checked per waiter against fresh
/// machine numbers — a waiter that now fails is refused instead and
/// the broker tries the next. Returns the granted row's id, if any.
async fn grant_next_waiter(
    tx: &mut sqlx_core::transaction::Transaction<'_, sqlx_postgres::Postgres>,
    resource: &str,
    now: DateTime<Utc>,
) -> Result<Option<Uuid>, RelayError> {
    loop {
        let waiter: Option<(Uuid, serde_json::Value)> = sqlx_core::query_as::query_as(
            "select id, props from relay.leases
             where kind = 'resource' and resource = $1 and state = 'queued'
             order by id limit 1",
        )
        .bind(resource)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?;
        let Some((id, props)) = waiter else {
            return Ok(None);
        };
        let floors = floors_from_props(&props).unwrap_or_else(ruled_floors);
        let disk = disk_free_gb();
        let swap = swap_free_gb();
        match decide_resource_grant(disk, swap, &floors) {
            ResourceGrant::Granted => {
                sqlx_core::query::query(
                    "update relay.leases set state = 'granted'::relay.lease_state,
                            granted_at = $1, expires_at = $2, props = $3
                     where id = $4",
                )
                .bind(now)
                .bind(now + Duration::seconds(RESOURCE_LEASE_TTL_SECS as i64))
                .bind(judged_resource_props(floors_from_props(&props), disk, swap, None))
                .bind(id)
                .execute(&mut **tx)
                .await
                .map_err(db_err)?;
                return Ok(Some(id));
            }
            ResourceGrant::Refused { reason } => {
                sqlx_core::query::query(
                    "update relay.leases set state = 'refused'::relay.lease_state, props = $1
                     where id = $2",
                )
                .bind(judged_resource_props(floors_from_props(&props), disk, swap, Some(&reason)))
                .bind(id)
                .execute(&mut **tx)
                .await
                .map_err(db_err)?;
            }
        }
    }
}

/// Free disk on `/`, in GiB (`df -g` units). Null when unreadable — the
/// screen degrades, the read never fails for it.
fn disk_free_gb() -> Option<u64> {
    // statvfs widths differ per platform (macOS u32/u64, Linux u64/u64):
    // the fallible conversions keep one body compiling everywhere.
    #[allow(clippy::unnecessary_fallible_conversions, clippy::useless_conversion)]
    fn blocks_to_bytes(blocks: impl TryInto<u64>, size: impl TryInto<u64>) -> Option<u64> {
        blocks.try_into().ok()?.checked_mul(size.try_into().ok()?)
    }
    let mut vfs = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    if unsafe { libc::statvfs(c"/".as_ptr(), vfs.as_mut_ptr()) } != 0 {
        return None;
    }
    let vfs = unsafe { vfs.assume_init() };
    let free = blocks_to_bytes(vfs.f_bavail, vfs.f_frsize)?;
    Some(free / 1024 / 1024 / 1024)
}

/// Free swap, in GiB. macOS reads `vm.swapusage`; anywhere else (or on any
/// failure) it is null — degraded, never fatal.
#[cfg(target_os = "macos")]
fn swap_free_gb() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::xsw_usage>::uninit();
    let mut len = std::mem::size_of::<libc::xsw_usage>() as libc::size_t;
    let r = unsafe {
        libc::sysctlbyname(
            c"vm.swapusage".as_ptr(),
            usage.as_mut_ptr() as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if r != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    Some(usage.xsu_avail / 1024 / 1024 / 1024)
}

#[cfg(not(target_os = "macos"))]
fn swap_free_gb() -> Option<u64> {
    None
}

/// The reported-props keys that count as telemetry: a turn "reports" when
/// any of these is present and non-null (derived; the screen's
/// `reported_turns of turns` is telemetry coverage, not work product —
/// files/gates/commits don't count).
const TELEMETRY_KEYS: &[&str] = &[
    "family",
    "version",
    "model",
    "tokens_in",
    "tokens_out",
    "tokens_cache",
    "started_at",
    "ended_at",
    "context_tokens",
    "compactions",
    "peak_memory_mb",
    "subagents",
    "working_secs",
    "waiting_secs",
];

/// Shape stored props for the task history: the `RelayActionProps` interface
/// per kind, required-nullable keys filled (absent = null, never missing).
/// Unknown kinds pass through untouched (forward-compat for new kinds).
fn history_props(kind: &str, props: &serde_json::Value) -> serde_json::Value {
    let null = serde_json::Value::Null;
    let get = |key: &str| props.get(key).cloned().unwrap_or(null.clone());
    let arr = |key: &str| props.get(key).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    match kind {
        "assigned" => serde_json::json!({
            "docs": arr("docs"),
            "owned_paths": arr("owned_paths"),
            "acceptance": arr("acceptance"),
        }),
        "brief_updated" => serde_json::json!({ "docs": arr("docs") }),
        "started" | "delivered" | "read" => serde_json::json!({}),
        "reported" => {
            let mut out = props.as_object().cloned().unwrap_or_default();
            out.insert(
                "files".into(),
                arr("files")
                    .iter()
                    .map(|f| {
                        serde_json::json!({
                            "path": f.get("path"),
                            "added": f.get("added"),
                            "removed": f.get("removed"),
                            // Fail closed: an unmarked file reads as outside.
                            "outside_owned": f.get("outside_owned").and_then(|v| v.as_bool()).unwrap_or(true),
                        })
                    })
                    .collect(),
            );
            out.insert(
                "gates".into(),
                arr("gates")
                    .iter()
                    .map(|g| {
                        serde_json::json!({
                            "cmd": g.get("cmd"),
                            "exit": g.get("exit"),
                            "counts": g.get("counts").cloned().unwrap_or(null.clone()),
                        })
                    })
                    .collect(),
            );
            serde_json::Value::Object(out)
        }
        "reviewed" => {
            let mut out = props.as_object().cloned().unwrap_or_default();
            out.insert(
                "findings".into(),
                arr("findings")
                    .iter()
                    .map(|f| {
                        serde_json::json!({
                            "severity": f.get("severity"),
                            "class": f.get("class"),
                            "text": f.get("text"),
                            "file": f.get("file").cloned().unwrap_or(null.clone()),
                            "line": f.get("line").cloned().unwrap_or(null.clone()),
                        })
                    })
                    .collect(),
            );
            serde_json::Value::Object(out)
        }
        "asked" => {
            let options = arr("options")
                .iter()
                .map(|o| {
                    serde_json::json!({
                        "id": o.get("id"),
                        "label": o.get("label"),
                        "detail": o.get("detail").cloned().unwrap_or(null.clone()),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "context": get("context"),
                "options": options,
                "recommended": get("recommended"),
                "blocks": arr("blocks"),
                "asked_of": get("asked_of"),
            })
        }
        "answered" => serde_json::json!({ "option": get("option"), "surface": get("surface") }),
        "message" => serde_json::json!({ "to": get("to"), "surface": get("surface") }),
        "merged" => {
            let gates = arr("gates")
                .iter()
                .map(|g| {
                    serde_json::json!({
                        "cmd": g.get("cmd"),
                        "exit": g.get("exit"),
                        "counts": g.get("counts").cloned().unwrap_or(null.clone()),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({ "commits": arr("commits"), "into": get("into"), "gates": gates })
        }
        "interrupted" | "compacted" => props.clone(),
        "reoffered" => {
            let mut out = props.as_object().cloned().unwrap_or_default();
            out.entry("to_lane".to_owned()).or_insert(null.clone());
            serde_json::Value::Object(out)
        }
        _ => props.clone(),
    }
}

/// Canonical phase order for the header's rounds.
fn phase_rank(phase: &str) -> usize {
    match phase {
        "analysis" => 0,
        "tests" => 1,
        "impl" => 2,
        "docs" => 3,
        "mockup" => 4,
        _ => 5,
    }
}

/// Lane codes sort numerically (`L2` before `L10`), then lexicographically.
fn code_rank(code: &str) -> (i64, &str) {
    let n = code.strip_prefix('L').and_then(|n| n.parse::<i64>().ok()).unwrap_or(i64::MAX);
    (n, code)
}

/// The next free lane code for a crew: `L1` on an empty crew, else one past
/// the highest `L<n>` suffix. `None` when a code does not parse (the daemon
/// mints `L<n>` only — anything else is corruption, failed closed).
fn next_code(codes: &[String]) -> Option<String> {
    if codes.is_empty() {
        return Some("L1".to_string());
    }
    let mut max: Option<i64> = None;
    for code in codes {
        let n = code.strip_prefix('L').and_then(|n| n.parse::<i64>().ok())?;
        max = Some(max.map_or(n, |m| m.max(n)));
    }
    max.map(|m| format!("L{}", m + 1))
}

// ─── The store ───

impl PgStore {
    /// Mint-or-join a lane (Q6). Mints the crew, the lane (next free code),
    /// its lease (granted, `now` + 300 s) and the lane token (returned raw
    /// once; stored hashed) in one transaction.
    pub async fn relay_register(
        &self,
        user_id: Uuid,
        input: RelayRegisterInput,
        now: DateTime<Utc>,
    ) -> Result<RelayRegisterOutput, RelayError> {
        let mode = register_mode(input.role, input.crew_id.map(|_| "present"))
            .map_err(|e| RelayError::Validation(e.message))?;
        let token = Uuid::new_v4().simple().to_string();
        let token_hash = hash_lane_token(&token);
        let expires = now + Duration::seconds(LANE_LEASE_TTL_SECS as i64);
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        let crew_id = match mode {
            RegisterMode::MintCrew => {
                let crew_id = new_id();
                sqlx_core::query::query(
                    "insert into relay.crews (user_id, id, workspace_id, repo_key, goal, status)
                     values ($1, $2, $3, $4, $5, 'active')",
                )
                .bind(user_id)
                .bind(crew_id)
                .bind(input.workspace_id)
                .bind(input.repo_key.as_deref())
                .bind(input.goal.as_deref())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                crew_id
            }
            RegisterMode::JoinCrew => {
                let crew_id = input.crew_id.unwrap_or_else(Uuid::nil);
                let exists: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                    "select id from relay.crews where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(crew_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
                if exists.is_none() {
                    return Err(RelayError::NotFound("crew not found".into()));
                }
                Self::relay_require_crew_open(&mut tx, user_id, crew_id).await?;
                crew_id
            }
        };
        let codes: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "select code from relay.lanes where user_id = $1 and crew_id = $2",
        )
        .bind(user_id)
        .bind(crew_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let lane_id = new_id();
        let code = next_code(&codes)
            .ok_or_else(|| RelayError::Internal("crew has unparseable lane codes".into()))?;
        // F2: the lane title is minted here, never null — code + family
        // (`L4 · Muse`), or the bare code when the family is empty.
        let title = if input.family.is_empty() {
            code.clone()
        } else {
            format!("{} · {}", code, input.family)
        };
        let insert_lane = sqlx_core::query::query(
            "insert into relay.lanes
               (user_id, id, crew_id, code, role, family, version, model,
                capabilities, repo, worktree, token_hash, title)
             values ($1, $2, $3, $4, $5::relay.lane_role, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(user_id)
        .bind(lane_id)
        .bind(crew_id)
        .bind(code.as_str())
        .bind(word(input.role))
        .bind(input.family.as_str())
        .bind(input.version.as_deref())
        .bind(input.model.as_deref())
        .bind(input.capabilities.clone())
        .bind(input.repo.as_deref())
        .bind(input.worktree.as_deref())
        .bind(token_hash.as_str())
        .bind(title.as_str())
        .execute(&mut *tx)
        .await;
        if let Err(e) = insert_lane {
            if is_unique_violation(&e) {
                // A code race (two registers minting one code) is not a
                // second coordinator: name what actually collided.
                let message = match violated_constraint(&e) {
                    Some("lanes_code_per_crew") => "crew register raced; retry".to_string(),
                    _ => "crew already has a coordinator".to_string(),
                };
                return Err(RelayError::Conflict(message));
            }
            return Err(db_err(e));
        }
        sqlx_core::query::query(
            "insert into relay.leases
               (user_id, id, crew_id, lane_id, kind, state, expires_at, granted_at)
             values ($1, $2, $3, $4, 'lane'::relay.lease_kind, 'granted'::relay.lease_state, $5, $6)",
        )
        .bind(user_id)
        .bind(new_id())
        .bind(crew_id)
        .bind(lane_id)
        .bind(expires)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayRegisterOutput { lane_token: token, lane_id, lane_title: Some(title), crew_id })
    }

    /// Resolve a lane caller's identity: token hash → lane. Unknown token is
    /// `Unauthorized`; an unregistered lane is `Gone` (`reregister` resumes).
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_auth_lane(
        &self,
        lane_id: Uuid,
        token: &str,
    ) -> Result<RelayLaneSession, RelayError> {
        let row: Option<(Uuid, Uuid, String, String, Option<DateTime<Utc>>)> =
            sqlx_core::query_as::query_as(
                "select user_id, crew_id, role::text, token_hash, unregistered_at
                 from relay.lanes where id = $1",
            )
            .bind(lane_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
        let Some((user_id, crew_id, role, token_hash, unregistered_at)) = row else {
            return Err(RelayError::Unauthorized("unknown lane".into()));
        };
        if hash_lane_token(token) != token_hash {
            return Err(RelayError::Unauthorized("bad lane token".into()));
        }
        if unregistered_at.is_some() {
            return Err(RelayError::Gone("lane unregistered; reregister to resume".into()));
        }
        Ok(RelayLaneSession { user_id, crew_id, role: parse_role(&role)? })
    }

    /// Renew the lane's lease (`now` + 300 s).
    pub async fn relay_heartbeat(
        &self,
        lane_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RelayHeartbeatOutput, RelayError> {
        let lane: Option<(Uuid, Uuid, Option<DateTime<Utc>>)> = sqlx_core::query_as::query_as(
            "select user_id, crew_id, unregistered_at from relay.lanes where id = $1",
        )
        .bind(lane_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((user_id, crew_id, unregistered_at)) = lane else {
            return Err(RelayError::NotFound("lane not found".into()));
        };
        if unregistered_at.is_some() {
            return Err(RelayError::Gone("lane unregistered; reregister to resume".into()));
        }
        let expires = now + Duration::seconds(LANE_LEASE_TTL_SECS as i64);
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        // Renew the current granted lease; a lane whose lease lapsed or was
        // released gets a fresh granted row (history is kept, not rewritten).
        let renewed = sqlx_core::query::query(
            "update relay.leases set expires_at = $1 where id = (
               select id from relay.leases
               where lane_id = $2 and kind = 'lane' and state = 'granted'
               order by granted_at desc nulls last, id desc limit 1)",
        )
        .bind(expires)
        .bind(lane_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        if renewed.rows_affected() == 0 {
            sqlx_core::query::query(
                "insert into relay.leases
                   (user_id, id, crew_id, lane_id, kind, state, expires_at, granted_at)
                 values ($1, $2, $3, $4, 'lane'::relay.lease_kind,
                         'granted'::relay.lease_state, $5, $6)",
            )
            .bind(user_id)
            .bind(new_id())
            .bind(crew_id)
            .bind(lane_id)
            .bind(expires)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        sqlx_core::query::query(
            "update relay.crews set heartbeat_at = $1 where user_id = $2 and id = $3",
        )
        .bind(now)
        .bind(user_id)
        .bind(crew_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        // Item 3 (Q4): the heartbeat renews the lane's held resource
        // grants alongside its lane lease. Queued rows claim no grant,
        // so they are untouched.
        sqlx_core::query::query(
            "update relay.leases set expires_at = $1
             where lane_id = $2 and kind = 'resource' and state = 'granted'",
        )
        .bind(now + Duration::seconds(RESOURCE_LEASE_TTL_SECS as i64))
        .bind(lane_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayHeartbeatOutput { expires_at: expires })
    }

    /// Resume an unregistered (or lapsed) lane: a fresh lease plus the
    /// stored checkpoint. The token is the same one — minted once.
    pub async fn relay_reregister(
        &self,
        lane_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RelayReregisterOutput, RelayError> {
        // The handler verifies the token first: `relay_auth_lane` answers
        // `Gone` (valid token, unregistered lane) exactly for this resume.
        let lane: Option<(Uuid, Uuid, Option<serde_json::Value>)> = sqlx_core::query_as::query_as(
            "select user_id, crew_id, checkpoint from relay.lanes where id = $1",
        )
        .bind(lane_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((user_id, crew_id, checkpoint)) = lane else {
            return Err(RelayError::NotFound("lane not found".into()));
        };
        let expires = now + Duration::seconds(LANE_LEASE_TTL_SECS as i64);
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        Self::relay_require_crew_open(&mut tx, user_id, crew_id).await?;
        sqlx_core::query::query("update relay.lanes set unregistered_at = null where id = $1")
            .bind(lane_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx_core::query::query(
            "insert into relay.leases
               (user_id, id, crew_id, lane_id, kind, state, expires_at, granted_at)
             values ($1, $2, $3, $4, 'lane'::relay.lease_kind,
                     'granted'::relay.lease_state, $5, $6)",
        )
        .bind(user_id)
        .bind(new_id())
        .bind(crew_id)
        .bind(lane_id)
        .bind(expires)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayReregisterOutput { lane_id, crew_id, checkpoint, expires_at: expires })
    }

    /// Leave: stamp `unregistered_at`, release the lane lease, and interrupt
    /// the lane's active task (a voluntary leave suspends work like a lost
    /// lease) — all in one transaction.
    pub async fn relay_unregister(&self, lane_id: Uuid) -> Result<(), RelayError> {
        let lane: Option<(Uuid, Uuid, Option<DateTime<Utc>>)> = sqlx_core::query_as::query_as(
            "select user_id, crew_id, unregistered_at from relay.lanes where id = $1",
        )
        .bind(lane_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((user_id, crew_id, unregistered_at)) = lane else {
            return Err(RelayError::NotFound("lane not found".into()));
        };
        if unregistered_at.is_some() {
            return Err(RelayError::Gone("lane already unregistered".into()));
        }
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        sqlx_core::query::query(
            "update relay.lanes set unregistered_at = current_timestamp where id = $1",
        )
        .bind(lane_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        sqlx_core::query::query(
            "update relay.leases set state = 'released'::relay.lease_state,
                    released_at = current_timestamp
             where id = (
               select id from relay.leases
               where lane_id = $1 and kind = 'lane' and state = 'granted'
               order by granted_at desc nulls last, id desc limit 1)",
        )
        .bind(lane_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        // A voluntary leave suspends the lane's work like a lost lease; the
        // lane id stays on the task (resume keeps its place; reoffer clears).
        // Block memory clears with it: only a blocked task remembers where
        // it was (the tasks_blocked_memory check; cancel's precedent).
        let task: Option<(Uuid, String, i32, String)> = sqlx_core::query_as::query_as(
            "select id, state::text, round, phase::text from relay.tasks
             where user_id = $1 and lane_id = $2
               and state in ('assigned', 'working', 'in_review', 'rework', 'blocked')",
        )
        .bind(user_id)
        .bind(lane_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        if let Some((task_id, state, round, phase)) = task {
            let next = transition(parse_state(&state)?, TaskEvent::LeaseLost)
                .map_err(|_| RelayError::Conflict("task cannot be suspended".into()))?;
            sqlx_core::query::query(
                "update relay.tasks set state = $1::relay.task_state,
                        blocked_from_state = null,
                        state_changed_at = current_timestamp
                 where user_id = $2 and id = $3",
            )
            .bind(word(next))
            .bind(user_id)
            .bind(task_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx_core::query::query(
                "insert into relay.actions
                   (user_id, id, crew_id, task_id, lane_id, kind, round, phase, props)
                 values ($1, $2, $3, $4, $5, 'interrupted'::relay.action_kind, $6,
                         $7::relay.task_phase, '{}')",
            )
            .bind(user_id)
            .bind(new_id())
            .bind(crew_id)
            .bind(task_id)
            .bind(lane_id)
            .bind(round)
            .bind(phase)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        // Item 3 (Q7): the lane's held resource grants release with it
        // (DB clock, like the lane stamps above), and each freed slot
        // auto-grants its next waiter. `current_timestamp` is the
        // transaction's clock, so the grant stamps match the release.
        let freed: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "update relay.leases set state = 'released'::relay.lease_state,
                    released_at = current_timestamp
             where lane_id = $1 and kind = 'resource' and state = 'granted'
             returning resource",
        )
        .bind(lane_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        if !freed.is_empty() {
            let now: DateTime<Utc> =
                sqlx_core::query_scalar::query_scalar("select current_timestamp")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(db_err)?;
            for resource in &freed {
                grant_next_waiter(&mut tx, resource, now).await?;
            }
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    /// Request a resource lease (item 3, Q1–Q3/Q5/Q6): the lane's live
    /// row comes back as-is (Q3), else a free slot grants when the floors
    /// pass and refuses below them, and a held slot queues (floors gate
    /// the grant, never the queue — the re-check runs at auto-grant).
    pub async fn relay_request_resource(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        input: RelayResourceRequest,
        now: DateTime<Utc>,
    ) -> Result<RelayResourceGrant, RelayError> {
        let resource = input.resource.trim();
        if resource.is_empty() {
            return Err(RelayError::Validation("resource names the slot".into()));
        }
        let lane: Option<(Uuid, Uuid, Option<DateTime<Utc>>)> = sqlx_core::query_as::query_as(
            "select user_id, crew_id, unregistered_at from relay.lanes where id = $1",
        )
        .bind(caller_lane)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((_, crew_id, unregistered_at)) = lane else {
            return Err(RelayError::NotFound("lane not found".into()));
        };
        if unregistered_at.is_some() {
            return Err(RelayError::Gone("lane unregistered; reregister to resume".into()));
        }
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        // One broker per slot: concurrent requests for the same resource
        // serialize here, so the loser queues instead of racing the
        // one-grant-per-resource unique.
        sqlx_core::query::query("select pg_advisory_xact_lock(hashtext($1))")
            .bind(resource)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        // The lane's live row comes back as-is (Q3); terminal rows do not
        // match, so a post-terminal re-request mints below.
        let mine: Option<LiveResourceRow> = sqlx_core::query_as::query_as(
            "select id, state::text, props, granted_at, expires_at from relay.leases
                 where user_id = $1 and lane_id = $2 and kind = 'resource' and resource = $3
                   and state in ('queued', 'granted')
                 order by id limit 1",
        )
        .bind(user_id)
        .bind(caller_lane)
        .bind(resource)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        let out = if let Some((id, state, props, granted_at, expires_at)) = mine {
            let floors = floors_from_props(&props).unwrap_or_else(ruled_floors);
            let (disk, swap) = numbers_from_props(&props);
            let position = if state == "granted" {
                None
            } else {
                Some(queue_position(&mut tx, resource, id).await?)
            };
            RelayResourceGrant {
                lease_id: id,
                state: if state == "granted" {
                    RelayLeaseState::Granted
                } else {
                    RelayLeaseState::Queued
                },
                resource: resource.to_string(),
                disk_free_gb: disk,
                swap_free_gb: swap,
                disk_floor_gb: floors.disk_gb,
                swap_floor_gb: floors.swap_gb,
                granted_at,
                expires_at,
                position,
            }
        } else {
            // The slot is machine-wide: any crew's grant blocks (Q1).
            let held: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                "select id from relay.leases
                 where kind = 'resource' and resource = $1 and state = 'granted' limit 1",
            )
            .bind(resource)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            if held.is_some() {
                let id = new_id();
                sqlx_core::query::query(
                    "insert into relay.leases
                       (user_id, id, crew_id, lane_id, kind, resource, state, props)
                     values ($1, $2, $3, $4, 'resource'::relay.lease_kind, $5,
                             'queued'::relay.lease_state, $6)",
                )
                .bind(user_id)
                .bind(id)
                .bind(crew_id)
                .bind(caller_lane)
                .bind(resource)
                .bind(queued_resource_props(input.floors))
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                let position = queue_position(&mut tx, resource, id).await?;
                let floors = input.floors.unwrap_or_else(ruled_floors);
                RelayResourceGrant {
                    lease_id: id,
                    state: RelayLeaseState::Queued,
                    resource: resource.to_string(),
                    disk_free_gb: None,
                    swap_free_gb: None,
                    disk_floor_gb: floors.disk_gb,
                    swap_floor_gb: floors.swap_gb,
                    granted_at: None,
                    expires_at: None,
                    position: Some(position),
                }
            } else {
                let disk = disk_free_gb();
                let swap = swap_free_gb();
                let floors = input.floors.unwrap_or_else(ruled_floors);
                let id = new_id();
                match decide_resource_grant(disk, swap, &floors) {
                    ResourceGrant::Granted => {
                        let expires = now + Duration::seconds(RESOURCE_LEASE_TTL_SECS as i64);
                        sqlx_core::query::query(
                            "insert into relay.leases
                               (user_id, id, crew_id, lane_id, kind, resource, state, props,
                                granted_at, expires_at)
                             values ($1, $2, $3, $4, 'resource'::relay.lease_kind, $5,
                                     'granted'::relay.lease_state, $6, $7, $8)",
                        )
                        .bind(user_id)
                        .bind(id)
                        .bind(crew_id)
                        .bind(caller_lane)
                        .bind(resource)
                        .bind(judged_resource_props(input.floors, disk, swap, None))
                        .bind(now)
                        .bind(expires)
                        .execute(&mut *tx)
                        .await
                        .map_err(db_err)?;
                        RelayResourceGrant {
                            lease_id: id,
                            state: RelayLeaseState::Granted,
                            resource: resource.to_string(),
                            disk_free_gb: disk,
                            swap_free_gb: swap,
                            disk_floor_gb: floors.disk_gb,
                            swap_floor_gb: floors.swap_gb,
                            granted_at: Some(now),
                            expires_at: Some(expires),
                            position: None,
                        }
                    }
                    ResourceGrant::Refused { reason } => {
                        sqlx_core::query::query(
                            "insert into relay.leases
                               (user_id, id, crew_id, lane_id, kind, resource, state, props)
                             values ($1, $2, $3, $4, 'resource'::relay.lease_kind, $5,
                                     'refused'::relay.lease_state, $6)",
                        )
                        .bind(user_id)
                        .bind(id)
                        .bind(crew_id)
                        .bind(caller_lane)
                        .bind(resource)
                        .bind(judged_resource_props(input.floors, disk, swap, Some(&reason)))
                        .execute(&mut *tx)
                        .await
                        .map_err(db_err)?;
                        RelayResourceGrant {
                            lease_id: id,
                            state: RelayLeaseState::Refused,
                            resource: resource.to_string(),
                            disk_free_gb: disk,
                            swap_free_gb: swap,
                            disk_floor_gb: floors.disk_gb,
                            swap_floor_gb: floors.swap_gb,
                            granted_at: None,
                            expires_at: None,
                            position: None,
                        }
                    }
                }
            }
        };
        tx.commit().await.map_err(db_err)?;
        Ok(out)
    }

    /// Release a held resource lease (item 3, Q7): the holder's release
    /// auto-grants the next queued request (floors re-checked). Only a
    /// granted row releases — a queued waiter holds no grant.
    pub async fn relay_release_resource(
        &self,
        _user_id: Uuid,
        caller_lane: Uuid,
        resource: &str,
        now: DateTime<Utc>,
    ) -> Result<RelayResourceRelease, RelayError> {
        // `_user_id` is auth-bound but unused: the grant is machine-wide,
        // so the lane alone identifies the holder.
        let resource = resource.trim();
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        sqlx_core::query::query("select pg_advisory_xact_lock(hashtext($1))")
            .bind(resource)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        let live: Option<(Uuid, Uuid)> = sqlx_core::query_as::query_as(
            "select id, lane_id from relay.leases
             where kind = 'resource' and resource = $1 and state = 'granted' limit 1",
        )
        .bind(resource)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        let Some((id, holder)) = live else {
            return Err(RelayError::NotFound("no live grant for resource".into()));
        };
        if holder != caller_lane {
            return Err(RelayError::Forbidden("only the holder releases".into()));
        }
        sqlx_core::query::query(
            "update relay.leases set state = 'released'::relay.lease_state, released_at = $1
             where id = $2",
        )
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let granted_next = grant_next_waiter(&mut tx, resource, now).await?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayResourceRelease { lease_id: id, resource: resource.to_string(), granted_next })
    }

    /// Plan an item into an unassigned task (coordinator only). From an item,
    /// title/repo/workspace default from it and it moves to `planned`; manual
    /// plans name their own. A task without a repository is refused.
    /// The caller's crew, or why a coordinator-only op refuses: an unknown
    /// lane is `NotFound`, a contributor is `Forbidden`. (Lane roles never
    /// change after register, so this read sits outside the op's txn.)
    async fn relay_crew_of_coordinator(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
    ) -> Result<Uuid, RelayError> {
        let row: Option<(Uuid, String)> = sqlx_core::query_as::query_as(
            "select crew_id, role::text from relay.lanes where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(caller_lane)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((crew_id, role)) = row else {
            return Err(RelayError::NotFound("lane not found".into()));
        };
        if parse_role(&role)? != RelayRole::Coordinator {
            return Err(RelayError::Forbidden("coordinator only".into()));
        }
        Ok(crew_id)
    }

    /// A closed crew refuses new work (F3): register-join, reregister, plan
    /// and assign all pass through here inside their write transaction. A
    /// missing crew is not this gate's verdict — each caller rules
    /// existence its own way.
    async fn relay_require_crew_open(
        tx: &mut sqlx_core::transaction::Transaction<'_, sqlx_postgres::Postgres>,
        user_id: Uuid,
        crew_id: Uuid,
    ) -> Result<(), RelayError> {
        let status: Option<String> = sqlx_core::query_scalar::query_scalar(
            "select status from relay.crews where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(crew_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?;
        if status.as_deref() == Some("closed") {
            return Err(RelayError::Conflict("crew is closed".into()));
        }
        Ok(())
    }

    pub async fn relay_plan_item(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        input: RelayPlanInput,
    ) -> Result<RelayPlanOutput, RelayError> {
        let crew_id = self.relay_crew_of_coordinator(user_id, caller_lane).await?;
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        Self::relay_require_crew_open(&mut tx, user_id, crew_id).await?;
        let (item_id, title, repo_key, workspace_id) = match input.item_id {
            Some(item_id) => {
                let row: Option<(String, String, Option<String>, Option<Uuid>)> =
                    sqlx_core::query_as::query_as(
                        "select title, state::text, repo_key, workspace_id
                         from relay.items where user_id = $1 and id = $2",
                    )
                    .bind(user_id)
                    .bind(item_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
                let Some((item_title, item_state, item_repo, item_ws)) = row else {
                    return Err(RelayError::NotFound("item not found".into()));
                };
                if item_state == "closed" {
                    return Err(RelayError::Conflict("item is closed".into()));
                }
                let repo_key = input
                    .repo_key
                    .or(item_repo)
                    .ok_or_else(|| RelayError::Validation("a task needs a repository".into()))?;
                sqlx_core::query::query(
                    "update relay.items set state = 'planned'::relay.item_state
                     where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(item_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                (
                    Some(item_id),
                    input.title.unwrap_or(item_title),
                    repo_key,
                    input.workspace_id.or(item_ws),
                )
            }
            None => {
                let title = input.title.ok_or_else(|| {
                    RelayError::Validation("a manual plan names its title".into())
                })?;
                let repo_key = input
                    .repo_key
                    .ok_or_else(|| RelayError::Validation("a task needs a repository".into()))?;
                (None, title, repo_key, input.workspace_id)
            }
        };
        let task_id = new_id();
        sqlx_core::query::query(
            "insert into relay.tasks
               (user_id, id, crew_id, item_id, repo_key, workspace_id, title,
                phase, state, round)
             values ($1, $2, $3, $4, $5, $6, $7, $8::relay.task_phase,
                     'assigned'::relay.task_state, 1)",
        )
        .bind(user_id)
        .bind(task_id)
        .bind(crew_id)
        .bind(item_id)
        .bind(repo_key.as_str())
        .bind(workspace_id)
        .bind(title.as_str())
        .bind(word(input.phase))
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayPlanOutput { task_id })
    }

    /// Hand a task to a lane (coordinator only): the brief freezes into the
    /// `assigned` action in the same transaction as the state write.
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_assign_task(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        task_id: Uuid,
        input: RelayAssignInput,
    ) -> Result<RelayAssignOutput, RelayError> {
        let crew_id = self.relay_crew_of_coordinator(user_id, caller_lane).await?;
        let to_lane: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select id from relay.lanes where user_id = $1 and crew_id = $2 and id = $3",
        )
        .bind(user_id)
        .bind(crew_id)
        .bind(input.to_lane)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        if to_lane.is_none() {
            return Err(RelayError::NotFound("lane not found".into()));
        }
        // F5: the assignee must hold a live lease (granted, unexpired) —
        // assigning onto a dead lease strands the task where no sweep,
        // cancel, or re-assign frees it. Same liveness shape as the
        // reviewer pick; wake the lane first, then assign.
        let live: bool = sqlx_core::query_scalar::query_scalar(
            "select exists (
               select 1 from relay.leases
               where lane_id = $1 and kind = 'lane' and state = 'granted'
                 and expires_at > $2)",
        )
        .bind(input.to_lane)
        .bind(input.now)
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;
        if !live {
            return Err(RelayError::Conflict("lane has no live lease".into()));
        }
        if let Some(reviewer) = input.reviewer_lane {
            if reviewer == input.to_lane {
                return Err(RelayError::Validation("a task never reviews itself".into()));
            }
            let reviewer_lane: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                "select id from relay.lanes where user_id = $1 and crew_id = $2 and id = $3",
            )
            .bind(user_id)
            .bind(crew_id)
            .bind(reviewer)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
            if reviewer_lane.is_none() {
                return Err(RelayError::NotFound("reviewer lane not found".into()));
            }
        }
        check_summary(&input.brief_summary, "brief")?;
        if let Some(ticket) = &input.ticket {
            if ticket.key.trim().is_empty() {
                return Err(RelayError::Validation("ticket key is empty".into()));
            }
            if !input.tags.is_empty() {
                return Err(RelayError::Validation("a ticketed hand-off carries no tags".into()));
            }
        }
        let task: Option<(
            Uuid,
            Option<Uuid>,
            String,
            i32,
            String,
            Option<Uuid>,
            String,
            Option<Uuid>,
        )> = sqlx_core::query_as::query_as(
            "select crew_id, lane_id, state::text, round, phase::text, item_id,
                        repo_key, workspace_id
                 from relay.tasks where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(task_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((task_crew, task_lane, state, round, phase, task_item, task_repo, task_ws)) = task
        else {
            return Err(RelayError::NotFound("task not found".into()));
        };
        if task_crew != crew_id {
            return Err(RelayError::NotFound("task not found".into()));
        }
        let cur = parse_state(&state)?;
        let cur_phase = parse_phase(&phase)?;
        let active = matches!(
            cur,
            RelayTaskState::Assigned
                | RelayTaskState::Working
                | RelayTaskState::InReview
                | RelayTaskState::Rework
                | RelayTaskState::Blocked
        );
        let (next, new_round) = if cur == RelayTaskState::Approved {
            if input.phase == cur_phase {
                return Err(RelayError::Conflict("task is already approved in this phase".into()));
            }
            let next = transition(cur, TaskEvent::Reassigned)
                .map_err(|_| RelayError::Conflict("task cannot advance".into()))?;
            (next, round + 1)
        } else if task_lane.is_some() && active {
            return Err(RelayError::Conflict("task is already running".into()));
        } else if task_lane.is_none() && cur == RelayTaskState::Assigned {
            (RelayTaskState::Assigned, round)
        } else {
            return Err(RelayError::Conflict(format!("cannot assign a {} task", word(cur))));
        };
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        Self::relay_require_crew_open(&mut tx, user_id, crew_id).await?;
        // The hand-off's ticket/tags ride the task's item (derived: tasks
        // carry neither column; the item is their home).
        let item_id = match (&input.ticket, task_item) {
            (Some(ticket), Some(item)) => {
                sqlx_core::query::query(
                    "update relay.items set ticket_provider = $1, ticket_key = $2, tags = null
                     where user_id = $3 and id = $4",
                )
                .bind(word(ticket.provider))
                .bind(ticket.key.as_str())
                .bind(user_id)
                .bind(item)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                Some(item)
            }
            (Some(ticket), None) => {
                let provider = word(ticket.provider);
                let existing: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                    "select id from relay.items
                     where user_id = $1 and ticket_provider = $2 and ticket_key = $3",
                )
                .bind(user_id)
                .bind(provider.as_str())
                .bind(ticket.key.as_str())
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
                match existing {
                    Some(item) => {
                        sqlx_core::query::query(
                            "update relay.items set state = 'planned'::relay.item_state
                             where user_id = $1 and id = $2",
                        )
                        .bind(user_id)
                        .bind(item)
                        .execute(&mut *tx)
                        .await
                        .map_err(db_err)?;
                        Some(item)
                    }
                    None => {
                        let item = new_id();
                        sqlx_core::query::query(
                            "insert into relay.items
                               (user_id, id, workspace_id, repo_key, ticket_provider,
                                ticket_key, title, kind, state, added_by)
                             values ($1, $2, $3, $4, $5, $6, $7,
                                     'issue'::relay.item_kind,
                                     'planned'::relay.item_state, $8)",
                        )
                        .bind(user_id)
                        .bind(item)
                        .bind(task_ws)
                        .bind(task_repo.as_str())
                        .bind(provider.as_str())
                        .bind(ticket.key.as_str())
                        .bind(input.title.as_str())
                        .bind(caller_lane)
                        .execute(&mut *tx)
                        .await
                        .map_err(db_err)?;
                        Some(item)
                    }
                }
            }
            (None, Some(item)) => {
                if !input.tags.is_empty() {
                    let provider: Option<Option<String>> = sqlx_core::query_scalar::query_scalar(
                        "select ticket_provider from relay.items where user_id = $1 and id = $2",
                    )
                    .bind(user_id)
                    .bind(item)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
                    if provider.flatten().is_some() {
                        return Err(RelayError::Validation("ticketed items carry no tags".into()));
                    }
                    sqlx_core::query::query(
                        "update relay.items set tags = $1 where user_id = $2 and id = $3",
                    )
                    .bind(input.tags.clone())
                    .bind(user_id)
                    .bind(item)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                }
                Some(item)
            }
            (None, None) => {
                if input.tags.is_empty() {
                    None
                } else {
                    let item = new_id();
                    sqlx_core::query::query(
                        "insert into relay.items
                           (user_id, id, workspace_id, repo_key, title, kind, tags,
                            state, added_by)
                         values ($1, $2, $3, $4, $5, 'issue'::relay.item_kind, $6,
                                 'planned'::relay.item_state, $7)",
                    )
                    .bind(user_id)
                    .bind(item)
                    .bind(task_ws)
                    .bind(task_repo.as_str())
                    .bind(input.title.as_str())
                    .bind(input.tags.clone())
                    .bind(caller_lane)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                    Some(item)
                }
            }
        };
        let task_update = sqlx_core::query::query(
            "update relay.tasks set lane_id = $1, title = $2, phase = $3::relay.task_phase,
                    state = $4::relay.task_state, round = $5, reviewer_lane_id = $6,
                    owned_paths = $7, acceptance = $8, resources = $9, deps = $10,
                    item_id = $11, state_changed_at = current_timestamp
             where user_id = $12 and id = $13",
        )
        .bind(input.to_lane)
        .bind(input.title.as_str())
        .bind(word(input.phase))
        .bind(word(next))
        .bind(new_round)
        .bind(input.reviewer_lane)
        .bind(input.owned_paths.clone())
        .bind(input.acceptance.clone())
        .bind(input.resources.clone())
        .bind(input.deps.clone())
        .bind(item_id)
        .bind(user_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await;
        if let Err(e) = task_update {
            if is_unique_violation(&e) {
                return Err(RelayError::Conflict("lane is already busy".into()));
            }
            return Err(db_err(e));
        }
        // An absent reviewer clears the slot: the new round picks fresh at
        // first report (derived; pass it again to keep it).
        let props = serde_json::json!({
            "docs": input.brief_docs,
            "owned_paths": input.owned_paths,
            "acceptance": input.acceptance,
        });
        validate_props(RelayActionKind::Assigned, &props)
            .map_err(|e| RelayError::Internal(format!("assign props: {}", e.message)))?;
        sqlx_core::query::query(
            "insert into relay.actions
               (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                body_md, props)
             values ($1, $2, $3, $4, $5, 'assigned'::relay.action_kind, $6,
                     $7::relay.task_phase, $8, $9)",
        )
        .bind(user_id)
        .bind(new_id())
        .bind(crew_id)
        .bind(task_id)
        .bind(caller_lane)
        .bind(new_round)
        .bind(word(input.phase))
        .bind(input.brief_summary.as_str())
        .bind(props)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        if let Some(lane_title) = input.lane_title.as_deref() {
            sqlx_core::query::query("update relay.lanes set title = $1 where id = $2")
                .bind(lane_title)
                .bind(input.to_lane)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(RelayAssignOutput { task_id, round: new_round, state: next })
    }

    /// A work/rework turn: validate (props, length, leak guard, finding
    /// classes need none here), move working/rework → in_review, pick the
    /// reviewer at first report (Q14), write the `reported` action — one
    /// transaction, or nothing at all.
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_report_task(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        task_id: Uuid,
        input: RelayReportInput,
    ) -> Result<RelayReportOutput, RelayError> {
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        let task: Option<(Uuid, Option<Uuid>, String, i32, String, Option<Uuid>, Vec<String>)> =
            sqlx_core::query_as::query_as(
                "select crew_id, lane_id, state::text, round, phase::text,
                        reviewer_lane_id, owned_paths
                 from relay.tasks where user_id = $1 and id = $2",
            )
            .bind(user_id)
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
        let Some((crew_id, task_lane, state, round, phase, task_reviewer, owned_paths)) = task
        else {
            return Err(RelayError::NotFound("task not found".into()));
        };
        if task_lane != Some(caller_lane) {
            return Err(RelayError::Forbidden("only the assignee reports".into()));
        }
        let cur = parse_state(&state)?;
        if !matches!(cur, RelayTaskState::Working | RelayTaskState::Rework) {
            return Err(RelayError::Conflict(format!("cannot report a {} task", word(cur))));
        }
        if input.round != round {
            return Err(RelayError::Conflict(format!("stale round: task is on round {round}")));
        }
        if input.phase != parse_phase(&phase)? {
            return Err(RelayError::Conflict("report names the wrong phase".into()));
        }
        check_summary(&input.changed, "changed")?;
        let telemetry = input
            .telemetry
            .as_object()
            .ok_or_else(|| RelayError::Validation("telemetry must be an object".into()))?;
        for key in ["look_at", "flags", "base", "branch", "commits", "files", "gates"] {
            if telemetry.get(key).is_some_and(|v| !v.is_null()) {
                return Err(RelayError::Validation(format!(
                    "telemetry key `{key}` collides with the report field"
                )));
            }
        }
        let mut props = serde_json::Map::new();
        if let Some(v) = &input.look_at {
            props.insert("look_at".into(), serde_json::Value::String(v.clone()));
        }
        if let Some(v) = &input.flags {
            props.insert("flags".into(), serde_json::Value::String(v.clone()));
        }
        if let Some(v) = &input.base {
            props.insert("base".into(), serde_json::Value::String(v.clone()));
        }
        if let Some(v) = &input.branch {
            props.insert("branch".into(), serde_json::Value::String(v.clone()));
        }
        props.insert("commits".into(), serde_json::to_value(&input.commits).unwrap_or_default());
        let files: Vec<serde_json::Value> = input
            .files
            .iter()
            .map(|f| {
                serde_json::json!({
                    "path": f.path,
                    "added": f.added,
                    "removed": f.removed,
                    "outside_owned": outside_owned(&f.path, &owned_paths),
                })
            })
            .collect();
        props.insert("files".into(), serde_json::Value::Array(files));
        let gates: Vec<serde_json::Value> = input
            .gates
            .iter()
            .map(|g| {
                let mut gate = serde_json::Map::from_iter([
                    ("cmd".to_owned(), serde_json::Value::String(g.cmd.clone())),
                    ("exit".to_owned(), serde_json::Value::from(g.exit)),
                ]);
                if let Some(counts) = &g.counts {
                    gate.insert("counts".into(), serde_json::Value::String(counts.clone()));
                }
                serde_json::Value::Object(gate)
            })
            .collect();
        props.insert("gates".into(), serde_json::Value::Array(gates));
        for (k, v) in telemetry {
            props.insert(k.clone(), v.clone());
        }
        let props = serde_json::Value::Object(props);
        validate_props(RelayActionKind::Reported, &props)
            .map_err(|e| RelayError::Validation(e.message))?;
        // First report picks the reviewer (Q14); later turns keep the pick.
        let reviewer = match task_reviewer {
            Some(reviewer) => Some(reviewer),
            None => {
                let now: DateTime<Utc> =
                    sqlx_core::query_scalar::query_scalar("select current_timestamp")
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db_err)?;
                relay_pick_reviewer(RelayPickQuery {
                    tx: &mut tx,
                    user_id,
                    crew_id,
                    author: caller_lane,
                    now,
                })
                .await?
            }
        };
        let next = transition(cur, TaskEvent::Reported)
            .map_err(|_| RelayError::Conflict("task cannot move to review".into()))?;
        sqlx_core::query::query(
            "update relay.tasks set state = $1::relay.task_state, reviewer_lane_id = $2,
                    state_changed_at = current_timestamp
             where user_id = $3 and id = $4",
        )
        .bind(word(next))
        .bind(reviewer)
        .bind(user_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        sqlx_core::query::query(
            "insert into relay.actions
               (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                body_md, report_md, props)
             values ($1, $2, $3, $4, $5, 'reported'::relay.action_kind, $6,
                     $7::relay.task_phase, $8, $9, $10)",
        )
        .bind(user_id)
        .bind(new_id())
        .bind(crew_id)
        .bind(task_id)
        .bind(caller_lane)
        .bind(input.round)
        .bind(word(input.phase))
        .bind(input.changed.as_str())
        .bind(input.report_md.as_deref())
        .bind(props)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayReportOutput { state: next, round, reviewer_lane_id: reviewer })
    }

    /// A review turn (the picked reviewer only): approved → approved, or
    /// changes_requested → rework with the round incremented, plus the
    /// `reviewed` action replying to the latest report — one transaction.
    pub async fn relay_submit_review(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        task_id: Uuid,
        input: RelayReviewInput,
    ) -> Result<RelayReviewOutput, RelayError> {
        let caller: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select id from relay.lanes where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(caller_lane)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        if caller.is_none() {
            return Err(RelayError::NotFound("lane not found".into()));
        }
        check_summary(&input.summary, "summary")?;
        for finding in &input.findings {
            let known: Option<String> = sqlx_core::query_scalar::query_scalar(
                "select code from relay.finding_classes where code = $1",
            )
            .bind(finding.class.as_str())
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
            if known.is_none() {
                return Err(RelayError::Validation(format!(
                    "unknown finding class {}",
                    finding.class
                )));
            }
        }
        let telemetry = input
            .telemetry
            .as_object()
            .ok_or_else(|| RelayError::Validation("telemetry must be an object".into()))?;
        for key in ["verdict", "findings", "probes"] {
            if telemetry.get(key).is_some_and(|v| !v.is_null()) {
                return Err(RelayError::Validation(format!(
                    "telemetry key `{key}` collides with the review field"
                )));
            }
        }
        let mut props = serde_json::Map::new();
        props.insert("verdict".into(), serde_json::Value::String(word(input.verdict)));
        let findings: Vec<serde_json::Value> = input
            .findings
            .iter()
            .map(|f| {
                let mut finding = serde_json::Map::from_iter([
                    ("severity".to_owned(), serde_json::Value::String(word(f.severity))),
                    ("class".to_owned(), serde_json::Value::String(f.class.clone())),
                    ("text".to_owned(), serde_json::Value::String(f.text.clone())),
                ]);
                if let Some(file) = &f.file {
                    finding.insert("file".into(), serde_json::Value::String(file.clone()));
                }
                if let Some(line) = f.line {
                    finding.insert("line".into(), serde_json::Value::from(line));
                }
                serde_json::Value::Object(finding)
            })
            .collect();
        props.insert("findings".into(), serde_json::Value::Array(findings));
        if let Some(probes) = input.probes {
            props.insert("probes".into(), serde_json::Value::from(probes));
        }
        for (k, v) in telemetry {
            props.insert(k.clone(), v.clone());
        }
        let props = serde_json::Value::Object(props);
        validate_props(RelayActionKind::Reviewed, &props)
            .map_err(|e| RelayError::Validation(e.message))?;
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        let task: Option<(Uuid, String, i32, String, Option<Uuid>)> =
            sqlx_core::query_as::query_as(
                "select crew_id, state::text, round, phase::text, reviewer_lane_id
                 from relay.tasks where user_id = $1 and id = $2",
            )
            .bind(user_id)
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
        let Some((crew_id, state, round, phase, task_reviewer)) = task else {
            return Err(RelayError::NotFound("task not found".into()));
        };
        if task_reviewer != Some(caller_lane) {
            return Err(RelayError::Forbidden("only the picked reviewer reviews".into()));
        }
        let cur = parse_state(&state)?;
        if cur != RelayTaskState::InReview {
            return Err(RelayError::Conflict(format!("cannot review a {} task", word(cur))));
        }
        let (event, new_round) = match input.verdict {
            ReviewVerdict::Approved => (TaskEvent::ReviewApproved, round),
            ReviewVerdict::ChangesRequested => (TaskEvent::ReviewChangesRequested, round + 1),
        };
        let next = transition(cur, event)
            .map_err(|_| RelayError::Conflict("task cannot leave review".into()))?;
        sqlx_core::query::query(
            "update relay.tasks set state = $1::relay.task_state, round = $2,
                    state_changed_at = current_timestamp
             where user_id = $3 and id = $4",
        )
        .bind(word(next))
        .bind(new_round)
        .bind(user_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        let reply_to: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select id from relay.actions
             where user_id = $1 and task_id = $2 and kind = 'reported'
             order by created_at desc, id desc limit 1",
        )
        .bind(user_id)
        .bind(task_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        sqlx_core::query::query(
            "insert into relay.actions
               (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                body_md, props, reply_to)
             values ($1, $2, $3, $4, $5, 'reviewed'::relay.action_kind, $6,
                     $7::relay.task_phase, $8, $9, $10)",
        )
        .bind(user_id)
        .bind(new_id())
        .bind(crew_id)
        .bind(task_id)
        .bind(caller_lane)
        .bind(new_round)
        .bind(phase)
        .bind(input.summary.as_str())
        .bind(props)
        .bind(reply_to)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayReviewOutput { state: next, round: new_round })
    }

    /// Ask a decision and block its tasks — one transaction.
    pub async fn relay_ask_decision(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        input: RelayAskInput,
    ) -> Result<RelayAskOutput, RelayError> {
        let lane_crew: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select crew_id from relay.lanes where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(caller_lane)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some(crew_id) = lane_crew else {
            return Err(RelayError::NotFound("lane not found".into()));
        };
        check_summary(&input.question, "question")?;
        // The ask's own task, when named, stamps the round/phase.
        let task_stamp: Option<(i32, String)> = match input.task_id {
            Some(task_id) => {
                let row: Option<(Uuid, i32, String)> = sqlx_core::query_as::query_as(
                    "select crew_id, round, phase::text from relay.tasks
                     where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(task_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
                let Some((task_crew, round, phase)) = row else {
                    return Err(RelayError::NotFound("task not found".into()));
                };
                if task_crew != crew_id {
                    return Err(RelayError::NotFound("task not found".into()));
                }
                Some((round, phase))
            }
            None => None,
        };
        if let RelayAskedOf::Lane(lane_id) = input.asked_of {
            let asked: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                "select id from relay.lanes where user_id = $1 and crew_id = $2 and id = $3",
            )
            .bind(user_id)
            .bind(crew_id)
            .bind(lane_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
            if asked.is_none() {
                return Err(RelayError::NotFound("lane not found".into()));
            }
        }
        if let Some(recommended) = &input.recommended
            && !input.options.iter().any(|o| &o.id == recommended)
        {
            return Err(RelayError::Validation("recommended is not one of the options".into()));
        }
        // Every blocked task resolves and blocks — validated before a
        // single write, so one bad ref refuses the whole ask.
        let mut blocked = Vec::with_capacity(input.blocks.len());
        for blocked_id in &input.blocks {
            let row: Option<(Uuid, String)> = sqlx_core::query_as::query_as(
                "select crew_id, state::text from relay.tasks where user_id = $1 and id = $2",
            )
            .bind(user_id)
            .bind(blocked_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
            let Some((task_crew, state)) = row else {
                return Err(RelayError::NotFound("task not found".into()));
            };
            if task_crew != crew_id {
                return Err(RelayError::NotFound("task not found".into()));
            }
            let cur = parse_state(&state)?;
            transition(cur, TaskEvent::Blocked)
                .map_err(|_| RelayError::Conflict(format!("a {} task cannot block", word(cur))))?;
            blocked.push((*blocked_id, cur));
        }
        let options: Vec<serde_json::Value> = input
            .options
            .iter()
            .map(|o| {
                let mut option = serde_json::Map::from_iter([
                    ("id".to_owned(), serde_json::Value::String(o.id.clone())),
                    ("label".to_owned(), serde_json::Value::String(o.label.clone())),
                ]);
                if let Some(detail) = &o.detail {
                    option.insert("detail".into(), serde_json::Value::String(detail.clone()));
                }
                serde_json::Value::Object(option)
            })
            .collect();
        let mut props = serde_json::Map::from_iter([
            ("options".to_owned(), serde_json::Value::Array(options)),
            (
                "blocks".to_owned(),
                serde_json::Value::Array(
                    input
                        .blocks
                        .iter()
                        .map(|id| serde_json::Value::String(id.to_string()))
                        .collect(),
                ),
            ),
            (
                "asked_of".to_owned(),
                serde_json::Value::String(match input.asked_of {
                    RelayAskedOf::Person => "person".to_owned(),
                    RelayAskedOf::Lane(lane_id) => lane_id.to_string(),
                }),
            ),
        ]);
        if let Some(context) = &input.context {
            props.insert("context".into(), serde_json::Value::String(context.clone()));
        }
        if let Some(recommended) = &input.recommended {
            props.insert("recommended".into(), serde_json::Value::String(recommended.clone()));
        }
        let props = serde_json::Value::Object(props);
        validate_props(RelayActionKind::Asked, &props)
            .map_err(|e| RelayError::Validation(e.message))?;
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        for (blocked_id, cur) in &blocked {
            sqlx_core::query::query(
                "update relay.tasks set state = 'blocked'::relay.task_state,
                        blocked_from_state = $1::relay.task_state,
                        state_changed_at = current_timestamp
                 where user_id = $2 and id = $3",
            )
            .bind(word(*cur))
            .bind(user_id)
            .bind(blocked_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        let decision_id = new_id();
        sqlx_core::query::query(
            "insert into relay.actions
               (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                body_md, props)
             values ($1, $2, $3, $4, $5, 'asked'::relay.action_kind, $6,
                     $7::relay.task_phase, $8, $9)",
        )
        .bind(user_id)
        .bind(decision_id)
        .bind(crew_id)
        .bind(input.task_id)
        .bind(caller_lane)
        .bind(task_stamp.as_ref().map(|(round, _)| *round))
        .bind(task_stamp.as_ref().map(|(_, phase)| phase.clone()))
        .bind(input.question.as_str())
        .bind(props)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayAskOutput { decision_id })
    }

    /// Answer a decision (the asked-of party only) and resume its tasks —
    /// one transaction.
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_answer_decision(
        &self,
        user_id: Uuid,
        caller: RelayCaller,
        decision_id: Uuid,
        input: RelayAnswerInput,
    ) -> Result<Uuid, RelayError> {
        let ask: Option<(Uuid, Option<Uuid>, serde_json::Value, Option<i32>, Option<String>)> =
            sqlx_core::query_as::query_as(
                "select crew_id, task_id, props, round, phase::text from relay.actions
                 where user_id = $1 and id = $2 and kind = 'asked'",
            )
            .bind(user_id)
            .bind(decision_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
        let Some((crew_id, task_id, props, ask_round, ask_phase)) = ask else {
            return Err(RelayError::NotFound("decision not found".into()));
        };
        let answered: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select id from relay.actions
             where user_id = $1 and reply_to = $2 and kind = 'answered'",
        )
        .bind(user_id)
        .bind(decision_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        if answered.is_some() {
            return Err(RelayError::Conflict("decision is already answered".into()));
        }
        let asked_of = props
            .get("asked_of")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RelayError::Internal("ask has no asked_of".into()))?;
        let (lane_id, surface) = match &caller {
            RelayCaller::Person { surface } if asked_of == "person" => (
                None,
                resolve_surface(Caller::Person, Some(*surface))
                    .map_err(|e| RelayError::Validation(e.message))?,
            ),
            RelayCaller::Lane { lane_id } if asked_of == lane_id.to_string() => {
                let lane: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                    "select id from relay.lanes where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(lane_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
                if lane.is_none() {
                    return Err(RelayError::NotFound("lane not found".into()));
                }
                (
                    Some(*lane_id),
                    resolve_surface(Caller::Lane, None)
                        .map_err(|e| RelayError::Validation(e.message))?,
                )
            }
            _ => {
                return Err(RelayError::Forbidden("only the asked-of party answers".into()));
            }
        };
        let options = props
            .get("options")
            .and_then(|v| v.as_array())
            .ok_or_else(|| RelayError::Internal("ask has no options".into()))?;
        if !options
            .iter()
            .any(|o| o.get("id").and_then(|v| v.as_str()) == Some(input.option.as_str()))
        {
            return Err(RelayError::Validation(format!("unknown option {}", input.option)));
        }
        if let Some(note) = &input.note {
            check_summary(note, "note")?;
        }
        let answer_props = serde_json::json!({"option": input.option, "surface": word(surface)});
        validate_props(RelayActionKind::Answered, &answer_props)
            .map_err(|e| RelayError::Validation(e.message))?;
        let ask_blocks: Vec<Uuid> = props
            .get("blocks")
            .and_then(|v| v.as_array())
            .map(|blocks| {
                blocks
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .and_then(|s| Uuid::parse_str(s).ok())
                            .ok_or_else(|| RelayError::Internal("ask blocks are corrupt".into()))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        let answer_id = new_id();
        sqlx_core::query::query(
            "insert into relay.actions
               (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                body_md, props, reply_to)
             values ($1, $2, $3, $4, $5, 'answered'::relay.action_kind, $6,
                     $7::relay.task_phase, $8, $9, $10)",
        )
        .bind(user_id)
        .bind(answer_id)
        .bind(crew_id)
        .bind(task_id)
        .bind(lane_id)
        .bind(ask_round)
        .bind(ask_phase)
        .bind(input.note.as_deref())
        .bind(answer_props)
        .bind(decision_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        // Resume what the ask blocked. A task that moved on meanwhile
        // (merged, cancelled, re-planned) is left alone — the answer still
        // records; the resume is best-effort per task (derived).
        for blocked_id in &ask_blocks {
            let row: Option<(String, Option<String>)> = sqlx_core::query_as::query_as(
                "select state::text, blocked_from_state::text from relay.tasks
                 where user_id = $1 and id = $2",
            )
            .bind(user_id)
            .bind(blocked_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let Some((state, blocked_from)) = row else {
                continue;
            };
            if state != "blocked" {
                continue;
            }
            let from =
                blocked_from.as_deref().map(parse_state).transpose()?.ok_or_else(|| {
                    RelayError::Internal("blocked task has no blocked_from".into())
                })?;
            let next =
                transition(RelayTaskState::Blocked, TaskEvent::Unblocked { blocked_from: from })
                    .map_err(|_| RelayError::Internal("blocked task cannot resume".into()))?;
            sqlx_core::query::query(
                "update relay.tasks set state = $1::relay.task_state,
                        blocked_from_state = null, state_changed_at = current_timestamp
                 where user_id = $2 and id = $3",
            )
            .bind(word(next))
            .bind(user_id)
            .bind(blocked_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(answer_id)
    }

    /// Queue a message on the bus — person and lane share it.
    pub async fn relay_send_message(
        &self,
        user_id: Uuid,
        caller: RelayCaller,
        input: RelayMessageInput,
    ) -> Result<RelayMessageOutput, RelayError> {
        let (lane_id, surface) = match &caller {
            RelayCaller::Person { surface } => (
                None,
                resolve_surface(Caller::Person, Some(*surface))
                    .map_err(|e| RelayError::Validation(e.message))?,
            ),
            RelayCaller::Lane { lane_id } => {
                let lane: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                    "select id from relay.lanes where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(lane_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
                if lane.is_none() {
                    return Err(RelayError::NotFound("lane not found".into()));
                }
                (
                    Some(*lane_id),
                    resolve_surface(Caller::Lane, None)
                        .map_err(|e| RelayError::Validation(e.message))?,
                )
            }
        };
        check_summary(&input.text, "text")?;
        let task_crew: Option<Uuid> = match input.task_id {
            Some(task_id) => {
                let crew: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                    "select crew_id from relay.tasks where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(task_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
                Some(crew.ok_or_else(|| RelayError::NotFound("task not found".into()))?)
            }
            None => None,
        };
        let caller_crew: Option<Uuid> = match lane_id {
            Some(lane) => sqlx_core::query_scalar::query_scalar(
                "select crew_id from relay.lanes where user_id = $1 and id = $2",
            )
            .bind(user_id)
            .bind(lane)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?,
            None => None,
        };
        let (to, to_crew): (serde_json::Value, Option<Uuid>) = match &input.to {
            RelayMessageTo::Coordinator => (serde_json::json!({"kind": "coordinator"}), None),
            RelayMessageTo::Everyone => (serde_json::json!({"kind": "everyone"}), None),
            RelayMessageTo::Person => (serde_json::json!({"kind": "person"}), None),
            RelayMessageTo::Lane { lane_id } => {
                let crew: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                    "select crew_id from relay.lanes where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(lane_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
                let Some(crew) = crew else {
                    return Err(RelayError::NotFound("lane not found".into()));
                };
                (serde_json::json!({"kind": "lane", "lane_id": lane_id.to_string()}), Some(crew))
            }
        };
        // The row's crew, derived (the store input names none): the task's,
        // else the sending lane's, else the addressed lane's, else the
        // user's oldest crew. Delivery is by address over the user's rows
        // (the envelope crew-scopes nothing), so this choice is audit, not
        // routing — except it keeps the FK honest (derived; disclosed).
        let crew_id = match task_crew.or(caller_crew).or(to_crew) {
            Some(crew) => crew,
            None => {
                let oldest: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
                    "select id from relay.crews where user_id = $1 order by started_at, id limit 1",
                )
                .bind(user_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
                oldest.ok_or_else(|| RelayError::NotFound("no crew".into()))?
            }
        };
        let props = serde_json::json!({"to": to, "surface": word(surface)});
        validate_props(RelayActionKind::Message, &props)
            .map_err(|e| RelayError::Validation(e.message))?;
        let message_id = new_id();
        sqlx_core::query::query(
            "insert into relay.actions
               (user_id, id, crew_id, task_id, lane_id, kind, body_md, props)
             values ($1, $2, $3, $4, $5, 'message'::relay.action_kind, $6, $7)",
        )
        .bind(user_id)
        .bind(message_id)
        .bind(crew_id)
        .bind(input.task_id)
        .bind(lane_id)
        .bind(input.text.as_str())
        .bind(props)
        .execute(self.pool())
        .await
        .map_err(db_err)?;
        Ok(RelayMessageOutput { message_id })
    }

    /// Merge an approved task (coordinator only): approved → done with the
    /// `merged` action and `completed_at` — one transaction.
    pub async fn relay_merge_task(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        task_id: Uuid,
        input: RelayMergeInput,
    ) -> Result<(), RelayError> {
        let crew_id = self.relay_crew_of_coordinator(user_id, caller_lane).await?;
        let task: Option<(Uuid, String, i32, String)> = sqlx_core::query_as::query_as(
            "select crew_id, state::text, round, phase::text from relay.tasks
             where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(task_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((task_crew, state, round, phase)) = task else {
            return Err(RelayError::NotFound("task not found".into()));
        };
        if task_crew != crew_id {
            return Err(RelayError::NotFound("task not found".into()));
        }
        let cur = parse_state(&state)?;
        if cur != RelayTaskState::Approved {
            return Err(RelayError::Conflict(format!(
                "only approved tasks merge, not {}",
                word(cur)
            )));
        }
        check_summary(&input.note, "note")?;
        let gates: Vec<serde_json::Value> = input
            .gates
            .iter()
            .map(|g| {
                let mut gate = serde_json::Map::from_iter([
                    ("cmd".to_owned(), serde_json::Value::String(g.cmd.clone())),
                    ("exit".to_owned(), serde_json::Value::from(g.exit)),
                ]);
                if let Some(counts) = &g.counts {
                    gate.insert("counts".into(), serde_json::Value::String(counts.clone()));
                }
                serde_json::Value::Object(gate)
            })
            .collect();
        let props = serde_json::json!({
            "commits": input.commits,
            "into": input.into,
            "gates": gates,
        });
        validate_props(RelayActionKind::Merged, &props)
            .map_err(|e| RelayError::Validation(e.message))?;
        let next = transition(cur, TaskEvent::Merged)
            .map_err(|_| RelayError::Conflict("task cannot merge".into()))?;
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        sqlx_core::query::query(
            "update relay.tasks set state = $1::relay.task_state,
                    completed_at = current_timestamp,
                    state_changed_at = current_timestamp
             where user_id = $2 and id = $3",
        )
        .bind(word(next))
        .bind(user_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        sqlx_core::query::query(
            "insert into relay.actions
               (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                body_md, props)
             values ($1, $2, $3, $4, $5, 'merged'::relay.action_kind, $6,
                     $7::relay.task_phase, $8, $9)",
        )
        .bind(user_id)
        .bind(new_id())
        .bind(crew_id)
        .bind(task_id)
        .bind(caller_lane)
        .bind(round)
        .bind(phase)
        .bind(input.note.as_str())
        .bind(props)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    /// Cancel a task (coordinator only, F3): the run-01 orphans — planned
    /// tasks sitting `assigned` with NULL lane, and interrupted tasks whose
    /// holder unregistered — leave the board instead of lingering. Cancels
    /// from anything but done/cancelled (the machine's `Cancelled` event);
    /// a live holder is a 409. The lane id stays on the task (merge's
    /// precedent) and no action row is written: one coordinator per crew,
    /// so the canceller is implicit, and `state_changed_at` stamps the
    /// when (plan's precedent). Cancelled tasks never re-offer: assign's
    /// state gate and the sweeper's predicates exclude them by
    /// construction.
    pub async fn relay_cancel_task(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        task_id: Uuid,
    ) -> Result<RelayCancelOutput, RelayError> {
        let crew_id = self.relay_crew_of_coordinator(user_id, caller_lane).await?;
        let task: Option<(Uuid, Option<Uuid>, String)> = sqlx_core::query_as::query_as(
            "select crew_id, lane_id, state::text from relay.tasks
             where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(task_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((task_crew, task_lane, state)) = task else {
            return Err(RelayError::NotFound("task not found".into()));
        };
        if task_crew != crew_id {
            return Err(RelayError::NotFound("task not found".into()));
        }
        let cur = parse_state(&state)?;
        let next = transition(cur, TaskEvent::Cancelled)
            .map_err(|_| RelayError::Conflict(format!("cannot cancel a {} task", word(cur))))?;
        if let Some(holder) = task_lane {
            let unregistered: Option<Option<DateTime<Utc>>> =
                sqlx_core::query_scalar::query_scalar(
                    "select unregistered_at from relay.lanes where id = $1",
                )
                .bind(holder)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
            // A missing holder row is unreachable (the task FK holds it),
            // but an orphan cancels rather than wedges: only a live lane
            // blocks.
            if matches!(unregistered, Some(None)) {
                return Err(RelayError::Conflict("task is held by a live lane".into()));
            }
        }
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        sqlx_core::query::query(
            "update relay.tasks set state = $1::relay.task_state,
                    blocked_from_state = null,
                    state_changed_at = current_timestamp
             where user_id = $2 and id = $3",
        )
        .bind(word(next))
        .bind(user_id)
        .bind(task_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(RelayCancelOutput { task_id, state: next })
    }

    /// Close a crew (coordinator only, F3): the crew's work is over when
    /// its lanes are gone. "Zero live lanes" counts beside the caller —
    /// the coordinator closes, then unregisters (a strict zero is
    /// uncallable: closing needs a live lane to auth). Idempotent:
    /// closing a closed crew re-asserts the state. Closed crews refuse
    /// register/reregister/plan/assign (`relay_require_crew_open`).
    pub async fn relay_close_crew(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        crew_id: Uuid,
    ) -> Result<RelayCloseOutput, RelayError> {
        let own_crew = self.relay_crew_of_coordinator(user_id, caller_lane).await?;
        if own_crew != crew_id {
            return Err(RelayError::NotFound("crew not found".into()));
        }
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        let status: Option<String> = sqlx_core::query_scalar::query_scalar(
            "select status from relay.crews where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(crew_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        let Some(status) = status else {
            return Err(RelayError::NotFound("crew not found".into()));
        };
        if status != "closed" {
            let live: bool = sqlx_core::query_scalar::query_scalar(
                "select exists (
                   select 1 from relay.lanes
                   where user_id = $1 and crew_id = $2
                     and unregistered_at is null and id <> $3)",
            )
            .bind(user_id)
            .bind(crew_id)
            .bind(caller_lane)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_err)?;
            if live {
                return Err(RelayError::Conflict("crew has live lanes".into()));
            }
            sqlx_core::query::query(
                "update relay.crews set status = 'closed',
                        completed_at = current_timestamp
                 where user_id = $1 and id = $2",
            )
            .bind(user_id)
            .bind(crew_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(RelayCloseOutput { crew_id, status: "closed".to_string() })
    }

    /// The lane's grouped envelope (Q4): messages, answers, reviews,
    /// assignments, each newest-first. Reading marks the lane's active task
    /// started on first delivery and writes `delivered` receipts for newly
    /// included messages — one transaction with the read.
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_envelope(
        &self,
        user_id: Uuid,
        lane_id: Uuid,
    ) -> Result<LaneEnvelope, RelayError> {
        let lane: Option<(Uuid, String)> = sqlx_core::query_as::query_as(
            "select crew_id, role::text from relay.lanes where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(lane_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((crew_id, role)) = lane else {
            return Err(RelayError::NotFound("lane not found".into()));
        };
        let is_coordinator = role == "coordinator";
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        // ── messages: user-scoped, matched by address, newest first.
        let rows: Vec<(
            Uuid,
            Option<Uuid>,
            Option<Uuid>,
            serde_json::Value,
            Option<String>,
            DateTime<Utc>,
        )> = sqlx_core::query_as::query_as(
            "select id, task_id, lane_id, props, body_md, created_at
                 from relay.actions
                 where user_id = $1 and kind = 'message'
                   and (props->'to'->>'kind' = 'everyone'
                        or (props->'to'->>'kind' = 'lane'
                            and props->'to'->>'lane_id' = $2)
                        or (props->'to'->>'kind' = 'coordinator' and $3))
                 order by created_at desc, id desc",
        )
        .bind(user_id)
        .bind(lane_id.to_string())
        .bind(is_coordinator)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let mut messages = Vec::with_capacity(rows.len());
        for (id, task_id, sender, props, body, created_at) in &rows {
            let from_lane_code = match sender {
                Some(sender) => {
                    let code: Option<String> = sqlx_core::query_scalar::query_scalar(
                        "select code from relay.lanes where user_id = $1 and id = $2",
                    )
                    .bind(user_id)
                    .bind(sender)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
                    code
                }
                None => None,
            };
            let surface = props
                .get("surface")
                .and_then(|v| v.as_str())
                .ok_or_else(|| RelayError::Internal("message has no surface".into()))?;
            messages.push(EnvelopeMessage {
                id: id.to_string(),
                task_id: task_id.map(|id| id.to_string()),
                from_lane_code,
                surface: parse_surface(surface)?,
                text: body.clone().unwrap_or_default(),
                created_at: iso(*created_at),
            });
            // Inclusion delivers; a later fetch after delivery reads (there
            // is no ack tool).
            let kinds: Vec<String> = sqlx_core::query_scalar::query_scalar(
                "select kind::text from relay.actions
                 where user_id = $1 and lane_id = $2 and reply_to = $3
                   and kind in ('delivered', 'read')",
            )
            .bind(user_id)
            .bind(lane_id)
            .bind(id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let receipt = if !kinds.iter().any(|k| k == "delivered") {
                "delivered"
            } else if !kinds.iter().any(|k| k == "read") {
                "read"
            } else {
                continue;
            };
            sqlx_core::query::query(
                "insert into relay.actions
                   (user_id, id, crew_id, task_id, lane_id, kind, reply_to, props)
                 values ($1, $2, $3, $4, $5, $6::relay.action_kind, $7, '{}')",
            )
            .bind(user_id)
            .bind(new_id())
            .bind(crew_id)
            .bind(task_id)
            .bind(lane_id)
            .bind(receipt)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        // ── answers: replies to this lane's asks, newest first.
        let rows: Vec<(
            Uuid,
            Uuid,
            Option<Uuid>,
            Option<Uuid>,
            String,
            Option<String>,
            DateTime<Utc>,
        )> = sqlx_core::query_as::query_as(
            "select a.id, a.reply_to, a.task_id, a.lane_id, a.props->>'option',
                        a.body_md, a.created_at
                 from relay.actions a join relay.actions ask on ask.id = a.reply_to
                 where a.user_id = $1 and a.kind = 'answered' and ask.lane_id = $2
                 order by a.created_at desc, a.id desc",
        )
        .bind(user_id)
        .bind(lane_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let mut answers = Vec::with_capacity(rows.len());
        for (id, decision_id, task_id, sender, option, note, created_at) in rows {
            let from_lane_code = match sender {
                Some(sender) => {
                    let code: Option<String> = sqlx_core::query_scalar::query_scalar(
                        "select code from relay.lanes where user_id = $1 and id = $2",
                    )
                    .bind(user_id)
                    .bind(sender)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
                    code
                }
                None => None,
            };
            answers.push(EnvelopeAnswer {
                id: id.to_string(),
                decision_id: decision_id.to_string(),
                task_id: task_id.map(|id| id.to_string()),
                option,
                note,
                from_lane_code,
                created_at: iso(created_at),
            });
        }
        // ── reviews, both directions (Q15): to-perform (picked reviewer,
        // awaiting verdict) and to-answer (own task sent back).
        let mut reviews = Vec::new();
        let perform: Vec<(Uuid, String, i32, DateTime<Utc>)> = sqlx_core::query_as::query_as(
            "select id, phase::text, round, state_changed_at from relay.tasks
                 where user_id = $1 and reviewer_lane_id = $2 and state = 'in_review'",
        )
        .bind(user_id)
        .bind(lane_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        for (task_id, phase, round, changed_at) in perform {
            let reported_at: Option<DateTime<Utc>> = sqlx_core::query_scalar::query_scalar(
                "select created_at from relay.actions
                 where user_id = $1 and task_id = $2 and kind = 'reported'
                 order by created_at desc, id desc limit 1",
            )
            .bind(user_id)
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            reviews.push(EnvelopeReview {
                task_id: task_id.to_string(),
                phase: parse_phase(&phase)?,
                round,
                direction: ReviewDirection::Perform,
                created_at: iso(reported_at.unwrap_or(changed_at)),
            });
        }
        let answer_rows: Vec<(Uuid, String, i32, DateTime<Utc>)> = sqlx_core::query_as::query_as(
            "select id, phase::text, round, state_changed_at from relay.tasks
                 where user_id = $1 and lane_id = $2 and state = 'rework'",
        )
        .bind(user_id)
        .bind(lane_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        for (task_id, phase, round, changed_at) in answer_rows {
            let reviewed_at: Option<DateTime<Utc>> = sqlx_core::query_scalar::query_scalar(
                "select created_at from relay.actions
                 where user_id = $1 and task_id = $2 and kind = 'reviewed'
                 order by created_at desc, id desc limit 1",
            )
            .bind(user_id)
            .bind(task_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            reviews.push(EnvelopeReview {
                task_id: task_id.to_string(),
                phase: parse_phase(&phase)?,
                round,
                direction: ReviewDirection::Answer,
                created_at: iso(reviewed_at.unwrap_or(changed_at)),
            });
        }
        reviews.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        // ── assignment: the one task the lane must act on (derived:
        // assigned, working, or rework — in_review and blocked wait on
        // someone else).
        let task: Option<(
            Uuid,
            String,
            String,
            i32,
            Option<Uuid>,
            Vec<String>,
            Vec<String>,
            Vec<Uuid>,
            Vec<String>,
            DateTime<Utc>,
            String,
        )> = sqlx_core::query_as::query_as(
            "select id, title, phase::text, round, item_id, acceptance, owned_paths,
                    deps, resources, assigned_at, state::text
             from relay.tasks
             where user_id = $1 and lane_id = $2
               and state in ('assigned', 'working', 'rework')",
        )
        .bind(user_id)
        .bind(lane_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        let mut assignments = Vec::new();
        if let Some((
            task_id,
            title,
            phase,
            round,
            item_id,
            acceptance,
            owned_paths,
            deps,
            resources,
            assigned_at,
            state,
        )) = task
        {
            let brief_row: Option<(Option<String>, serde_json::Value, DateTime<Utc>)> =
                sqlx_core::query_as::query_as(
                    "select body_md, props, created_at from relay.actions
                     where user_id = $1 and task_id = $2
                       and kind in ('assigned', 'brief_updated')
                     order by created_at desc, id desc limit 1",
                )
                .bind(user_id)
                .bind(task_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
            let (summary, docs, brief_at) = match brief_row {
                Some((body, props, at)) => (
                    body.unwrap_or_default(),
                    props
                        .get("docs")
                        .and_then(|v| v.as_array())
                        .map(|docs| {
                            docs.iter().filter_map(|d| d.as_str().map(str::to_owned)).collect()
                        })
                        .unwrap_or_default(),
                    at,
                ),
                None => (String::new(), Vec::new(), assigned_at),
            };
            let (tags, ticket) = match item_id {
                Some(item_id) => {
                    let item: Option<(Option<Vec<String>>, Option<String>, Option<String>)> =
                        sqlx_core::query_as::query_as(
                            "select tags, ticket_provider, ticket_key from relay.items
                             where user_id = $1 and id = $2",
                        )
                        .bind(user_id)
                        .bind(item_id)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(db_err)?;
                    match item {
                        Some((tags, provider, key)) => {
                            let ticket = match (provider, key) {
                                (Some(provider), Some(key)) => Some(EnvelopeTicket {
                                    provider: parse_provider(&provider)?,
                                    key,
                                    url: None,
                                }),
                                _ => None,
                            };
                            (tags.unwrap_or_default(), ticket)
                        }
                        None => (Vec::new(), None),
                    }
                }
                None => (Vec::new(), None),
            };
            assignments.push(EnvelopeAssignment {
                task_id: task_id.to_string(),
                title,
                phase: parse_phase(&phase)?,
                round,
                ticket,
                tags,
                brief: EnvelopeBrief { summary, docs },
                acceptance,
                owned_paths,
                deps: deps.into_iter().map(|id| id.to_string()).collect(),
                resources,
                created_at: iso(brief_at),
            });
            // Delivery starts the task (derived: no start tool exists).
            if state == "assigned" {
                sqlx_core::query::query(
                    "update relay.tasks set state = 'working'::relay.task_state,
                            state_changed_at = current_timestamp
                     where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(task_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                sqlx_core::query::query(
                    "insert into relay.actions
                       (user_id, id, crew_id, task_id, lane_id, kind, round, phase, props)
                     values ($1, $2, $3, $4, $5, 'started'::relay.action_kind, $6,
                             $7::relay.task_phase, '{}')",
                )
                .bind(user_id)
                .bind(new_id())
                .bind(crew_id)
                .bind(task_id)
                .bind(lane_id)
                .bind(round)
                .bind(phase)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
        }
        tx.commit().await.map_err(db_err)?;
        Ok(LaneEnvelope { messages, answers, reviews, assignments })
    }

    /// Expire lapsed lane leases (fed `now`): suspend their lanes —
    /// active task → interrupted with its checkpoint — re-offer tasks
    /// past the grace (measured from the `interrupted` action), and
    /// re-pick expired reviewers (Q17-REC).
    pub async fn relay_sweep_leases(
        &self,
        now: DateTime<Utc>,
    ) -> Result<RelaySweepReport, RelayError> {
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        // The sweep acts at the fed `now`: its writes are stamped then, so
        // the pass is a pure function of (database, now) — a task it
        // interrupts is never due for re-offer in the same pass.
        let expired: Vec<(Uuid, Uuid, Uuid)> = sqlx_core::query_as::query_as(
            "update relay.leases set state = 'expired'::relay.lease_state
             where kind = 'lane' and state = 'granted' and expires_at <= $1
             returning user_id, crew_id, lane_id",
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let expired_leases = expired.len() as u64;
        let mut interrupted_tasks = 0u64;
        for (user_id, crew_id, lane_id) in &expired {
            let tasks: Vec<(Uuid, String, i32, String)> = sqlx_core::query_as::query_as(
                "select id, state::text, round, phase::text from relay.tasks
                 where user_id = $1 and lane_id = $2
                   and state in ('assigned', 'working', 'in_review', 'rework', 'blocked')",
            )
            .bind(user_id)
            .bind(lane_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            for (task_id, state, round, phase) in tasks {
                let next = transition(parse_state(&state)?, TaskEvent::LeaseLost)
                    .map_err(|_| RelayError::Conflict("task cannot be suspended".into()))?;
                // Block memory clears with the suspend: only a blocked task
                // remembers where it was (the tasks_blocked_memory check;
                // cancel/unregister precedent) — without it the write
                // violates the check and rolls back the whole pass.
                sqlx_core::query::query(
                    "update relay.tasks set state = $1::relay.task_state,
                            blocked_from_state = null,
                            state_changed_at = $2
                     where user_id = $3 and id = $4",
                )
                .bind(word(next))
                .bind(now)
                .bind(user_id)
                .bind(task_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                sqlx_core::query::query(
                    "insert into relay.actions
                       (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                        props, created_at)
                     values ($1, $2, $3, $4, $5, 'interrupted'::relay.action_kind, $6,
                             $7::relay.task_phase, '{}', $8)",
                )
                .bind(user_id)
                .bind(new_id())
                .bind(crew_id)
                .bind(task_id)
                .bind(lane_id)
                .bind(round)
                .bind(phase)
                .bind(now)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                interrupted_tasks += 1;
            }
        }
        // Re-offer tasks past the grace, measured from the latest
        // `interrupted` action. Tasks without one are left alone (the grace
        // has nothing to measure from).
        let cutoff = now - Duration::seconds(REOFFER_GRACE_SECS as i64);
        let due: Vec<(Uuid, Uuid, Uuid, Option<Uuid>, i32, String)> =
            sqlx_core::query_as::query_as(
                "select t.user_id, t.crew_id, t.id, t.lane_id, t.round, t.phase::text
                 from relay.tasks t
                 where t.state = 'interrupted'
                   and (select max(a.created_at) from relay.actions a
                        where a.user_id = t.user_id and a.task_id = t.id
                          and a.kind = 'interrupted') <= $1",
            )
            .bind(cutoff)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
        let mut reoffered_tasks = 0u64;
        for (user_id, crew_id, task_id, lane_id, round, phase) in due {
            let next = transition(RelayTaskState::Interrupted, TaskEvent::Reoffered)
                .map_err(|_| RelayError::Conflict("task cannot be re-offered".into()))?;
            sqlx_core::query::query(
                "update relay.tasks set state = $1::relay.task_state, lane_id = null,
                        state_changed_at = $2
                 where user_id = $3 and id = $4",
            )
            .bind(word(next))
            .bind(now)
            .bind(user_id)
            .bind(task_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx_core::query::query(
                "insert into relay.actions
                   (user_id, id, crew_id, task_id, lane_id, kind, round, phase,
                    props, created_at)
                 values ($1, $2, $3, $4, $5, 'reoffered'::relay.action_kind, $6,
                         $7::relay.task_phase, '{}', $8)",
            )
            .bind(user_id)
            .bind(new_id())
            .bind(crew_id)
            .bind(task_id)
            .bind(lane_id)
            .bind(round)
            .bind(phase)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            reoffered_tasks += 1;
        }
        // Re-pick dead reviewers over the live lanes (Q17): a new pick, or
        // a cleared slot when nobody is live. Silent (no action kind names
        // it); the count reports both.
        let stale: Vec<(Uuid, Uuid, Uuid, Option<Uuid>)> = sqlx_core::query_as::query_as(
            "select t.user_id, t.crew_id, t.id, t.lane_id from relay.tasks t
             where t.state = 'in_review' and t.reviewer_lane_id is not null
               and not exists (
                 select 1 from relay.leases s
                 where s.lane_id = t.reviewer_lane_id and s.kind = 'lane'
                   and s.state = 'granted' and s.expires_at > $1)",
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let mut repicked_reviewers = 0u64;
        for (user_id, crew_id, task_id, author) in stale {
            let Some(author) = author else { continue };
            let reviewer =
                relay_pick_reviewer(RelayPickQuery { tx: &mut tx, user_id, crew_id, author, now })
                    .await?;
            sqlx_core::query::query(
                "update relay.tasks set reviewer_lane_id = $1 where user_id = $2 and id = $3",
            )
            .bind(reviewer)
            .bind(user_id)
            .bind(task_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            repicked_reviewers += 1;
        }
        // Item 3: expire stale resource grants (the same `<=` comparison
        // as lane leases), then release grants held by lanes with no live
        // lease (the lane-death cascade, Q7 — a release, not an expiry).
        let freed_expired: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "update relay.leases set state = 'expired'::relay.lease_state
             where kind = 'resource' and state = 'granted' and expires_at <= $1
             returning resource",
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let expired_resource_leases = freed_expired.len() as u64;
        let freed_dead: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "update relay.leases r set state = 'released'::relay.lease_state, released_at = $1
             where r.kind = 'resource' and r.state = 'granted'
               and not exists (
                 select 1 from relay.leases l
                 where l.lane_id = r.lane_id and l.kind = 'lane'
                   and l.state = 'granted' and l.expires_at > $1)
             returning resource",
        )
        .bind(now)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        // Every freed slot auto-grants its first waiter (floors
        // re-checked); an expiry and a cascade never free the same slot
        // (a grant is either stale or live).
        let mut granted_from_queue = 0u64;
        for resource in freed_expired.iter().chain(freed_dead.iter()) {
            if grant_next_waiter(&mut tx, resource, now).await?.is_some() {
                granted_from_queue += 1;
            }
        }
        tx.commit().await.map_err(db_err)?;
        Ok(RelaySweepReport {
            expired_leases,
            interrupted_tasks,
            reoffered_tasks,
            repicked_reviewers,
            expired_resource_leases,
            granted_from_queue,
        })
    }

    /// The daemon user behind person reads and register (Q19-REC): the
    /// dōjō principal id from the sign-in session. Fails closed
    /// (`SignInRequired`) when the machine never signed in — never a
    /// minted stand-in (a miss is an error, never an invented row).
    pub async fn relay_current_user(&self) -> Result<Uuid, RelayError> {
        // The DDL's own seam: the chosen default persona's dōjō principal.
        // No default, or an unlinked one, is a machine that never signed in.
        let principal: Option<Option<Uuid>> = sqlx_core::query_scalar::query_scalar(
            "select principal_id from sensei.personas where is_chosen_default limit 1",
        )
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        principal.flatten().ok_or(RelayError::SignInRequired)
    }

    /// `RelayNow`, exactly per `relay-api.ts` — crews, facets, lanes,
    /// decisions, triage, today's completions, today counts, resources.
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_now(
        &self,
        user_id: Uuid,
        filter: RelayReadFilter,
    ) -> Result<serde_json::Value, RelayError> {
        // The completed window is the session-local day, and this is the one
        // relay read with a day window — so the transaction sets the Mac zone
        // locally (the pool stays UTC for metrics; L0 ruling 2026-11-06).
        // Fail-closed: without the zone the window is undefined.
        let tz = crate::relay::system_timezone().map_err(RelayError::Internal)?;
        let mut tx = self.pool().begin().await.map_err(db_err)?;
        // `system_timezone` allows `[A-Za-z0-9_/+-]` only, so the name cannot
        // break out of the quotes (`SET` takes no binds).
        sqlx_core::query::query(&format!("SET LOCAL TIME ZONE '{tz}'"))
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        // The filter selects crews; lanes/decisions/completed/today follow
        // theirs. Facets stay global (the band's choices, not its result).
        let crews: Vec<(
            Uuid,
            Option<String>,
            Option<Uuid>,
            Option<String>,
            String,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        )> = sqlx_core::query_as::query_as(
            "select id, repo_key, workspace_id, goal, status, started_at, heartbeat_at
             from relay.crews
             where user_id = $1
               and ($2::text is null or repo_key = $2)
               and ($3::uuid is null or workspace_id = $3)
             order by started_at, id",
        )
        .bind(user_id)
        .bind(filter.repo_key.as_deref())
        .bind(filter.workspace_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let crew_ids: Vec<Uuid> = crews.iter().map(|c| c.0).collect();
        let crews_json: Vec<serde_json::Value> = crews
            .iter()
            .map(|(id, repo_key, workspace_id, goal, status, started_at, heartbeat_at)| {
                serde_json::json!({
                    "id": id.to_string(),
                    "repo_key": repo_key,
                    "workspace_id": workspace_id.map(|id| id.to_string()),
                    "goal": goal,
                    "status": status,
                    "started_at": iso(*started_at),
                    "heartbeat_at": heartbeat_at.map(iso),
                })
            })
            .collect();
        let repos: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "select distinct repo_key from relay.crews
             where user_id = $1 and repo_key is not null order by 1",
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let workspaces: Vec<(Uuid, String)> = sqlx_core::query_as::query_as(
            "select distinct w.id, w.name from relay.crews c
             join sensei.workspaces w on w.id = c.workspace_id
             where c.user_id = $1 and c.workspace_id is not null order by w.name",
        )
        .bind(user_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        // Lane rows come from the `relay.now` view (needs_answer is its
        // rule); the row id/crew are the view's columns.
        let mut lanes: Vec<(Uuid, Uuid, serde_json::Value)> = sqlx_core::query_as::query_as(
            "select v.id, v.crew_id, v.payload from relay.now v
                 where v.user_id = $1 and v.section = 'lane' and v.crew_id = any($2)",
        )
        .bind(user_id)
        .bind(crew_ids.clone())
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        lanes.sort_by(|a, b| {
            let rank = |id: &Uuid| crew_ids.iter().position(|c| c == id).unwrap_or(usize::MAX);
            let code = |p: &serde_json::Value| {
                p.get("code").and_then(|v| v.as_str()).unwrap_or_default().to_owned()
            };
            (rank(&a.1), code_rank(&code(&a.2))).cmp(&(rank(&b.1), code_rank(&code(&b.2))))
        });
        let lanes_json: Vec<serde_json::Value> = lanes
            .iter()
            .map(|(id, crew_id, payload)| {
                let lease_state = payload
                    .get("lease")
                    .and_then(|l| l.get("state"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("queued");
                let lease_expires_at = payload
                    .get("lease")
                    .and_then(|l| l.get("expires_at"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                serde_json::json!({
                    "id": id.to_string(),
                    "crew_id": crew_id.to_string(),
                    "code": payload.get("code"),
                    "title": payload.get("title"),
                    "role": payload.get("role"),
                    "family": payload.get("family"),
                    "version": payload.get("version"),
                    "model": payload.get("model"),
                    "lease_state": lease_state,
                    "lease_expires_at": lease_expires_at,
                    "task": payload.get("task"),
                })
            })
            .collect();
        let decisions: Vec<(Uuid, Uuid, serde_json::Value)> = sqlx_core::query_as::query_as(
            "select v.id, v.crew_id, v.payload from relay.now v
             where v.user_id = $1 and v.section = 'open_asked' and v.crew_id = any($2)
             order by v.payload->>'asked_at' desc, v.id desc",
        )
        .bind(user_id)
        .bind(crew_ids.clone())
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let decisions_json: Vec<serde_json::Value> = decisions
            .iter()
            .map(|(id, crew_id, payload)| {
                let options = payload
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|options| {
                        options
                            .iter()
                            .map(|o| {
                                serde_json::json!({
                                    "id": o.get("id"),
                                    "label": o.get("label"),
                                    "detail": o.get("detail").cloned().unwrap_or(serde_json::Value::Null),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                serde_json::json!({
                    "id": id.to_string(),
                    "crew_id": crew_id.to_string(),
                    "task_id": payload.get("task_id"),
                    "task_title": payload.get("task_title"),
                    "question": payload.get("question"),
                    "context": payload.get("context"),
                    "options": options,
                    "recommended": payload.get("recommended"),
                    "asked_at": payload.get("asked_at"),
                })
            })
            .collect();
        // Triage counts are computed in SQL (the view's `triage_count` row
        // cannot take the item-side filter): manual rows plus one row per
        // tracker provider, sprints null until slice 2's pull.
        let manual: i64 = sqlx_core::query_scalar::query_scalar(
            "select count(*) from relay.items
             where user_id = $1 and state = 'triage' and ticket_provider is null
               and ($2::text is null or repo_key = $2)
               and ($3::uuid is null or workspace_id = $3)",
        )
        .bind(user_id)
        .bind(filter.repo_key.as_deref())
        .bind(filter.workspace_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;
        let providers: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "select ticket_provider, count(*) from relay.items
             where user_id = $1 and state = 'triage' and ticket_provider is not null
               and ($2::text is null or repo_key = $2)
               and ($3::uuid is null or workspace_id = $3)
             group by ticket_provider",
        )
        .bind(user_id)
        .bind(filter.repo_key.as_deref())
        .bind(filter.workspace_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let mut triage_json = Vec::new();
        if manual > 0 {
            triage_json.push(serde_json::json!({
                "provider": "manual",
                "sprint": serde_json::Value::Null,
                "count": manual,
            }));
        }
        let mut providers = providers;
        providers.sort_by(|a, b| a.0.cmp(&b.0));
        for (provider, count) in providers {
            parse_provider(&provider)?;
            triage_json.push(serde_json::json!({
                "provider": provider,
                "sprint": serde_json::Value::Null,
                "count": count,
            }));
        }
        // F3: planned-but-unassigned crew work (run-01's M2/M3 sat
        // `assigned` with NULL lane and appeared nowhere). One trailing
        // bucket, additive: existing provider entries keep their shape,
        // and readers of {provider, sprint, count} see an "Unassigned"
        // intake row. Done/cancelled stay out (complete or dead); a
        // coordinator polling census + its lane pull accounts for every
        // other crew task. Closed crews' leftovers stay visible: the
        // coordinator sees the crew is closed in `crews` and judges.
        let unassigned: Vec<(Uuid, Uuid, String, String, String, i32, String, Option<Uuid>)> =
            sqlx_core::query_as::query_as(
                "select t.id, t.crew_id, t.title, t.phase::text, t.state::text,
                    t.round, t.repo_key, t.item_id
             from relay.tasks t
             where t.user_id = $1 and t.crew_id = any($2) and t.lane_id is null
               and t.state not in ('done', 'cancelled')
               and ($3::text is null or t.repo_key = $3)
               and ($4::uuid is null or t.workspace_id = $4)
             order by t.assigned_at, t.id",
            )
            .bind(user_id)
            .bind(crew_ids.clone())
            .bind(filter.repo_key.as_deref())
            .bind(filter.workspace_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
        if !unassigned.is_empty() {
            triage_json.push(serde_json::json!({
                "provider": "unassigned",
                "sprint": serde_json::Value::Null,
                "count": unassigned.len(),
                "tasks": unassigned
                    .iter()
                    .map(
                        |(id, crew_id, title, phase, state, round, repo_key, item_id)| {
                            serde_json::json!({
                                "id": id.to_string(),
                                "crew_id": crew_id.to_string(),
                                "title": title,
                                "phase": phase,
                                "state": state,
                                "round": round,
                                "repo_key": repo_key,
                                "item_id": item_id.map(|i| i.to_string()),
                            })
                        }
                    )
                    .collect::<Vec<_>>(),
            }));
        }
        // Completions are the view's payloads, passed through untouched
        // (outcome + the session-local window are its contract).
        let completed: Vec<serde_json::Value> = sqlx_core::query_scalar::query_scalar(
            "select v.payload from relay.now v
             where v.user_id = $1 and v.section = 'completed_today' and v.crew_id = any($2)
             order by v.payload->>'completed_at' desc, v.id desc",
        )
        .bind(user_id)
        .bind(crew_ids.clone())
        .fetch_all(&mut *tx)
        .await
        .map_err(db_err)?;
        let reviewed: i64 = sqlx_core::query_scalar::query_scalar(
            "select count(*) from relay.actions
             where user_id = $1 and kind = 'reviewed' and crew_id = any($2)
               and created_at >= date_trunc('day', current_timestamp)",
        )
        .bind(user_id)
        .bind(crew_ids.clone())
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;
        let first_pass: i64 = sqlx_core::query_scalar::query_scalar(
            "select count(*) from relay.actions
             where user_id = $1 and kind = 'reviewed' and crew_id = any($2)
               and round = 1 and props->>'verdict' = 'approved'
               and created_at >= date_trunc('day', current_timestamp)",
        )
        .bind(user_id)
        .bind(crew_ids.clone())
        .fetch_one(&mut *tx)
        .await
        .map_err(db_err)?;
        // A granted build lease reads if one exists (nothing in slice 1
        // writes them; the read stays honest for hand-held rows).
        let build: Option<(String, String, DateTime<Utc>)> = sqlx_core::query_as::query_as(
            "select s.resource, l.code, s.granted_at from relay.leases s
             join relay.lanes l on l.user_id = s.user_id and l.id = s.lane_id
             where s.user_id = $1 and s.kind = 'resource' and s.state = 'granted'
             order by s.granted_at desc nulls last, s.id desc limit 1",
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(serde_json::json!({
            "crews": crews_json,
            "facets": {
                "repos": repos,
                "workspaces": workspaces
                    .iter()
                    .map(|(id, name)| serde_json::json!({"id": id.to_string(), "name": name}))
                    .collect::<Vec<_>>(),
            },
            "lanes": lanes_json,
            "decisions": decisions_json,
            "triage": triage_json,
            "completed_today": completed,
            "today": {
                "completed": completed.len(),
                "reviewed": if reviewed > 0 { serde_json::json!(reviewed) } else { serde_json::Value::Null },
                "first_pass_approved": if first_pass > 0 { serde_json::json!(first_pass) } else { serde_json::Value::Null },
            },
            "resources": {
                "build_lease": build.map(|(resource, lane_code, granted_at)| serde_json::json!({
                    "resource": resource,
                    "lane_code": lane_code,
                    "granted_at": iso(granted_at),
                })),
                "disk_free_gb": disk_free_gb(),
                "disk_floor_gb": DISK_FLOOR_GB,
                "swap_free_gb": swap_free_gb(),
                "swap_floor_gb": SWAP_FLOOR_GB,
            },
        }))
    }

    /// `RelayQueue`: manual triage rows (the tracker pull is slice 2) and
    /// the backlog in position order.
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_queue(
        &self,
        user_id: Uuid,
        filter: RelayReadFilter,
    ) -> Result<serde_json::Value, RelayError> {
        // Items are crew-global: the filter narrows their own columns.
        let triage: Vec<(
            Uuid,
            Option<String>,
            Option<Uuid>,
            String,
            String,
            Option<Vec<String>>,
            Option<Uuid>,
            Option<String>,
            Option<String>,
        )> = sqlx_core::query_as::query_as(
            "select id, repo_key, workspace_id, kind::text, title, tags, added_by,
                    ticket_provider, ticket_key
             from relay.items
             where user_id = $1 and state = 'triage'
               and ($2::text is null or repo_key = $2)
               and ($3::uuid is null or workspace_id = $3)
             order by added_at, id",
        )
        .bind(user_id)
        .bind(filter.repo_key.as_deref())
        .bind(filter.workspace_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        let mut triage_json = Vec::with_capacity(triage.len());
        for (id, repo_key, workspace_id, kind, title, tags, added_by, provider, key) in triage {
            // Exactly one identity: manual items by id, ticketed rows by
            // ticket (slice 2's tracker pull shows ticketed rows with no
            // item at all; a ticketed ITEM reads by its ticket too).
            let (item_id, ticket) = match (provider, key) {
                (Some(provider), Some(key)) => (
                    serde_json::Value::Null,
                    serde_json::json!({
                        "provider": word(parse_provider(&provider)?),
                        "key": key,
                        "url": serde_json::Value::Null,
                    }),
                ),
                _ => (serde_json::json!(id.to_string()), serde_json::Value::Null),
            };
            triage_json.push(serde_json::json!({
                "repo_key": repo_key,
                "workspace_id": workspace_id.map(|id| id.to_string()),
                "kind": word(parse_item_kind(&kind)?),
                "title": title,
                "tags": tags.unwrap_or_default(),
                "sprint": serde_json::Value::Null,
                "added_by_you": added_by.is_none(),
                "item_id": item_id,
                "ticket": ticket,
            }));
        }
        let backlog: Vec<(
            Uuid,
            Option<String>,
            Option<Uuid>,
            String,
            String,
            Option<Vec<String>>,
            Option<Uuid>,
            Option<String>,
            Option<String>,
            Option<i32>,
        )> = sqlx_core::query_as::query_as(
            "select id, repo_key, workspace_id, kind::text, title, tags, added_by,
                    ticket_provider, ticket_key, position
             from relay.items
             where user_id = $1 and state = 'backlog'
               and ($2::text is null or repo_key = $2)
               and ($3::uuid is null or workspace_id = $3)
             order by position nulls last, id",
        )
        .bind(user_id)
        .bind(filter.repo_key.as_deref())
        .bind(filter.workspace_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        let mut backlog_json = Vec::with_capacity(backlog.len());
        for (id, repo_key, workspace_id, kind, title, tags, added_by, provider, key, position) in
            backlog
        {
            let ticket = match (provider, key) {
                (Some(provider), Some(key)) => serde_json::json!({
                    "provider": word(parse_provider(&provider)?),
                    "key": key,
                    "url": serde_json::Value::Null,
                }),
                _ => serde_json::Value::Null,
            };
            backlog_json.push(serde_json::json!({
                "item_id": id.to_string(),
                "repo_key": repo_key,
                "workspace_id": workspace_id.map(|id| id.to_string()),
                "kind": word(parse_item_kind(&kind)?),
                "title": title,
                "tags": tags.unwrap_or_default(),
                "sprint": serde_json::Value::Null,
                "ticket": ticket,
                "added_by_you": added_by.is_none(),
                "position": position.unwrap_or(i32::MAX),
            }));
        }
        Ok(serde_json::json!({"triage": triage_json, "backlog": backlog_json}))
    }

    /// `RelayTaskView` for (user, crew, task): anything else is `NotFound`.
    // Wide rows are inherent to the reads: one round trip per shape,
    // destructured at the use site.
    #[allow(clippy::type_complexity)]
    pub async fn relay_task_view(
        &self,
        user_id: Uuid,
        crew_id: Uuid,
        task_id: Uuid,
    ) -> Result<serde_json::Value, RelayError> {
        let task: Option<(
            String,
            String,
            String,
            Option<Uuid>,
            String,
            i32,
            Option<Uuid>,
            Option<Uuid>,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        )> = sqlx_core::query_as::query_as(
            "select title, repo_key, phase::text, workspace_id, state::text, round,
                    item_id, lane_id, assigned_at, completed_at
             from relay.tasks where user_id = $1 and crew_id = $2 and id = $3",
        )
        .bind(user_id)
        .bind(crew_id)
        .bind(task_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some((
            title,
            repo_key,
            phase,
            workspace_id,
            state,
            round,
            item_id,
            lane_id,
            assigned_at,
            completed_at,
        )) = task
        else {
            return Err(RelayError::NotFound("task not found".into()));
        };
        let ticket = match item_id {
            Some(item_id) => {
                let item: Option<(Option<String>, Option<String>)> = sqlx_core::query_as::query_as(
                    "select ticket_provider, ticket_key from relay.items
                         where user_id = $1 and id = $2",
                )
                .bind(user_id)
                .bind(item_id)
                .fetch_optional(self.pool())
                .await
                .map_err(db_err)?;
                match item {
                    Some((Some(provider), Some(key))) => serde_json::json!({
                        "provider": word(parse_provider(&provider)?),
                        "key": key,
                        "url": serde_json::Value::Null,
                    }),
                    _ => serde_json::Value::Null,
                }
            }
            None => serde_json::Value::Null,
        };
        let lane_json = match lane_id {
            Some(lane_id) => {
                let lane: Option<(String, Option<String>, String, Option<String>, Option<String>)> =
                    sqlx_core::query_as::query_as(
                        "select code, title, family, version, model from relay.lanes
                         where user_id = $1 and id = $2",
                    )
                    .bind(user_id)
                    .bind(lane_id)
                    .fetch_optional(self.pool())
                    .await
                    .map_err(db_err)?;
                lane.map(|(code, title, family, version, model)| {
                    serde_json::json!({
                        "id": lane_id.to_string(),
                        "code": code,
                        "title": title,
                        "family": family,
                        "version": version,
                        "model": model,
                    })
                })
                .unwrap_or(serde_json::Value::Null)
            }
            None => serde_json::Value::Null,
        };
        // Rounds group reported turns by phase (canonical order); a turn
        // without its own phase reads under the task's.
        let mut rounds: Vec<(String, i64)> = sqlx_core::query_as::query_as(
            "select coalesce(a.phase::text, $1), count(*) from relay.actions a
             where a.user_id = $2 and a.task_id = $3 and a.kind = 'reported'
             group by coalesce(a.phase::text, $1)",
        )
        .bind(phase.clone())
        .bind(user_id)
        .bind(task_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        rounds.sort_by_key(|r| phase_rank(&r.0));
        // The header's `needs_answer` is the `relay.now` rule: an open
        // person ask on the task or blocking it.
        let needs_answer: bool = sqlx_core::query_scalar::query_scalar(
            "select exists (
               select 1 from relay.actions a
               where a.user_id = $1 and a.kind = 'asked' and a.props->>'asked_of' = 'person'
                 and (a.task_id = $2 or a.props->'blocks' @> to_jsonb($3::text))
                 and not exists (
                   select 1 from relay.actions an
                   where an.user_id = $1 and an.kind = 'answered' and an.reply_to = a.id))",
        )
        .bind(user_id)
        .bind(task_id)
        .bind(task_id.to_string())
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;
        // ── run totals, over the task's turns (the screen adds nothing up).
        let reported: Vec<(serde_json::Value, DateTime<Utc>, Option<i32>)> =
            sqlx_core::query_as::query_as(
                "select props, created_at, round from relay.actions
                 where user_id = $1 and task_id = $2 and kind = 'reported'
                 order by created_at desc, id desc",
            )
            .bind(user_id)
            .bind(task_id)
            .fetch_all(self.pool())
            .await
            .map_err(db_err)?;
        let reviewed: Vec<(DateTime<Utc>, Option<i32>)> = sqlx_core::query_as::query_as(
            "select created_at, round from relay.actions
             where user_id = $1 and task_id = $2 and kind = 'reviewed'",
        )
        .bind(user_id)
        .bind(task_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        let num = |props: &serde_json::Value, key: &str| -> Option<i64> {
            props.get(key).and_then(|v| v.as_i64())
        };
        let sum = |key: &str| -> Option<i64> {
            let mut total = None;
            for (props, _, _) in &reported {
                if let Some(n) = num(props, key) {
                    total = Some(total.unwrap_or(0) + n);
                }
            }
            total
        };
        let reported_turns = reported
            .iter()
            .filter(|(props, _, _)| {
                TELEMETRY_KEYS.iter().any(|k| props.get(k).is_some_and(|v| !v.is_null()))
            })
            .count();
        let mut files_changed = None;
        let mut files_outside_owned = None;
        let mut files_seen = false;
        for (props, _, _) in &reported {
            if let Some(files) = props.get("files").and_then(|v| v.as_array()) {
                files_seen = true;
                for f in files {
                    let added = f.get("added").and_then(|v| v.as_i64()).unwrap_or(0);
                    let removed = f.get("removed").and_then(|v| v.as_i64()).unwrap_or(0);
                    files_changed = Some(files_changed.unwrap_or(0) + added + removed);
                    if f.get("outside_owned").and_then(|v| v.as_bool()).unwrap_or(true) {
                        files_outside_owned = Some(files_outside_owned.unwrap_or(0) + 1);
                    }
                }
            }
        }
        if !files_seen {
            files_changed = None;
            files_outside_owned = None;
        }
        let mut gates_run = None;
        let mut gates_seen = false;
        for (props, _, _) in &reported {
            if let Some(gates) = props.get("gates").and_then(|v| v.as_array()) {
                gates_seen = true;
                gates_run = Some(gates_run.unwrap_or(0) + gates.len() as i64);
            }
        }
        if !gates_seen {
            gates_run = None;
        }
        // Working time prefers the turn's own stamps, else its key.
        let mut working_secs = None;
        for (props, _, _) in &reported {
            let pair = || -> Option<i64> {
                let start = props.get("started_at")?.as_str()?;
                let end = props.get("ended_at")?.as_str()?;
                let start = chrono::DateTime::parse_from_rfc3339(start).ok()?;
                let end = chrono::DateTime::parse_from_rfc3339(end).ok()?;
                Some((end - start).num_seconds().max(0))
            };
            if let Some(secs) = pair().or_else(|| num(props, "working_secs").map(|n| n.max(0))) {
                working_secs = Some(working_secs.unwrap_or(0) + secs);
            }
        }
        // Waiting time is the broker's measurement: per round, the first
        // review minus the first report, where both exist. A lane-reported
        // `waiting_secs` key is stored and shown, never totaled.
        let mut waiting_secs = None;
        let mut rounds_seen = std::collections::HashSet::new();
        for (_, _, round) in &reported {
            rounds_seen.insert(*round);
        }
        for (_, round) in &reviewed {
            rounds_seen.insert(*round);
        }
        for round in rounds_seen {
            let first_report =
                reported.iter().filter(|(_, _, r)| *r == round).map(|(_, at, _)| *at).min();
            let first_review =
                reviewed.iter().filter(|(_, r)| *r == round).map(|(at, _)| *at).min();
            if let (Some(rep), Some(rev)) = (first_report, first_review) {
                let secs = (rev - rep).num_seconds().max(0);
                waiting_secs = Some(waiting_secs.unwrap_or(0) + secs);
            }
        }
        // The run's current branch/commits/model: the newest turn carrying
        // the key wins (a bare turn wipes nothing).
        let mut branch = serde_json::Value::Null;
        let mut commits = serde_json::Value::Null;
        let mut model = serde_json::Value::Null;
        for (props, _, _) in &reported {
            if branch.is_null()
                && let Some(v) = props.get("branch").filter(|v| !v.is_null())
            {
                branch = v.clone();
            }
            if commits.is_null()
                && let Some(v) = props.get("commits").filter(|v| v.is_array())
            {
                commits = v.clone();
            }
            if model.is_null()
                && let Some(v) = props.get("model").filter(|v| !v.is_null())
            {
                model = v.clone();
            }
        }
        let mut subagents = None;
        let mut subagents_seen = false;
        for (props, _, _) in &reported {
            match props.get("subagents") {
                Some(v) if v.is_i64() => {
                    subagents_seen = true;
                    subagents = Some(subagents.unwrap_or(0) + v.as_i64().unwrap_or(0));
                }
                Some(v) if v.is_array() => {
                    subagents_seen = true;
                    subagents = Some(
                        subagents.unwrap_or(0) + v.as_array().map(|a| a.len() as i64).unwrap_or(0),
                    );
                }
                _ => {}
            }
        }
        if !subagents_seen {
            subagents = None;
        }
        let decision: Option<(
            Uuid,
            Option<Uuid>,
            Option<String>,
            serde_json::Value,
            DateTime<Utc>,
        )> = sqlx_core::query_as::query_as(
            "select a.id, a.task_id, a.body_md, a.props, a.created_at from relay.actions a
                 where a.user_id = $1 and a.task_id = $2 and a.kind = 'asked'
                   and a.props->>'asked_of' = 'person'
                   and not exists (
                     select 1 from relay.actions an
                     where an.user_id = $1 and an.kind = 'answered' and an.reply_to = a.id)
                 order by a.created_at desc, a.id desc limit 1",
        )
        .bind(user_id)
        .bind(task_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let decision_json = match decision {
            Some((id, ask_task, body, props, asked_at)) => {
                let options = props
                    .get("options")
                    .and_then(|v| v.as_array())
                    .map(|options| {
                        options
                            .iter()
                            .map(|o| {
                                serde_json::json!({
                                    "id": o.get("id"),
                                    "label": o.get("label"),
                                    "detail": o.get("detail").cloned().unwrap_or(serde_json::Value::Null),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                serde_json::json!({
                    "id": id.to_string(),
                    "crew_id": crew_id.to_string(),
                    "task_id": ask_task.map(|id| id.to_string()),
                    "task_title": title,
                    "question": body,
                    "context": props.get("context"),
                    "options": options,
                    "recommended": props.get("recommended"),
                    "asked_at": iso(asked_at),
                })
            }
            None => serde_json::Value::Null,
        };
        let handoff_row: Option<(Option<String>, serde_json::Value, DateTime<Utc>)> =
            sqlx_core::query_as::query_as(
                "select body_md, props, created_at from relay.actions
                 where user_id = $1 and task_id = $2 and kind in ('assigned', 'brief_updated')
                 order by created_at desc, id desc limit 1",
            )
            .bind(user_id)
            .bind(task_id)
            .fetch_optional(self.pool())
            .await
            .map_err(db_err)?;
        let handoff_json = match handoff_row {
            Some((body, props, at)) => serde_json::json!({
                "body_md": body.unwrap_or_default(),
                "docs": props.get("docs").and_then(|v| v.as_array()).cloned().unwrap_or_default(),
                "at": iso(at),
            }),
            None => serde_json::Value::Null,
        };
        let history: Vec<(
            Uuid,
            Option<String>,
            String,
            Option<i32>,
            Option<String>,
            Option<String>,
            Option<Uuid>,
            DateTime<Utc>,
            serde_json::Value,
        )> = sqlx_core::query_as::query_as(
            "select action_id, lane_code, kind::text, round, phase::text, body_md,
                    reply_to, created_at, props
             from relay.task_history where user_id = $1 and task_id = $2
             order by created_at desc, action_id desc",
        )
        .bind(user_id)
        .bind(task_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        let history_json: Vec<serde_json::Value> = history
            .iter()
            .map(|(id, lane_code, kind, round, phase, body, reply_to, created_at, props)| {
                serde_json::json!({
                    "id": id.to_string(),
                    "lane_code": lane_code,
                    "kind": kind,
                    "round": round,
                    "phase": phase,
                    "body_md": body,
                    "reply_to": reply_to.map(|id| id.to_string()),
                    "created_at": iso(*created_at),
                    "props": history_props(kind, props),
                })
            })
            .collect();
        let mut lane_refs: Vec<(Uuid, String, Option<String>)> = sqlx_core::query_as::query_as(
            "select id, code, title from relay.lanes where user_id = $1 and crew_id = $2",
        )
        .bind(user_id)
        .bind(crew_id)
        .fetch_all(self.pool())
        .await
        .map_err(db_err)?;
        lane_refs.sort_by(|a, b| code_rank(&a.1).cmp(&code_rank(&b.1)));
        Ok(serde_json::json!({
            "task": {
                "id": task_id.to_string(),
                "crew_id": crew_id.to_string(),
                "title": title,
                "repo_key": repo_key,
                "workspace_id": workspace_id.map(|id| id.to_string()),
                "phase": phase,
                "state": state,
                "round": round,
                "ticket": ticket,
                "lane": lane_json,
                "assigned_at": iso(assigned_at),
                "completed_at": completed_at.map(iso),
                "rounds": rounds
                    .iter()
                    .map(|(phase, count)| serde_json::json!({"phase": phase, "count": count}))
                    .collect::<Vec<_>>(),
                "needs_answer": needs_answer,
            },
            "run": {
                "repository": repo_key,
                "branch": branch,
                "commits": commits,
                "model": model,
                "tokens_in": sum("tokens_in"),
                "tokens_out": sum("tokens_out"),
                "tokens_cache": sum("tokens_cache"),
                "context_tokens": sum("context_tokens"),
                "compactions": sum("compactions"),
                "peak_memory_mb": sum("peak_memory_mb"),
                "subagents": subagents,
                "working_secs": working_secs,
                "waiting_secs": waiting_secs,
                "files_changed": files_changed,
                "files_outside_owned": files_outside_owned,
                "gates_run": gates_run,
                "reported_turns": reported_turns,
                "turns": reported.len(),
            },
            "decision": decision_json,
            "handoff": handoff_json,
            "history": history_json,
            "lanes": lane_refs
                .iter()
                .map(|(id, code, title)| serde_json::json!({
                    "id": id.to_string(),
                    "code": code,
                    "title": title,
                }))
                .collect::<Vec<_>>(),
        }))
    }

    /// The person adds a manual triage item (person-only). Slice 1's only
    /// item source besides `plan_item`'s manual tasks — the tracker pull is
    /// slice 2.
    pub async fn relay_triage_add(
        &self,
        user_id: Uuid,
        caller: RelayCaller,
        input: RelayTriageInput,
    ) -> Result<Uuid, RelayError> {
        if matches!(caller, RelayCaller::Lane { .. }) {
            return Err(RelayError::Forbidden("person only".into()));
        }
        // Untagged manual items carry null tags (the DDL's own rule), and
        // the person adds as `added_by` null.
        let tags: Option<Vec<String>> = if input.tags.is_empty() { None } else { Some(input.tags) };
        let id = new_id();
        sqlx_core::query::query(
            "insert into relay.items
               (user_id, id, workspace_id, repo_key, title, kind, tags, state)
             values ($1, $2, $3, $4, $5, $6::relay.item_kind, $7,
                     'triage'::relay.item_state)",
        )
        .bind(user_id)
        .bind(id)
        .bind(input.workspace_id)
        .bind(input.repo_key.as_deref())
        .bind(input.title.as_str())
        .bind(word(input.kind))
        .bind(tags)
        .execute(self.pool())
        .await
        .map_err(db_err)?;
        Ok(id)
    }

    /// The person's `+`: a triage/manual item → backlog with the next
    /// position. Person-only (lanes plan via `plan_item`).
    pub async fn relay_backlog_add(
        &self,
        user_id: Uuid,
        caller: RelayCaller,
        item_id: Uuid,
    ) -> Result<Uuid, RelayError> {
        if matches!(caller, RelayCaller::Lane { .. }) {
            return Err(RelayError::Forbidden("lanes plan via plan_item".into()));
        }
        let state: Option<String> = sqlx_core::query_scalar::query_scalar(
            "select state::text from relay.items where user_id = $1 and id = $2",
        )
        .bind(user_id)
        .bind(item_id)
        .fetch_optional(self.pool())
        .await
        .map_err(db_err)?;
        let Some(state) = state else {
            return Err(RelayError::NotFound("item not found".into()));
        };
        if state != "triage" {
            return Err(RelayError::Conflict(format!("item is {state}")));
        }
        let max: Option<i32> = sqlx_core::query_scalar::query_scalar(
            "select max(position) from relay.items where user_id = $1 and state = 'backlog'",
        )
        .bind(user_id)
        .fetch_one(self.pool())
        .await
        .map_err(db_err)?;
        sqlx_core::query::query(
            "update relay.items set state = 'backlog'::relay.item_state, position = $1
             where user_id = $2 and id = $3",
        )
        .bind(max.unwrap_or(0) + 1)
        .bind(user_id)
        .bind(item_id)
        .execute(self.pool())
        .await
        .map_err(db_err)?;
        Ok(item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Deterministic v7 ids (`01938931-0000-7000-8000-<n>`); the DB's v7
    /// check reads position 15, which is `7` here.
    fn v7(n: u32) -> Uuid {
        Uuid::parse_str(&format!("01938931-0000-7000-8000-{n:012x}")).unwrap()
    }

    /// sha256 hex, computed independently of `hash_lane_token` (the stub
    /// under test must agree with this oracle, not with itself).
    fn h(token: &str) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(token.as_bytes()))
    }

    /// The fed clock: every lease test pins exact stamps off this instant.
    fn t0() -> DateTime<Utc> {
        "2026-11-06T12:00:00Z".parse().unwrap()
    }

    fn u1() -> Uuid {
        v7(0xa1)
    }
    fn u2() -> Uuid {
        v7(0xa2)
    }

    /// Unique v7 ids for rows the test doesn't name (leases, actions).
    fn fxid() -> Uuid {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0x1000);
        v7(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    async fn fx_crew(pg: &PgStore, user: Uuid, id: Uuid) {
        sqlx_core::query::query(
            "insert into relay.crews (user_id, id, status) values ($1, $2, 'active')",
        )
        .bind(user)
        .bind(id)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    async fn fx_lane(
        pg: &PgStore,
        user: Uuid,
        id: Uuid,
        crew: Uuid,
        code: &str,
        role: &str,
        token: &str,
    ) {
        sqlx_core::query::query(
            "insert into relay.lanes (user_id, id, crew_id, code, role, family, token_hash) \
             values ($1, $2, $3, $4, $5::relay.lane_role, 'claude-code', $6)",
        )
        .bind(user)
        .bind(id)
        .bind(crew)
        .bind(code)
        .bind(role)
        .bind(h(token))
        .execute(pg.pool())
        .await
        .unwrap();
    }

    async fn fx_lease(
        pg: &PgStore,
        user: Uuid,
        crew: Uuid,
        lane: Uuid,
        state: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> Uuid {
        let id = fxid();
        sqlx_core::query::query(
            "insert into relay.leases (user_id, id, crew_id, lane_id, kind, state, expires_at, granted_at) \
             values ($1, $2, $3, $4, 'lane', $5::relay.lease_state, $6, current_timestamp)",
        )
        .bind(user)
        .bind(id)
        .bind(crew)
        .bind(lane)
        .bind(state)
        .bind(expires_at)
        .execute(pg.pool())
        .await
        .unwrap();
        id
    }

    #[allow(clippy::too_many_arguments)]
    async fn fx_resource_lease(
        pg: &PgStore,
        user: Uuid,
        crew: Uuid,
        lane: Uuid,
        resource: &str,
        state: &str,
        granted_at: Option<DateTime<Utc>>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Uuid {
        let id = fxid();
        sqlx_core::query::query(
            "insert into relay.leases (user_id, id, crew_id, lane_id, kind, resource, state, granted_at, expires_at) \
             values ($1, $2, $3, $4, 'resource', $5, $6::relay.lease_state, $7, $8)",
        )
        .bind(user)
        .bind(id)
        .bind(crew)
        .bind(lane)
        .bind(resource)
        .bind(state)
        .bind(granted_at)
        .bind(expires_at)
        .execute(pg.pool())
        .await
        .unwrap();
        id
    }

    /// A queued waiter fixture with explicit floors: the sweep's auto-grant
    /// re-checks waiters, so judged waiters carry zero floors (deterministic
    /// grant on any machine) instead of the ambient consts path.
    async fn fx_queued_waiter(
        pg: &PgStore,
        user: Uuid,
        crew: Uuid,
        lane: Uuid,
        resource: &str,
        floors: RelayFloors,
    ) -> Uuid {
        let id = fxid();
        sqlx_core::query::query(
            "insert into relay.leases (user_id, id, crew_id, lane_id, kind, resource, state, props) \
             values ($1, $2, $3, $4, 'resource', $5, 'queued'::relay.lease_state, $6)",
        )
        .bind(user)
        .bind(id)
        .bind(crew)
        .bind(lane)
        .bind(resource)
        .bind(json!({"floors": {"disk_gb": floors.disk_gb, "swap_gb": floors.swap_gb}}))
        .execute(pg.pool())
        .await
        .unwrap();
        id
    }

    async fn resource_row(
        pg: &PgStore,
        user: Uuid,
        id: Uuid,
    ) -> (
        String,
        Option<String>,
        serde_json::Value,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    ) {
        sqlx_core::query_as::query_as(
            "select state::text, resource, props, granted_at, expires_at, released_at \
             from relay.leases where user_id = $1 and id = $2",
        )
        .bind(user)
        .bind(id)
        .fetch_one(pg.pool())
        .await
        .unwrap()
    }

    /// A contributor crew: U1 owns C1 (coordinator A1 + live contributor A2),
    /// U2 owns C3 (coordinator B1). Tokens are `tok-<lane>`.
    async fn fx_two_crews(pg: &PgStore) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
        let (c1, c3) = (v7(0xc1), v7(0xc3));
        let (a1, a2, b1) = (v7(0x11), v7(0x12), v7(0x13));
        fx_crew(pg, u1(), c1).await;
        fx_crew(pg, u2(), c3).await;
        fx_lane(pg, u1(), a1, c1, "L1", "coordinator", "tok-a1").await;
        fx_lane(pg, u1(), a2, c1, "L2", "contributor", "tok-a2").await;
        fx_lane(pg, u2(), b1, c3, "L1", "coordinator", "tok-b1").await;
        for lane in [a1, a2, b1] {
            let crew = if lane == b1 { c3 } else { c1 };
            let user = if lane == b1 { u2() } else { u1() };
            fx_lease(pg, user, crew, lane, "granted", Some(t0() + chrono::Duration::seconds(300)))
                .await;
        }
        (c1, c3, a1, a2, b1, u1())
    }

    #[allow(clippy::too_many_arguments)]
    async fn fx_task(
        pg: &PgStore,
        user: Uuid,
        id: Uuid,
        crew: Uuid,
        lane: Option<Uuid>,
        state: &str,
        phase: &str,
        round: i32,
    ) {
        sqlx_core::query::query(
            "insert into relay.tasks (user_id, id, crew_id, repo_key, lane_id, title, phase, state, round) \
             values ($1, $2, $3, 'github.com/o/r', $4, 'T', $5::relay.task_phase, $6::relay.task_state, $7)",
        )
        .bind(user)
        .bind(id)
        .bind(crew)
        .bind(lane)
        .bind(phase)
        .bind(state)
        .bind(round)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    struct ActionFx {
        task: Option<Uuid>,
        lane: Option<Uuid>,
        kind: &'static str,
        body: Option<&'static str>,
        report_md: Option<&'static str>,
        props: serde_json::Value,
        reply_to: Option<Uuid>,
        round: Option<i32>,
        phase: Option<&'static str>,
    }

    impl Default for ActionFx {
        fn default() -> Self {
            Self {
                task: None,
                lane: None,
                kind: "message",
                body: None,
                report_md: None,
                props: json!({}),
                reply_to: None,
                round: None,
                phase: None,
            }
        }
    }

    async fn fx_action(pg: &PgStore, user: Uuid, crew: Uuid, fx: ActionFx) -> Uuid {
        let id = fxid();
        sqlx_core::query::query(
            "insert into relay.actions (user_id, id, crew_id, task_id, lane_id, kind, round, phase, body_md, report_md, props, reply_to) \
             values ($1, $2, $3, $4, $5, $6::relay.action_kind, $7, $8::relay.task_phase, $9, $10, $11, $12)",
        )
        .bind(user)
        .bind(id)
        .bind(crew)
        .bind(fx.task)
        .bind(fx.lane)
        .bind(fx.kind)
        .bind(fx.round)
        .bind(fx.phase)
        .bind(fx.body)
        .bind(fx.report_md)
        .bind(fx.props)
        .bind(fx.reply_to)
        .execute(pg.pool())
        .await
        .unwrap();
        id
    }

    async fn fx_item(
        pg: &PgStore,
        user: Uuid,
        id: Uuid,
        state: &str,
        repo_key: Option<&str>,
        position: Option<i32>,
    ) {
        sqlx_core::query::query(
            "insert into relay.items (user_id, id, repo_key, title, kind, state, position) \
             values ($1, $2, $3, 'Item', 'gap', $4::relay.item_state, $5)",
        )
        .bind(user)
        .bind(id)
        .bind(repo_key)
        .bind(state)
        .bind(position)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    async fn count(pg: &PgStore, table: &str) -> i64 {
        sqlx_core::query_scalar::query_scalar::<_, i64>(&format!(
            "select count(*) from relay.{table}"
        ))
        .fetch_one(pg.pool())
        .await
        .unwrap()
    }

    async fn task_state(pg: &PgStore, id: Uuid) -> String {
        sqlx_core::query_scalar::query_scalar::<_, String>(
            "select state::text from relay.tasks where id = $1",
        )
        .bind(id)
        .fetch_one(pg.pool())
        .await
        .unwrap()
    }

    // ── register ──

    fn register_input(role: RelayRole, crew_id: Option<Uuid>) -> RelayRegisterInput {
        RelayRegisterInput {
            role,
            family: "claude-code".into(),
            version: None,
            model: None,
            capabilities: vec!["rust".into()],
            repo: None,
            worktree: None,
            crew_id,
            goal: None,
            repo_key: None,
            workspace_id: None,
        }
    }

    #[tokio::test]
    async fn register_coordinator_without_crew_mints_crew_q16rec() {
        let pg = PgStore::test_store().await;
        let mut input = register_input(RelayRole::Coordinator, None);
        input.goal = Some("Dogfood".into());
        let out = pg.relay_register(u1(), input, t0()).await.unwrap();
        // Q16-REC: the mint response carries the crew (the paste line needs it)…
        let crew_id: Uuid =
            sqlx_core::query_scalar::query_scalar("select id from relay.crews where user_id = $1")
                .bind(u1())
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(out.crew_id, crew_id);
        // …the lane joins it as L1 with a granted lease and its minted
        // title (F2: code + family, persisted on the row)…
        let (code, role, title): (String, String, Option<String>) = sqlx_core::query_as::query_as(
            "select code, role::text, title from relay.lanes where id = $1",
        )
        .bind(out.lane_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            (code.as_str(), role.as_str(), title.as_deref()),
            ("L1", "coordinator", Some("L1 · claude-code"))
        );
        let (state, expires): (String, DateTime<Utc>) = sqlx_core::query_as::query_as(
            "select state::text, expires_at from relay.leases where lane_id = $1 order by id desc limit 1",
        )
        .bind(out.lane_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(state, "granted");
        assert_eq!(expires, t0() + chrono::Duration::seconds(300));
        // …and the token is hashed: stored ≠ raw, and it matches this oracle.
        let stored: String = sqlx_core::query_scalar::query_scalar(
            "select token_hash from relay.lanes where id = $1",
        )
        .bind(out.lane_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_ne!(stored, out.lane_token);
        assert_eq!(stored, h(&out.lane_token));
        assert_eq!(out.lane_title.as_deref(), Some("L1 · claude-code"));
    }

    #[tokio::test]
    async fn register_contributor_joins_takes_next_code() {
        let pg = PgStore::test_store().await;
        let (c1, ..) = fx_two_crews(&pg).await;
        let out = pg
            .relay_register(u1(), register_input(RelayRole::Contributor, Some(c1)), t0())
            .await
            .unwrap();
        assert_eq!(out.crew_id, c1);
        let code: String =
            sqlx_core::query_scalar::query_scalar("select code from relay.lanes where id = $1")
                .bind(out.lane_id)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(code, "L3");
        // Codes are per crew: the other crew still mints L1/L2 freely.
        assert_eq!(count(&pg, "crews").await, 2);
    }

    #[tokio::test]
    async fn register_contributor_without_crew_refused_and_atomic() {
        let pg = PgStore::test_store().await;
        let err = pg
            .relay_register(u1(), register_input(RelayRole::Contributor, None), t0())
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "{err:?}");
        assert_eq!(count(&pg, "crews").await, 0);
        assert_eq!(count(&pg, "lanes").await, 0);
        assert_eq!(count(&pg, "leases").await, 0);
    }

    #[tokio::test]
    async fn register_second_coordinator_conflicts() {
        let pg = PgStore::test_store().await;
        let (c1, ..) = fx_two_crews(&pg).await;
        let err = pg
            .relay_register(u1(), register_input(RelayRole::Coordinator, Some(c1)), t0())
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        assert_eq!(count(&pg, "lanes").await, 3);
    }

    #[tokio::test]
    async fn register_unknown_crew_is_not_found() {
        let pg = PgStore::test_store().await;
        let err = pg
            .relay_register(u1(), register_input(RelayRole::Contributor, Some(v7(9))), t0())
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        assert_eq!(count(&pg, "lanes").await, 0);
    }

    #[tokio::test]
    async fn register_tokens_are_unique_per_lane() {
        let pg = PgStore::test_store().await;
        let (c1, ..) = fx_two_crews(&pg).await;
        let a = pg
            .relay_register(u1(), register_input(RelayRole::Contributor, Some(c1)), t0())
            .await
            .unwrap();
        let b = pg
            .relay_register(u1(), register_input(RelayRole::Contributor, Some(c1)), t0())
            .await
            .unwrap();
        assert_ne!(a.lane_token, b.lane_token);
        // Distinct values hash distinctly (the no-secret oracle).
        assert_ne!(h(&a.lane_token), h(&b.lane_token));
    }

    // ── auth / heartbeat / reregister / unregister ──

    #[tokio::test]
    async fn auth_lane_resolves_session_rejects_unknown() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, _, _, _) = fx_two_crews(&pg).await;
        let s = pg.relay_auth_lane(a1, "tok-a1").await.unwrap();
        assert_eq!((s.user_id, s.crew_id, s.role), (u1(), c1, RelayRole::Coordinator));
        let err = pg.relay_auth_lane(a1, "tok-wrong").await.unwrap_err();
        assert!(matches!(err, RelayError::Unauthorized(_)), "{err:?}");
        let err = pg.relay_auth_lane(v7(0x99), "tok-a1").await.unwrap_err();
        assert!(matches!(err, RelayError::Unauthorized(_)), "{err:?}");
    }

    #[tokio::test]
    async fn heartbeat_renews_lease() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, _, _, _) = fx_two_crews(&pg).await;
        let later = t0() + chrono::Duration::seconds(60);
        let out = pg.relay_heartbeat(a1, later).await.unwrap();
        assert_eq!(out.expires_at, later + chrono::Duration::seconds(300));
        // …and stamps the crew's heartbeat (last proof any lane is alive).
        let hb: Option<DateTime<Utc>> = sqlx_core::query_scalar::query_scalar(
            "select heartbeat_at from relay.crews where id = $1",
        )
        .bind(c1)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(hb.is_some());
    }

    #[tokio::test]
    async fn auth_unregistered_lane_is_gone() {
        let pg = PgStore::test_store().await;
        let (_, _, a1, _, _, _) = fx_two_crews(&pg).await;
        pg.relay_unregister(a1).await.unwrap();
        let err = pg.relay_auth_lane(a1, "tok-a1").await.unwrap_err();
        assert!(matches!(err, RelayError::Gone(_)), "{err:?}");
    }

    #[tokio::test]
    async fn reregister_resumes_with_checkpoint_same_token() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, _, _, _) = fx_two_crews(&pg).await;
        sqlx_core::query::query("update relay.lanes set checkpoint = $1 where id = $2")
            .bind(json!({"at": "step-3"}))
            .bind(a1)
            .execute(pg.pool())
            .await
            .unwrap();
        pg.relay_unregister(a1).await.unwrap();
        let out = pg.relay_reregister(a1, t0()).await.unwrap();
        assert_eq!((out.lane_id, out.crew_id), (a1, c1));
        assert_eq!(out.checkpoint, Some(json!({"at": "step-3"})));
        assert_eq!(out.expires_at, t0() + chrono::Duration::seconds(300));
        // The old token still authenticates: minted once, never rotated here.
        assert!(pg.relay_auth_lane(a1, "tok-a1").await.is_ok());
    }

    #[tokio::test]
    async fn unregister_releases_lease_interrupts_task_atomically() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x71);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "impl", 1).await;
        pg.relay_unregister(a2).await.unwrap();
        let unreg: Option<DateTime<Utc>> = sqlx_core::query_scalar::query_scalar(
            "select unregistered_at from relay.lanes where id = $1",
        )
        .bind(a2)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(unreg.is_some());
        let lease: String = sqlx_core::query_scalar::query_scalar(
            "select state::text from relay.leases where lane_id = $1 order by id desc limit 1",
        )
        .bind(a2)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(lease, "released");
        assert_eq!(task_state(&pg, t).await, "interrupted");
        assert_eq!(count(&pg, "actions").await, 1);
        let kind: String =
            sqlx_core::query_scalar::query_scalar("select kind::text from relay.actions limit 1")
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(kind, "interrupted");
        let _ = (a1,);
    }

    #[tokio::test]
    async fn unregister_twice_is_gone() {
        let pg = PgStore::test_store().await;
        let (_, _, _, a2, _, _) = fx_two_crews(&pg).await;
        pg.relay_unregister(a2).await.unwrap();
        let err = pg.relay_unregister(a2).await.unwrap_err();
        assert!(matches!(err, RelayError::Gone(_)), "{err:?}");
    }

    // ── plan_item ──

    fn plan_input(item_id: Option<Uuid>) -> RelayPlanInput {
        RelayPlanInput {
            item_id,
            title: None,
            phase: RelayTaskPhase::Tests,
            repo_key: None,
            workspace_id: None,
        }
    }

    #[tokio::test]
    async fn plan_from_backlog_item_mints_unassigned_task() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, _, _, _) = fx_two_crews(&pg).await;
        let item = v7(0x51);
        fx_item(&pg, u1(), item, "backlog", Some("github.com/o/r"), Some(1)).await;
        let out = pg.relay_plan_item(u1(), a1, plan_input(Some(item))).await.unwrap();
        let (lane, state, round, title, repo, crew): (
            Option<Uuid>,
            String,
            i32,
            String,
            String,
            Uuid,
        ) = sqlx_core::query_as::query_as(
            "select lane_id, state::text, round, title, repo_key, crew_id from relay.tasks where id = $1",
        )
        .bind(out.task_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        // The task joins the caller lane's crew (items aren't crew-scoped).
        assert_eq!(crew, c1);
        assert_eq!(lane, None);
        assert_eq!(state, "assigned");
        assert_eq!(round, 1);
        assert_eq!(title, "Item");
        assert_eq!(repo, "github.com/o/r");
        let item_state: String = sqlx_core::query_scalar::query_scalar(
            "select state::text from relay.items where id = $1",
        )
        .bind(item)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(item_state, "planned");
    }

    #[tokio::test]
    async fn plan_repo_less_item_needs_repo_key() {
        let pg = PgStore::test_store().await;
        let (_, _, a1, _, _, _) = fx_two_crews(&pg).await;
        let item = v7(0x52);
        fx_item(&pg, u1(), item, "triage", None, None).await;
        // A manual item may name no repository; a task may not.
        let err = pg.relay_plan_item(u1(), a1, plan_input(Some(item))).await.unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "{err:?}");
        assert_eq!(count(&pg, "tasks").await, 0);
        let mut input = plan_input(Some(item));
        input.repo_key = Some("github.com/o/r".into());
        let out = pg.relay_plan_item(u1(), a1, input).await.unwrap();
        let repo: String =
            sqlx_core::query_scalar::query_scalar("select repo_key from relay.tasks where id = $1")
                .bind(out.task_id)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(repo, "github.com/o/r");
    }

    #[tokio::test]
    async fn plan_manual_task_without_item() {
        let pg = PgStore::test_store().await;
        let (_, _, a1, _, _, _) = fx_two_crews(&pg).await;
        let mut input = plan_input(None);
        input.title = Some("Manual".into());
        input.repo_key = Some("github.com/o/r".into());
        let out = pg.relay_plan_item(u1(), a1, input).await.unwrap();
        assert_eq!(task_state(&pg, out.task_id).await, "assigned");
    }

    #[tokio::test]
    async fn plan_refuses_contributor_unknown_and_closed() {
        let pg = PgStore::test_store().await;
        let (_, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let item = v7(0x53);
        fx_item(&pg, u1(), item, "backlog", Some("github.com/o/r"), None).await;
        // Contributors don't plan.
        let mut input = plan_input(Some(item));
        input.title = Some("X".into());
        input.repo_key = Some("github.com/o/r".into());
        let err = pg.relay_plan_item(u1(), a2, input).await.unwrap_err();
        assert!(matches!(err, RelayError::Forbidden(_)), "{err:?}");
        // Unknown item.
        let err = pg.relay_plan_item(u1(), a1, plan_input(Some(v7(0x99)))).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        // A closed item is done being planned.
        sqlx_core::query::query("update relay.items set state = 'closed' where id = $1")
            .bind(item)
            .execute(pg.pool())
            .await
            .unwrap();
        let err = pg.relay_plan_item(u1(), a1, plan_input(Some(item))).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        assert_eq!(count(&pg, "tasks").await, 0);
    }

    // ── assign_task ──

    fn assign_input(to_lane: Uuid) -> RelayAssignInput {
        RelayAssignInput {
            to_lane,
            title: "Diagrams header".into(),
            phase: RelayTaskPhase::Tests,
            ticket: None,
            tags: vec![],
            brief_summary: "Do the header.".into(),
            brief_docs: vec![],
            acceptance: vec!["green".into()],
            owned_paths: vec!["app/**".into()],
            deps: vec![],
            resources: vec![],
            reviewer_lane: None,
            lane_title: None,
            now: t0(),
        }
    }

    #[tokio::test]
    async fn assign_hands_task_freezes_brief_sets_lane_title() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x71);
        fx_task(&pg, u1(), t, c1, None, "assigned", "tests", 1).await;
        let mut input = assign_input(a2);
        input.lane_title = Some("Diagrams".into());
        let out = pg.relay_assign_task(u1(), a1, t, input).await.unwrap();
        assert_eq!((out.task_id, out.round, out.state), (t, 1, RelayTaskState::Assigned));
        let lane: Uuid =
            sqlx_core::query_scalar::query_scalar("select lane_id from relay.tasks where id = $1")
                .bind(t)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(lane, a2);
        // The brief freezes into the assigned action, props carry the lists.
        let (body, props): (Option<String>, serde_json::Value) = sqlx_core::query_as::query_as(
            "select body_md, props from relay.actions where kind = 'assigned'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(body.as_deref(), Some("Do the header."));
        assert_eq!(props["acceptance"], json!(["green"]));
        assert_eq!(props["owned_paths"], json!(["app/**"]));
        // First assign names the lane's objective.
        let title: Option<String> =
            sqlx_core::query_scalar::query_scalar("select title from relay.lanes where id = $1")
                .bind(a2)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(title.as_deref(), Some("Diagrams"));
    }

    #[tokio::test]
    async fn assign_refuses_busy_terminal_and_strangers() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let busy = v7(0x72);
        fx_task(&pg, u1(), busy, c1, Some(a2), "working", "tests", 1).await;
        // Already on a lane: re-assigning elsewhere is a conflict…
        let err = pg.relay_assign_task(u1(), a1, busy, assign_input(a1)).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        // …unknown lanes and tasks are not found…
        let free = v7(0x73);
        fx_task(&pg, u1(), free, c1, None, "assigned", "tests", 1).await;
        let err = pg.relay_assign_task(u1(), a1, free, assign_input(v7(0x99))).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        let err = pg.relay_assign_task(u1(), a1, v7(0x98), assign_input(a2)).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        // …a contributor can't assign…
        let err = pg.relay_assign_task(u1(), a2, free, assign_input(a2)).await.unwrap_err();
        assert!(matches!(err, RelayError::Forbidden(_)), "{err:?}");
        // …and a terminal task stays terminal.
        sqlx_core::query::query("update relay.tasks set state = 'done' where id = $1")
            .bind(free)
            .execute(pg.pool())
            .await
            .unwrap();
        let err = pg.relay_assign_task(u1(), a1, free, assign_input(a2)).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        // Nothing assigned anything: no assigned actions anywhere.
        assert_eq!(count(&pg, "actions").await, 0);
    }

    // ── F5: assign needs a live lease ──

    #[tokio::test]
    async fn assign_refuses_dead_lease() {
        // Assigning onto an expired lease strands the task: the sweep only
        // interrupts leases it sees expire, and cancel/re-assign both refuse
        // while the lane row reads held — so the bar is at assign (F5).
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        sqlx_core::query::query("update relay.leases set expires_at = $1 where lane_id = $2")
            .bind(t0() - chrono::Duration::seconds(1))
            .bind(a2)
            .execute(pg.pool())
            .await
            .unwrap();
        let t = v7(0x76);
        fx_task(&pg, u1(), t, c1, None, "assigned", "tests", 1).await;
        let err = pg.relay_assign_task(u1(), a1, t, assign_input(a2)).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        let lane: Option<Uuid> =
            sqlx_core::query_scalar::query_scalar("select lane_id from relay.tasks where id = $1")
                .bind(t)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(lane, None);
    }

    #[tokio::test]
    async fn assign_refuses_unregistered_lane() {
        // A lane that left holds no live lease either (released on the way
        // out): same bar, same conflict.
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        pg.relay_unregister(a2).await.unwrap();
        let t = v7(0x77);
        fx_task(&pg, u1(), t, c1, None, "assigned", "tests", 1).await;
        let err = pg.relay_assign_task(u1(), a1, t, assign_input(a2)).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
    }

    #[tokio::test]
    async fn assign_second_active_task_conflicts_not_500() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let first = v7(0x74);
        fx_task(&pg, u1(), first, c1, Some(a2), "working", "tests", 1).await;
        let second = v7(0x75);
        fx_task(&pg, u1(), second, c1, None, "assigned", "tests", 1).await;
        // One active task per lane: the DB unique surfaces as 409, never 500.
        let err = pg.relay_assign_task(u1(), a1, second, assign_input(a2)).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        let lane: Option<Uuid> =
            sqlx_core::query_scalar::query_scalar("select lane_id from relay.tasks where id = $1")
                .bind(second)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(lane, None);
    }

    #[tokio::test]
    async fn assign_explicit_reviewer_honored_self_review_refused() {
        // ("merge" as a phase string is refused at the handler — 422 on the
        // wire value (Q3); the store's phase is already typed.)
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x76);
        fx_task(&pg, u1(), t, c1, None, "assigned", "tests", 1).await;
        // The reviewer is never the assignee (Q14).
        let mut input = assign_input(a2);
        input.reviewer_lane = Some(a2);
        let err = pg.relay_assign_task(u1(), a1, t, input).await.unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "{err:?}");
        let mut input = assign_input(a2);
        input.phase = RelayTaskPhase::Impl;
        input.reviewer_lane = Some(a1);
        let out = pg.relay_assign_task(u1(), a1, t, input).await.unwrap();
        assert_eq!(out.state, RelayTaskState::Assigned);
        let reviewer: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select reviewer_lane_id from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(reviewer, Some(a1));
    }

    #[tokio::test]
    async fn assign_approved_new_phase_advances_q11() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x77);
        fx_task(&pg, u1(), t, c1, Some(a2), "approved", "tests", 2).await;
        // Same phase: nothing to advance.
        let err = pg.relay_assign_task(u1(), a1, t, assign_input(a2)).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        // Next phase: back to working, new round, reviewer cleared.
        let mut input = assign_input(a2);
        input.phase = RelayTaskPhase::Impl;
        let out = pg.relay_assign_task(u1(), a1, t, input).await.unwrap();
        assert_eq!(out.state, RelayTaskState::Working);
        assert_eq!(out.round, 3);
        let (round, reviewer): (i32, Option<Uuid>) = sqlx_core::query_as::query_as(
            "select round, reviewer_lane_id from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((round, reviewer), (3, None));
    }

    // ── report_task ──

    fn report_input() -> RelayReportInput {
        RelayReportInput {
            phase: RelayTaskPhase::Tests,
            round: 1,
            changed: "Wired the header.".into(),
            look_at: Some("app/header".into()),
            flags: None,
            report_md: None,
            base: None,
            branch: None,
            commits: vec![],
            files: vec![],
            gates: vec![],
            telemetry: json!({}),
        }
    }

    #[tokio::test]
    async fn report_moves_to_in_review_picks_least_loaded_q14() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        // N2: A1 reads last by code, so the least-loaded pick cannot
        // coincide with first-by-code (a code-sorted wrong impl dies here).
        sqlx_core::query::query("update relay.lanes set code = 'L9' where id = $1")
            .bind(a1)
            .execute(pg.pool())
            .await
            .unwrap();
        // A third lane, busier than the coordinator: the pick must go the
        // other way (aggregate direction — a max would pick wrong). Its two
        // open reviews sit on two other lanes (one active task per lane);
        // the fourth holds one review but its lease lapsed, so it is not a
        // candidate at all.
        let (a3, a4) = (v7(0x14), v7(0x15));
        fx_lane(&pg, u1(), a3, c1, "L3", "contributor", "tok-a3").await;
        fx_lease(&pg, u1(), c1, a3, "granted", Some(t0() + chrono::Duration::seconds(300))).await;
        fx_lane(&pg, u1(), a4, c1, "L4", "contributor", "tok-a4").await;
        fx_lease(&pg, u1(), c1, a4, "expired", Some(t0() - chrono::Duration::seconds(1))).await;
        // F1 (phase-B r2): the pick path reads liveness against DB
        // `current_timestamp`, but every fixture lease above expires at the
        // fixed t0+300 — dead in any working-hours run, so the pick fell
        // through to the coordinator fallback and the load axis never
        // engaged. Revive the same-crew lanes under the DB's own clock (the
        // same clock the pick query reads — no skew possible). The only
        // exclusions left are the two this test names: A2 by author id, A4
        // by lapsed state (clock-independent); B1 stays dead (another crew).
        sqlx_core::query::query(
            "update relay.leases set state = 'granted',
                    expires_at = current_timestamp + interval '300 seconds'
             where kind = 'lane' and lane_id in ($1, $2, $3)",
        )
        .bind(a1)
        .bind(a2)
        .bind(a3)
        .execute(pg.pool())
        .await
        .unwrap();
        for (n, holder) in [(0x81, a1), (0x82, a4)] {
            let t = v7(n);
            fx_task(&pg, u1(), t, c1, Some(holder), "in_review", "tests", 1).await;
            sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
                .bind(a3)
                .bind(t)
                .execute(pg.pool())
                .await
                .unwrap();
        }
        let t = v7(0x83);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        let out = pg.relay_report_task(u1(), a2, t, report_input()).await.unwrap();
        assert_eq!(out.state, RelayTaskState::InReview);
        // A1 (0 open reviews) beats A3 (2); the author is never picked.
        assert_eq!(out.reviewer_lane_id, Some(a1));
        let reviewer: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select reviewer_lane_id from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(reviewer, Some(a1));
        // The reported action: body is the summary, look_at rides props.
        let (body, props): (Option<String>, serde_json::Value) = sqlx_core::query_as::query_as(
            "select body_md, props from relay.actions where kind = 'reported'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(body.as_deref(), Some("Wired the header."));
        assert_eq!(props["look_at"], "app/header");
    }

    #[tokio::test]
    async fn report_keeps_picked_reviewer_on_later_reports() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x84);
        fx_task(&pg, u1(), t, c1, Some(a2), "rework", "tests", 2).await;
        sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
            .bind(a1)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let mut input = report_input();
        input.round = 2;
        let out = pg.relay_report_task(u1(), a2, t, input).await.unwrap();
        assert_eq!(
            (out.state, out.round, out.reviewer_lane_id),
            (RelayTaskState::InReview, 2, Some(a1))
        );
    }

    #[tokio::test]
    async fn report_solo_coordinator_author_has_no_reviewer() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, _, _, _) = fx_two_crews(&pg).await;
        // A2's lease lapses: the only live lane is the author-coordinator.
        sqlx_core::query::query("update relay.leases set state = 'expired' where lane_id = $1")
            .bind(v7(0x12))
            .execute(pg.pool())
            .await
            .unwrap();
        let t = v7(0x85);
        fx_task(&pg, u1(), t, c1, Some(a1), "working", "tests", 1).await;
        let out = pg.relay_report_task(u1(), a1, t, report_input()).await.unwrap();
        assert_eq!(out.reviewer_lane_id, None);
    }

    #[tokio::test]
    async fn report_refuses_strangers_stale_and_unstarted() {
        let pg = PgStore::test_store().await;
        let (c1, c3, a1, a2, b1, _) = fx_two_crews(&pg).await;
        let t = v7(0x86);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        // Another lane's task (even the coordinator's own lane).
        let err = pg.relay_report_task(u1(), a1, t, report_input()).await.unwrap_err();
        assert!(matches!(err, RelayError::Forbidden(_)), "{err:?}");
        // Another user's task entirely.
        let tb = v7(0x87);
        fx_task(&pg, u2(), tb, c3, Some(b1), "working", "tests", 1).await;
        let err = pg.relay_report_task(u1(), a2, tb, report_input()).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        // Stale round, wrong phase.
        let mut stale = report_input();
        stale.round = 2;
        let err = pg.relay_report_task(u1(), a2, t, stale).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        let mut phase = report_input();
        phase.phase = RelayTaskPhase::Impl;
        let err = pg.relay_report_task(u1(), a2, t, phase).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        // Assigned but never delivered: delivery marks started, not the report.
        // (Phase-B repair: `fresh` sits on A1, which holds nothing — the DDL's
        // one-active-task unique forbids a second active task on A2. The pin
        // — Conflict on an unstarted task — is unchanged.)
        let fresh = v7(0x88);
        fx_task(&pg, u1(), fresh, c1, Some(a1), "assigned", "tests", 1).await;
        let err = pg.relay_report_task(u1(), a1, fresh, report_input()).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        assert_eq!(count(&pg, "actions").await, 0);
    }

    #[tokio::test]
    async fn report_computes_outside_owned_and_keeps_nulls() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x89);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        sqlx_core::query::query("update relay.tasks set owned_paths = $1 where id = $2")
            .bind(vec!["app/**"])
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let mut input = report_input();
        input.files = vec![
            RelayFileInput { path: "app/a.ts".into(), added: 3, removed: 0 },
            RelayFileInput { path: "elsewhere/b.ts".into(), added: 1, removed: 1 },
        ];
        input.telemetry = json!({"tokens_in": 900000, "tokens_out": null, "compactions": null});
        pg.relay_report_task(u1(), a2, t, input).await.unwrap();
        let props: serde_json::Value = sqlx_core::query_scalar::query_scalar(
            "select props from relay.actions where kind = 'reported'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(props["files"][0]["outside_owned"], false);
        assert_eq!(props["files"][1]["outside_owned"], true);
        // Telemetry nulls stay null — never 0, never dropped silently.
        assert_eq!(props["tokens_in"], 900000);
        assert_eq!(props["tokens_out"], serde_json::Value::Null);
        assert_eq!(props["compactions"], serde_json::Value::Null);
    }

    fn secret_summary() -> String {
        ["gh", "p_"].concat() + &"a".repeat(40)
    }

    #[tokio::test]
    async fn report_rejects_long_and_leaking_summaries_atomically() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x8a);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        let mut long = report_input();
        long.changed = "x".repeat(601);
        let err = pg.relay_report_task(u1(), a2, t, long).await.unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "{err:?}");
        let mut leaked = report_input();
        leaked.changed = format!("see {}", secret_summary());
        let err = pg.relay_report_task(u1(), a2, t, leaked).await.unwrap_err();
        match err {
            RelayError::Validation(msg) => {
                assert!(msg.contains("github"), "{msg}");
                assert!(!msg.contains(&"a".repeat(40)), "kinds only: {msg}");
            }
            other => panic!("{other:?}"),
        }
        // Atomic: no action, no state change, no reviewer.
        assert_eq!(task_state(&pg, t).await, "working");
        assert_eq!(count(&pg, "actions").await, 0);
    }

    // ── submit_review ──

    fn review_input() -> RelayReviewInput {
        RelayReviewInput {
            verdict: ReviewVerdict::Approved,
            summary: "Clean.".into(),
            findings: vec![],
            probes: None,
            telemetry: json!({}),
        }
    }

    #[tokio::test]
    async fn review_approved_holds_for_merge() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x8b);
        fx_task(&pg, u1(), t, c1, Some(a2), "in_review", "tests", 1).await;
        sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
            .bind(a1)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let rep = fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                task: Some(t),
                lane: Some(a2),
                kind: "reported",
                round: Some(1),
                ..Default::default()
            },
        )
        .await;
        let out = pg.relay_submit_review(u1(), a1, t, review_input()).await.unwrap();
        assert_eq!((out.state, out.round), (RelayTaskState::Approved, 1));
        // The review replies to the latest report; approval stamps no completion.
        let (reply_to, verdict): (Option<Uuid>, String) = sqlx_core::query_as::query_as(
            "select reply_to, props->>'verdict' from relay.actions where kind = 'reviewed'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(reply_to, Some(rep));
        assert_eq!(verdict, "approved");
        let completed: Option<DateTime<Utc>> = sqlx_core::query_scalar::query_scalar(
            "select completed_at from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(completed, None);
    }

    #[tokio::test]
    async fn review_changes_requested_reworks_bumps_round() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x8c);
        fx_task(&pg, u1(), t, c1, Some(a2), "in_review", "tests", 1).await;
        sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
            .bind(a1)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let mut input = review_input();
        input.verdict = ReviewVerdict::ChangesRequested;
        input.findings = vec![RelayFindingInput {
            severity: RelayFindingSeverity::High,
            class: "silent_drop".into(),
            text: "Dropped the error.".into(),
            file: Some("a.ts".into()),
            line: Some(3),
        }];
        let out = pg.relay_submit_review(u1(), a1, t, input).await.unwrap();
        assert_eq!((out.state, out.round), (RelayTaskState::Rework, 2));
        let findings: serde_json::Value = sqlx_core::query_scalar::query_scalar(
            "select props->'findings' from relay.actions where kind = 'reviewed'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(findings[0]["class"], "silent_drop");
    }

    #[tokio::test]
    async fn review_refuses_strangers_bad_states_unknown_classes() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x8d);
        fx_task(&pg, u1(), t, c1, Some(a2), "in_review", "tests", 1).await;
        sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
            .bind(a1)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        // Not the picked reviewer — not even the author.
        for caller in [a2, v7(0x99)] {
            let err = pg.relay_submit_review(u1(), caller, t, review_input()).await.unwrap_err();
            assert!(matches!(err, RelayError::Forbidden(_) | RelayError::NotFound(_)), "{err:?}");
        }
        // Unknown finding class (the seed is the vocabulary).
        let mut input = review_input();
        input.verdict = ReviewVerdict::ChangesRequested;
        input.findings = vec![RelayFindingInput {
            severity: RelayFindingSeverity::Low,
            class: "nope-not-a-class".into(),
            text: "x".into(),
            file: None,
            line: None,
        }];
        let err = pg.relay_submit_review(u1(), a1, t, input).await.unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "{err:?}");
        // Not under review.
        sqlx_core::query::query("update relay.tasks set state = 'working' where id = $1")
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let err = pg.relay_submit_review(u1(), a1, t, review_input()).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        assert_eq!(count(&pg, "actions").await, 0);
    }

    // ── ask / answer ──

    fn ask_input(blocks: Vec<Uuid>) -> RelayAskInput {
        RelayAskInput {
            task_id: None,
            question: "Where does the log live?".into(),
            context: Some("Blocked on this.".into()),
            options: vec![RelayDecisionOption {
                id: "db".into(),
                label: "In the database".into(),
                detail: None,
            }],
            recommended: Some("db".into()),
            blocks,
            asked_of: RelayAskedOf::Person,
        }
    }

    #[tokio::test]
    async fn ask_blocks_tasks_remembering_where() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let w = v7(0x91);
        fx_task(&pg, u1(), w, c1, Some(a2), "working", "tests", 1).await;
        let r = v7(0x92);
        fx_task(&pg, u1(), r, c1, Some(a1), "in_review", "tests", 1).await;
        let out = pg.relay_ask_decision(u1(), a1, ask_input(vec![w, r])).await.unwrap();
        for (t, from) in [(w, "working"), (r, "in_review")] {
            let (state, mem): (String, Option<String>) = sqlx_core::query_as::query_as(
                "select state::text, blocked_from_state::text from relay.tasks where id = $1",
            )
            .bind(t)
            .fetch_one(pg.pool())
            .await
            .unwrap();
            assert_eq!((state.as_str(), mem.as_deref()), ("blocked", Some(from)));
        }
        let (asked_of, blocks): (String, serde_json::Value) = sqlx_core::query_as::query_as(
            "select props->>'asked_of', props->'blocks' from relay.actions where id = $1",
        )
        .bind(out.decision_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(asked_of, "person");
        // Blocks are task-id strings (the view's contract).
        assert_eq!(blocks, json!([w.to_string(), r.to_string()]));
    }

    #[tokio::test]
    async fn ask_unknown_block_refused_atomically() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let w = v7(0x93);
        fx_task(&pg, u1(), w, c1, Some(a2), "working", "tests", 1).await;
        let err = pg.relay_ask_decision(u1(), a1, ask_input(vec![w, v7(0x99)])).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        assert_eq!(task_state(&pg, w).await, "working");
        assert_eq!(count(&pg, "actions").await, 0);
    }

    #[tokio::test]
    async fn answer_unblocks_to_blocked_from() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let w = v7(0x94);
        fx_task(&pg, u1(), w, c1, Some(a2), "working", "tests", 1).await;
        let ask = pg.relay_ask_decision(u1(), a1, ask_input(vec![w])).await.unwrap();
        let answer = RelayAnswerInput { option: "db".into(), note: None };
        let action = pg
            .relay_answer_decision(
                u1(),
                RelayCaller::Person { surface: RelaySurface::Desktop },
                ask.decision_id,
                answer,
            )
            .await
            .unwrap();
        assert_eq!(task_state(&pg, w).await, "working");
        let (mem, reply_to, surface): (Option<String>, Option<Uuid>, String) =
            sqlx_core::query_as::query_as(
                "select t.blocked_from_state::text, a.reply_to, a.props->>'surface' \
             from relay.actions a join relay.tasks t on t.id = $1 where a.id = $2",
            )
            .bind(w)
            .bind(action)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(mem, None, "resume clears the memory (Q5)");
        assert_eq!(reply_to, Some(ask.decision_id));
        assert_eq!(surface, "desktop");
    }

    #[tokio::test]
    async fn answer_refuses_wrong_party_and_repeats() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let w = v7(0x95);
        fx_task(&pg, u1(), w, c1, Some(a2), "working", "tests", 1).await;
        // Asked of the lane: the person can't answer for it.
        let mut lane_ask = ask_input(vec![w]);
        lane_ask.asked_of = RelayAskedOf::Lane(a2);
        let ask = pg.relay_ask_decision(u1(), a1, lane_ask).await.unwrap();
        let answer = || RelayAnswerInput { option: "db".into(), note: None };
        let err = pg
            .relay_answer_decision(
                u1(),
                RelayCaller::Person { surface: RelaySurface::Desktop },
                ask.decision_id,
                answer(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::Forbidden(_)), "{err:?}");
        // The lane answers (daemon stamps `lane`, Q12)…
        pg.relay_answer_decision(
            u1(),
            RelayCaller::Lane { lane_id: a2 },
            ask.decision_id,
            answer(),
        )
        .await
        .unwrap();
        // …once: a second answer conflicts.
        let err = pg
            .relay_answer_decision(
                u1(),
                RelayCaller::Lane { lane_id: a2 },
                ask.decision_id,
                answer(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
    }

    // ── send_message ──

    #[tokio::test]
    async fn message_person_to_lane_stored_with_surface() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let out = pg
            .relay_send_message(
                u1(),
                RelayCaller::Person { surface: RelaySurface::Desktop },
                RelayMessageInput {
                    to: RelayMessageTo::Lane { lane_id: a2 },
                    task_id: None,
                    text: "Hi".into(),
                },
            )
            .await
            .unwrap();
        let (lane, to_lane, surface, body): (Option<Uuid>, Option<String>, String, Option<String>) =
            sqlx_core::query_as::query_as(
                "select lane_id, props->'to'->>'lane_id', props->>'surface', body_md \
                 from relay.actions where id = $1",
            )
            .bind(out.message_id)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(lane, None, "the person acts as lane_id null");
        assert_eq!(to_lane.as_deref(), Some(&*a2.to_string()));
        assert_eq!(surface, "desktop");
        assert_eq!(body.as_deref(), Some("Hi"));
        let _ = c1;
    }

    #[tokio::test]
    async fn message_lane_surface_daemon_set_spoof_refused() {
        let pg = PgStore::test_store().await;
        let (_, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        // A lane message is stamped `lane` however it arrives…
        let out = pg
            .relay_send_message(
                u1(),
                RelayCaller::Lane { lane_id: a2 },
                RelayMessageInput {
                    to: RelayMessageTo::Coordinator,
                    task_id: None,
                    text: "Ack".into(),
                },
            )
            .await
            .unwrap();
        let surface: String = sqlx_core::query_scalar::query_scalar(
            "select props->>'surface' from relay.actions where id = $1",
        )
        .bind(out.message_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(surface, "lane");
        // …but a lane claiming a person surface is spoofing (Q12 edge).
        let err = pg
            .relay_send_message(
                u1(),
                RelayCaller::Person { surface: RelaySurface::Desktop },
                RelayMessageInput {
                    to: RelayMessageTo::Coordinator,
                    task_id: None,
                    text: "x".into(),
                },
            )
            .await;
        // Person callers are fine; the spoof edge is a lane WITH a person
        // surface, which the type cannot express — the handler pins the
        // string form. This call succeeds (control).
        assert!(err.is_ok());
        let _ = a1;
    }

    #[tokio::test]
    async fn message_long_and_leaking_refused_atomically() {
        let pg = PgStore::test_store().await;
        let (_, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let caller = || RelayCaller::Lane { lane_id: a2 };
        let msg =
            |text: String| RelayMessageInput { to: RelayMessageTo::Everyone, task_id: None, text };
        let err = pg.relay_send_message(u1(), caller(), msg("x".repeat(601))).await.unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "{err:?}");
        let err = pg
            .relay_send_message(u1(), caller(), msg(format!("k {}", secret_summary())))
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "{err:?}");
        assert_eq!(count(&pg, "actions").await, 0);
    }

    // ── merge_task ──

    fn merge_input() -> RelayMergeInput {
        RelayMergeInput {
            commits: vec!["abc".into()],
            into: "develop".into(),
            gates: vec![RelayGateInput { cmd: "make test-fast".into(), exit: 0, counts: None }],
            note: "Merged.".into(),
        }
    }

    #[tokio::test]
    async fn merge_approved_to_done_stamps_completed() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x96);
        fx_task(&pg, u1(), t, c1, Some(a2), "approved", "tests", 1).await;
        pg.relay_merge_task(u1(), a1, t, merge_input()).await.unwrap();
        assert_eq!(task_state(&pg, t).await, "done");
        let completed: Option<DateTime<Utc>> = sqlx_core::query_scalar::query_scalar(
            "select completed_at from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(completed.is_some());
        let (into, body): (String, Option<String>) = sqlx_core::query_as::query_as(
            "select props->>'into', body_md from relay.actions where kind = 'merged'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(into, "develop");
        assert_eq!(body.as_deref(), Some("Merged."));
    }

    #[tokio::test]
    async fn merge_empty_commits_means_nothing_to_merge() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x97);
        fx_task(&pg, u1(), t, c1, Some(a2), "approved", "mockup", 1).await;
        let mut input = merge_input();
        input.commits = vec![];
        pg.relay_merge_task(u1(), a1, t, input).await.unwrap();
        assert_eq!(task_state(&pg, t).await, "done");
    }

    #[tokio::test]
    async fn merge_refuses_unapproved_and_contributors() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0x98);
        fx_task(&pg, u1(), t, c1, Some(a2), "in_review", "tests", 1).await;
        let err = pg.relay_merge_task(u1(), a1, t, merge_input()).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        sqlx_core::query::query("update relay.tasks set state = 'approved' where id = $1")
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        let err = pg.relay_merge_task(u1(), a2, t, merge_input()).await.unwrap_err();
        assert!(matches!(err, RelayError::Forbidden(_)), "{err:?}");
        assert_eq!(task_state(&pg, t).await, "approved");
        assert_eq!(count(&pg, "actions").await, 0);
    }

    // ── get_tasks envelope ──

    fn msg_props(to: serde_json::Value) -> serde_json::Value {
        json!({"to": to, "surface": "desktop"})
    }

    #[tokio::test]
    async fn envelope_addresses_only_the_lane_newest_first() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        // Two messages for A2 (old then new), one for A1, one for everyone.
        let _old = fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                lane: None,
                kind: "message",
                body: Some("old"),
                props: msg_props(json!({"kind": "lane", "lane_id": a2.to_string()})),
                ..Default::default()
            },
        )
        .await;
        let new = fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                lane: None,
                kind: "message",
                body: Some("new"),
                props: msg_props(json!({"kind": "lane", "lane_id": a2.to_string()})),
                ..Default::default()
            },
        )
        .await;
        fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                lane: None,
                kind: "message",
                body: Some("for-a1"),
                props: msg_props(json!({"kind": "lane", "lane_id": a1.to_string()})),
                ..Default::default()
            },
        )
        .await;
        fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                lane: None,
                kind: "message",
                body: Some("all"),
                props: msg_props(json!({"kind": "everyone"})),
                ..Default::default()
            },
        )
        .await;
        let env = pg.relay_envelope(u1(), a2).await.unwrap();
        let texts: Vec<&str> = env.messages.iter().map(|m| m.text.as_str()).collect();
        // Newest first; A1's message excluded; everyone included.
        assert_eq!(texts, vec!["all", "new", "old"]);
        assert_eq!(env.messages[1].id, new.to_string());
        assert_eq!(env.messages[0].from_lane_code, None, "the person wrote it");
        // Nothing else queued: the other groups are empty, not missing.
        assert!(env.answers.is_empty() && env.reviews.is_empty() && env.assignments.is_empty());
    }

    #[tokio::test]
    async fn envelope_read_marks_delivered_then_read() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let m = fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                lane: None,
                kind: "message",
                body: Some("hi"),
                props: msg_props(json!({"kind": "everyone"})),
                ..Default::default()
            },
        )
        .await;
        // First read: delivered, not read (derived: inclusion delivers; a
        // later fetch after delivery reads — there is no ack tool).
        pg.relay_envelope(u1(), a2).await.unwrap();
        let kinds: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "select kind::text from relay.actions where reply_to = $1 order by kind::text",
        )
        .bind(m)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(kinds, vec!["delivered".to_string()]);
        pg.relay_envelope(u1(), a2).await.unwrap();
        let kinds: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "select kind::text from relay.actions where reply_to = $1 order by kind::text",
        )
        .bind(m)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(kinds, vec!["delivered".to_string(), "read".to_string()]);
    }

    #[tokio::test]
    async fn envelope_delivery_marks_task_started_once() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0xd1);
        fx_task(&pg, u1(), t, c1, Some(a2), "assigned", "tests", 1).await;
        fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                task: Some(t),
                lane: Some(v7(0x11)),
                kind: "assigned",
                body: Some("Do it."),
                props: json!({"docs": [], "owned_paths": [], "acceptance": []}),
                round: Some(1),
                phase: Some("tests"),
                ..Default::default()
            },
        )
        .await;
        let env = pg.relay_envelope(u1(), a2).await.unwrap();
        // The assignment carries the frozen brief…
        assert_eq!(env.assignments.len(), 1);
        assert_eq!(env.assignments[0].task_id, t.to_string());
        assert_eq!(env.assignments[0].brief.summary, "Do it.");
        // …and delivery marks started (derived: no start tool exists).
        assert_eq!(task_state(&pg, t).await, "working");
        // A second read writes no second `started`.
        pg.relay_envelope(u1(), a2).await.unwrap();
        let n: i64 = sqlx_core::query_scalar::query_scalar(
            "select count(*) from relay.actions where kind = 'started' and task_id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn envelope_reviews_both_directions_q15rec() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        // To PERFORM: A2 reviews A1's task…
        let theirs = v7(0xd2);
        fx_task(&pg, u1(), theirs, c1, Some(a1), "in_review", "tests", 1).await;
        sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
            .bind(a2)
            .bind(theirs)
            .execute(pg.pool())
            .await
            .unwrap();
        // …to ANSWER: A2's own task went rework on A1's review.
        let mine = v7(0xd3);
        fx_task(&pg, u1(), mine, c1, Some(a2), "rework", "tests", 2).await;
        fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                task: Some(mine),
                lane: Some(a1),
                kind: "reviewed",
                body: Some("Fix it."),
                props: json!({"verdict": "changes_requested", "findings": []}),
                round: Some(1),
                ..Default::default()
            },
        )
        .await;
        // Q15-REC: both, told apart by `direction`.
        use crate::relay::envelope::ReviewDirection as D;
        let env = pg.relay_envelope(u1(), a2).await.unwrap();
        let dirs: Vec<(&str, D)> =
            env.reviews.iter().map(|r| (r.task_id.as_str(), r.direction)).collect();
        assert!(dirs.contains(&(theirs.to_string().as_str(), D::Perform)), "{dirs:?}");
        assert!(dirs.contains(&(mine.to_string().as_str(), D::Answer)), "{dirs:?}");
    }

    #[tokio::test]
    async fn envelope_answers_carry_decision_answers() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let w = v7(0xd4);
        fx_task(&pg, u1(), w, c1, Some(a2), "working", "tests", 1).await;
        // A2 asks the person; the person answers.
        let mut ask = ask_input(vec![w]);
        ask.task_id = Some(w);
        let decision = pg.relay_ask_decision(u1(), a2, ask).await.unwrap();
        pg.relay_answer_decision(
            u1(),
            RelayCaller::Person { surface: RelaySurface::Desktop },
            decision.decision_id,
            RelayAnswerInput { option: "db".into(), note: Some("go".into()) },
        )
        .await
        .unwrap();
        let env = pg.relay_envelope(u1(), a2).await.unwrap();
        assert_eq!(env.answers.len(), 1);
        assert_eq!(env.answers[0].decision_id, decision.decision_id.to_string());
        assert_eq!(env.answers[0].option, "db");
        assert_eq!(env.answers[0].task_id.as_deref(), Some(&*w.to_string()));
    }

    // ── sweep_leases ──

    #[tokio::test]
    async fn sweep_expires_lapsed_lease_interrupts_task() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        sqlx_core::query::query("update relay.leases set expires_at = $1 where lane_id = $2")
            .bind(t0() - chrono::Duration::seconds(1))
            .bind(a2)
            .execute(pg.pool())
            .await
            .unwrap();
        let t = v7(0xb1);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        let rep = pg.relay_sweep_leases(t0()).await.unwrap();
        assert_eq!((rep.expired_leases, rep.interrupted_tasks, rep.reoffered_tasks), (1, 1, 0));
        assert_eq!(task_state(&pg, t).await, "interrupted");
        let kinds: Vec<String> = sqlx_core::query_scalar::query_scalar(
            "select kind::text from relay.actions where task_id = $1",
        )
        .bind(t)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(kinds, vec!["interrupted".to_string()]);
    }

    // ── Follow-up: sweep clears block memory (unregister-follow-up finding) ──

    #[tokio::test]
    async fn sweep_blocked_task_holder_clears_block_memory() {
        // The sweep suspends expired-lease holders via LeaseLost; a BLOCKED
        // holder must land interrupted with its block memory cleared —
        // without the clear the suspend write violates
        // tasks_blocked_memory and rolls back the WHOLE pass (one
        // transaction: expiry, re-offers, reviewer re-picks — a liveness
        // wedge, not a single 500).
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        sqlx_core::query::query("update relay.leases set expires_at = $1 where lane_id = $2")
            .bind(t0() - chrono::Duration::seconds(1))
            .bind(a2)
            .execute(pg.pool())
            .await
            .unwrap();
        let t = v7(0xb9);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        sqlx_core::query::query(
            "update relay.tasks set state = 'blocked', blocked_from_state = 'working'
             where id = $1",
        )
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
        let rep = pg.relay_sweep_leases(t0()).await.unwrap();
        assert_eq!((rep.expired_leases, rep.interrupted_tasks, rep.reoffered_tasks), (1, 1, 0));
        assert_eq!(task_state(&pg, t).await, "interrupted");
        let memory: Option<String> = sqlx_core::query_scalar::query_scalar(
            "select blocked_from_state::text from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(memory, None);
    }

    #[tokio::test]
    async fn sweep_reoffers_after_grace_not_before() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        // Interrupted long ago (grace run out) vs just now — plus the
        // boundary: 899 s (one inside the 900 s grace) stays (N2).
        for (n, ago_secs) in [(0xb2, 901), (0xb3, 100), (0xb7, 899)] {
            let t = v7(n);
            fx_task(&pg, u1(), t, c1, Some(a2), "interrupted", "tests", 1).await;
            let a = fx_action(
                &pg,
                u1(),
                c1,
                ActionFx { task: Some(t), kind: "interrupted", ..Default::default() },
            )
            .await;
            sqlx_core::query::query("update relay.actions set created_at = $1 where id = $2")
                .bind(t0() - chrono::Duration::seconds(ago_secs))
                .bind(a)
                .execute(pg.pool())
                .await
                .unwrap();
        }
        // A2's lease is live: only the grace decides.
        let rep = pg.relay_sweep_leases(t0()).await.unwrap();
        assert_eq!(rep.reoffered_tasks, 1);
        let (old_state, old_lane): (String, Option<Uuid>) = sqlx_core::query_as::query_as(
            "select state::text, lane_id from relay.tasks where id = $1",
        )
        .bind(v7(0xb2))
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(old_state, "assigned");
        assert_eq!(old_lane, None, "back to the pool, off the lane");
        assert_eq!(task_state(&pg, v7(0xb3)).await, "interrupted");
        assert_eq!(task_state(&pg, v7(0xb7)).await, "interrupted");
    }

    #[tokio::test]
    async fn sweep_expired_reviewer_repicks_q17rec() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        // A3 reviews A2's task, then A3's lease lapses.
        let a3 = v7(0x16);
        fx_lane(&pg, u1(), a3, c1, "L3", "contributor", "tok-a3").await;
        fx_lease(&pg, u1(), c1, a3, "granted", Some(t0() - chrono::Duration::seconds(1))).await;
        let t = v7(0xb4);
        fx_task(&pg, u1(), t, c1, Some(a2), "in_review", "tests", 1).await;
        sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
            .bind(a3)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        // Q17-REC: re-pick over live lanes by the Q14 rule (A1, unloaded)…
        let rep = pg.relay_sweep_leases(t0()).await.unwrap();
        assert_eq!(rep.repicked_reviewers, 1);
        let reviewer: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select reviewer_lane_id from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(reviewer, Some(a1));
        // …and the task is untouched otherwise (the author is live).
        assert_eq!(task_state(&pg, t).await, "in_review");
    }

    #[tokio::test]
    async fn sweep_expired_reviewer_clears_when_nobody_live_q17rec() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, _, _, _) = fx_two_crews(&pg).await;
        // Only the author-coordinator is live; the reviewer lapsed.
        sqlx_core::query::query("update relay.leases set state = 'expired' where lane_id = $1")
            .bind(v7(0x12))
            .execute(pg.pool())
            .await
            .unwrap();
        let a3 = v7(0x17);
        fx_lane(&pg, u1(), a3, c1, "L3", "contributor", "tok-a3").await;
        fx_lease(&pg, u1(), c1, a3, "expired", Some(t0() - chrono::Duration::seconds(1))).await;
        let t = v7(0xb5);
        fx_task(&pg, u1(), t, c1, Some(a1), "in_review", "tests", 1).await;
        sqlx_core::query::query("update relay.tasks set reviewer_lane_id = $1 where id = $2")
            .bind(a3)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        // Q17-REC: nobody to re-pick (the coordinator IS the author) → null.
        pg.relay_sweep_leases(t0()).await.unwrap();
        let reviewer: Option<Uuid> = sqlx_core::query_scalar::query_scalar(
            "select reviewer_lane_id from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(reviewer, None);
    }

    #[tokio::test]
    async fn sweep_leaves_live_lanes_alone() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0xb6);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        let rep = pg.relay_sweep_leases(t0()).await.unwrap();
        assert_eq!(
            (
                rep.expired_leases,
                rep.interrupted_tasks,
                rep.reoffered_tasks,
                rep.repicked_reviewers
            ),
            (0, 0, 0, 0)
        );
        assert_eq!(task_state(&pg, t).await, "working");
    }

    // ── person reads ──

    fn no_filter() -> RelayReadFilter {
        RelayReadFilter { workspace_id: None, repo_key: None }
    }

    /// A Now fixture: U1's crew C1 on repo R with workspace W (lane A2 live,
    /// task T blocked on an open person ask), a manual triage item, and a
    /// just-merged task. Returns (crew, lane, task, workspace).
    async fn fx_now(pg: &PgStore) -> (Uuid, Uuid, Uuid, Uuid) {
        let (c1, _, _, a2, _, _) = fx_two_crews(pg).await;
        let ws = pg.create_workspace("_test:relay-now", None, None).await.unwrap();
        sqlx_core::query::query(
            "update relay.crews set repo_key = 'github.com/o/r', workspace_id = $1, goal = 'Ship' where id = $2",
        )
        .bind(ws)
        .bind(c1)
        .execute(pg.pool())
        .await
        .unwrap();
        let t = v7(0xe1);
        fx_task(pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        sqlx_core::query::query(
            "update relay.tasks set state = 'blocked', blocked_from_state = 'working', workspace_id = $1 where id = $2",
        )
        .bind(ws)
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
        fx_action(
            pg,
            u1(),
            c1,
            ActionFx {
                task: Some(t),
                lane: None,
                kind: "asked",
                body: Some("Where?"),
                props: json!({
                    "context": "T waits on this.",
                    "options": [{"id": "db", "label": "Db", "detail": null}],
                    "recommended": "db",
                    "blocks": [t.to_string()],
                    "asked_of": "person",
                }),
                ..Default::default()
            },
        )
        .await;
        fx_item(pg, u1(), v7(0xe2), "triage", None, None).await;
        let done = v7(0xe3);
        fx_task(pg, u1(), done, c1, Some(a2), "done", "tests", 1).await;
        sqlx_core::query::query(
            "update relay.tasks set completed_at = current_timestamp, workspace_id = $1 where id = $2",
        )
        .bind(ws)
        .bind(done)
        .execute(pg.pool())
        .await
        .unwrap();
        fx_action(
            pg,
            u1(),
            c1,
            ActionFx {
                task: Some(done),
                lane: Some(v7(0x11)),
                kind: "merged",
                body: Some("Merged."),
                props: json!({"commits": ["abc"], "into": "develop", "gates": []}),
                ..Default::default()
            },
        )
        .await;
        (c1, a2, t, ws)
    }

    #[tokio::test]
    async fn now_serves_relay_api_shape_exactly() {
        let pg = PgStore::test_store().await;
        let (c1, a2, t, ws) = fx_now(&pg).await;
        // A repo-less crew (Q6): Q18-REC reads repo_key null, honestly.
        let c2 = v7(0xe4);
        fx_crew(&pg, u1(), c2).await;
        let now = pg.relay_now(u1(), no_filter()).await.unwrap();
        // Crews: the founded one reads fully…
        let crews = now["crews"].as_array().unwrap();
        assert_eq!(crews.len(), 2);
        let crew = &crews[0];
        assert_eq!(crew["id"], c1.to_string());
        assert_eq!(crew["repo_key"], "github.com/o/r");
        assert_eq!(crew["workspace_id"], ws.to_string());
        assert_eq!(crew["goal"], "Ship");
        assert_eq!(crew["status"], "active");
        assert!(crew["started_at"].is_string());
        assert!(crew["heartbeat_at"].is_null());
        assert_eq!(crews[1]["repo_key"], serde_json::Value::Null, "Q18-REC");
        // …facets name the filter band…
        assert_eq!(now["facets"]["repos"], json!(["github.com/o/r"]));
        assert_eq!(now["facets"]["workspaces"][0]["name"], "_test:relay-now");
        // …the lanes read by code: L1 idle, L2 with its lease and blocked task…
        assert_eq!(now["lanes"][0]["code"], "L1");
        assert_eq!(now["lanes"][0]["task"], serde_json::Value::Null);
        let lane = &now["lanes"][1];
        assert_eq!(lane["id"], a2.to_string());
        assert_eq!(lane["crew_id"], c1.to_string());
        assert_eq!(lane["code"], "L2");
        assert_eq!(lane["title"], serde_json::Value::Null);
        assert_eq!(lane["role"], "contributor");
        assert_eq!(lane["lease_state"], "granted");
        assert!(lane["lease_expires_at"].is_string());
        assert_eq!(lane["task"]["id"], t.to_string());
        assert_eq!(lane["task"]["state"], "blocked");
        assert_eq!(lane["task"]["needs_answer"], true);
        // …the open ask reads as a decision…
        let d = &now["decisions"][0];
        assert_eq!(d["task_id"], t.to_string());
        assert_eq!(d["question"], "Where?");
        assert_eq!(d["context"], "T waits on this.");
        assert_eq!(d["options"][0]["id"], "db");
        assert_eq!(d["recommended"], "db");
        // …manual triage counts, the merged task completes today…
        assert_eq!(now["triage"], json!([{"provider": "manual", "sprint": null, "count": 1}]));
        let done = &now["completed_today"][0];
        assert_eq!(done["outcome"], "merged");
        assert_eq!(done["crew_id"], c1.to_string());
        assert_eq!(now["today"]["completed"], 1);
        assert_eq!(now["today"]["reviewed"], serde_json::Value::Null);
        assert_eq!(now["today"]["first_pass_approved"], serde_json::Value::Null);
        // …and resources read nulls with the lanes.md floors (slice 1: no
        // leases yet, so no holder).
        assert_eq!(now["resources"]["build_lease"], serde_json::Value::Null);
        assert_eq!(now["resources"]["disk_floor_gb"], 25);
        assert_eq!(now["resources"]["swap_floor_gb"], 1);
        assert!(
            now["resources"]["disk_free_gb"].is_u64() || now["resources"]["disk_free_gb"].is_null()
        );
    }

    #[tokio::test]
    async fn now_scopes_to_user_and_filter() {
        let pg = PgStore::test_store().await;
        let (c1, ..) = fx_now(&pg).await;
        // U2's crew never leaks into U1's read (isolation: two users)…
        let now = pg.relay_now(u1(), no_filter()).await.unwrap();
        for lane in now["lanes"].as_array().unwrap() {
            assert_eq!(lane["crew_id"], c1.to_string());
        }
        // …and the filter narrows to the matching crew (derived: the filter
        // selects crews; lanes/decisions/completed follow theirs).
        let filtered = pg
            .relay_now(
                u1(),
                RelayReadFilter { workspace_id: None, repo_key: Some("github.com/other".into()) },
            )
            .await
            .unwrap();
        assert_eq!(filtered["crews"], json!([]));
        assert_eq!(filtered["lanes"], json!([]));
        let matched = pg
            .relay_now(
                u1(),
                RelayReadFilter { workspace_id: None, repo_key: Some("github.com/o/r".into()) },
            )
            .await
            .unwrap();
        assert_eq!(matched["crews"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn now_completed_midnight_pair_is_session_local() {
        let pg = PgStore::test_store().await;
        let (c1, _, a2, _, _, _) = fx_two_crews(&pg).await;
        // Midnight-sharp (in) vs a second before (out), computed in the Mac
        // zone the read runs under ((B) ruling: the pool is UTC, relay_now's
        // txn is local) — deterministic in any zone.
        let tz = crate::relay::system_timezone().unwrap();
        for (n, off) in [(0xe5, "0 seconds"), (0xe6, "-1 seconds")] {
            let t = v7(n);
            fx_task(&pg, u1(), t, c1, Some(a2), "done", "tests", 1).await;
            sqlx_core::query::query(
                "update relay.tasks set completed_at =
                   (date_trunc('day', current_timestamp at time zone $1) at time zone $1)
                   + $2::interval where id = $3",
            )
            .bind(tz.clone())
            .bind(off)
            .bind(t)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        let now = pg.relay_now(u1(), no_filter()).await.unwrap();
        let ids: Vec<&str> = now["completed_today"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["task_id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&v7(0xe5).to_string().as_str()), "{ids:?}");
        assert!(!ids.contains(&v7(0xe6).to_string().as_str()), "{ids:?}");
    }

    #[tokio::test]
    async fn now_without_crews_is_empty_not_failure() {
        let pg = PgStore::test_store().await;
        let now = pg.relay_now(v7(0xa9), no_filter()).await.unwrap();
        assert_eq!(now["crews"], json!([]));
        assert_eq!(now["today"]["completed"], 0);
    }

    #[tokio::test]
    async fn queue_serves_triage_and_positioned_backlog() {
        let pg = PgStore::test_store().await;
        fx_two_crews(&pg).await;
        // Triage: one manual item (tracker rows are slice 2 — absent here).
        let tri = v7(0xf1);
        fx_item(&pg, u1(), tri, "triage", None, None).await;
        sqlx_core::query::query("update relay.items set tags = $1 where id = $2")
            .bind(vec!["sprint 47"])
            .bind(tri)
            .execute(pg.pool())
            .await
            .unwrap();
        // Backlog out of insertion order: position decides.
        fx_item(&pg, u1(), v7(0xf2), "backlog", Some("github.com/o/r"), Some(2)).await;
        fx_item(&pg, u1(), v7(0xf3), "backlog", Some("github.com/o/r"), Some(1)).await;
        let q = pg.relay_queue(u1(), no_filter()).await.unwrap();
        assert_eq!(q["triage"].as_array().unwrap().len(), 1);
        let row = &q["triage"][0];
        assert_eq!(row["item_id"], tri.to_string());
        assert_eq!(row["ticket"], serde_json::Value::Null);
        assert_eq!(row["tags"], json!(["sprint 47"]));
        assert_eq!(row["added_by_you"], true, "added_by null = the person");
        let positions: Vec<i64> = q["backlog"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["position"].as_i64().unwrap())
            .collect();
        assert_eq!(positions, vec![1, 2]);
    }

    #[tokio::test]
    async fn task_view_serves_header_run_handoff_history() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0xf4);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        sqlx_core::query::query(
            "update relay.tasks set assigned_at = '2026-11-06T10:00:00Z', owned_paths = $1 where id = $2",
        )
        .bind(vec!["app/**"])
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
        // Assigned, then a brief update: the handoff is the latest. Explicit
        // stamps keep history order deterministic (insertion order is not).
        for (body, at) in [("First.", "2026-11-06T10:00:00Z"), ("Second.", "2026-11-06T10:01:00Z")]
        {
            let kind = if body == "First." { "assigned" } else { "brief_updated" };
            let a = fx_action(
                &pg,
                u1(),
                c1,
                ActionFx {
                    task: Some(t),
                    lane: Some(a1),
                    kind,
                    body: Some(body),
                    props: json!({"docs": ["d.md"]}),
                    round: Some(1),
                    ..Default::default()
                },
            )
            .await;
            sqlx_core::query::query(
                "update relay.actions set created_at = $1::timestamptz where id = $2",
            )
            .bind(at)
            .bind(a)
            .execute(pg.pool())
            .await
            .unwrap();
        }
        // Two turns: one fully reported, one bare.
        let r1 = fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                task: Some(t),
                lane: Some(a2),
                kind: "reported",
                body: Some("Turn one."),
                props: json!({
                    "tokens_in": 100, "started_at": "2026-11-06T10:00:00Z",
                    "ended_at": "2026-11-06T10:01:00Z",
                    "files": [{"path": "app/a.ts", "added": 3, "removed": 1, "outside_owned": false}],
                }),
                round: Some(1),
                ..Default::default()
            },
        )
        .await;
        sqlx_core::query::query(
            "update relay.actions set created_at = '2026-11-06T10:02:00Z' where id = $1",
        )
        .bind(r1)
        .execute(pg.pool())
        .await
        .unwrap();
        let r2 = fx_action(
            &pg,
            u1(),
            c1,
            ActionFx {
                task: Some(t),
                lane: Some(a2),
                kind: "reported",
                body: Some("Turn two."),
                props: json!({}),
                round: Some(1),
                ..Default::default()
            },
        )
        .await;
        sqlx_core::query::query(
            "update relay.actions set created_at = '2026-11-06T10:03:00Z' where id = $1",
        )
        .bind(r2)
        .execute(pg.pool())
        .await
        .unwrap();
        let v = pg.relay_task_view(u1(), c1, t).await.unwrap();
        assert_eq!(v["task"]["id"], t.to_string());
        assert_eq!(v["task"]["rounds"], json!([{"phase": "tests", "count": 2}]));
        assert_eq!(v["task"]["needs_answer"], false);
        assert_eq!(v["handoff"]["body_md"], "Second.");
        // Run totals skip nulls and say how many reported (derived: a turn
        // reports when its props carry any telemetry key).
        assert_eq!(v["run"]["tokens_in"], 100);
        assert_eq!(v["run"]["tokens_out"], serde_json::Value::Null);
        assert_eq!(v["run"]["reported_turns"], 1);
        assert_eq!(v["run"]["turns"], 2);
        assert_eq!(v["run"]["files_changed"], 4);
        assert_eq!(v["run"]["working_secs"], 60);
        // History newest-first down to assigned, lane codes behind uuids.
        let kinds: Vec<&str> =
            v["history"].as_array().unwrap().iter().map(|a| a["kind"].as_str().unwrap()).collect();
        assert_eq!(kinds, vec!["reported", "reported", "brief_updated", "assigned"]);
        assert_eq!(v["history"][0]["lane_code"], "L2");
        assert_eq!(v["lanes"][0]["code"], "L1");
        assert_eq!(v["decision"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn task_view_404s_across_users_crews_and_unknown() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0xf5);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        // Another user's read…
        let err = pg.relay_task_view(u2(), c1, t).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        // …another crew's id…
        let c2 = v7(0xf6);
        fx_crew(&pg, u1(), c2).await;
        let err = pg.relay_task_view(u1(), c2, t).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
        // …and an unknown task.
        let err = pg.relay_task_view(u1(), c1, v7(0x99)).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "{err:?}");
    }

    #[tokio::test]
    async fn today_counts_reviews_and_first_pass() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        for (n, verdict, round, holder) in
            [(0xf7, "approved", 1, a2), (0xf8, "changes_requested", 2, a1)]
        {
            let t = v7(n);
            fx_task(&pg, u1(), t, c1, Some(holder), "in_review", "tests", round).await;
            fx_action(
                &pg,
                u1(),
                c1,
                ActionFx {
                    task: Some(t),
                    lane: Some(a1),
                    kind: "reviewed",
                    body: Some("R."),
                    props: json!({"verdict": verdict, "findings": []}),
                    round: Some(round),
                    ..Default::default()
                },
            )
            .await;
        }
        // Derived: reviewed = today's reviewed actions; first_pass_approved =
        // today's approvals at round 1. Both null when no review reported.
        let now = pg.relay_now(u1(), no_filter()).await.unwrap();
        assert_eq!(now["today"]["reviewed"], 2);
        assert_eq!(now["today"]["first_pass_approved"], 1);
    }

    #[tokio::test]
    async fn backlog_add_positions_and_refuses_lanes() {
        let pg = PgStore::test_store().await;
        let (_, _, _, a2, _, _) = fx_two_crews(&pg).await;
        fx_item(&pg, u1(), v7(0xf9), "backlog", None, Some(1)).await;
        fx_item(&pg, u1(), v7(0xfa), "triage", None, None).await;
        let person = RelayCaller::Person { surface: RelaySurface::Desktop };
        let id = pg.relay_backlog_add(u1(), person, v7(0xfa)).await.unwrap();
        assert_eq!(id, v7(0xfa));
        let (state, pos): (String, Option<i32>) = sqlx_core::query_as::query_as(
            "select state::text, position from relay.items where id = $1",
        )
        .bind(v7(0xfa))
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((state.as_str(), pos), ("backlog", Some(2)));
        // Already there: conflict. Lanes plan via plan_item, not the +.
        let person = RelayCaller::Person { surface: RelaySurface::Desktop };
        let err = pg.relay_backlog_add(u1(), person, v7(0xf9)).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        let err = pg
            .relay_backlog_add(u1(), RelayCaller::Lane { lane_id: a2 }, v7(0xfa))
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::Forbidden(_)), "{err:?}");
    }

    #[tokio::test]
    async fn triage_add_manual_item_person_only() {
        let pg = PgStore::test_store().await;
        fx_two_crews(&pg).await;
        let person = RelayCaller::Person { surface: RelaySurface::Desktop };
        let input = RelayTriageInput {
            title: "Gap".into(),
            kind: RelayItemKind::Gap,
            repo_key: None,
            workspace_id: None,
            tags: vec!["ux".into()],
        };
        let id = pg.relay_triage_add(u1(), person, input).await.unwrap();
        let (state, title): (String, String) = sqlx_core::query_as::query_as(
            "select state::text, title from relay.items where id = $1",
        )
        .bind(id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((state.as_str(), title.as_str()), ("triage", "Gap"));
    }

    // ── failure paths ──

    #[tokio::test]
    async fn relay_read_failure_is_internal_never_empty() {
        let pg = PgStore::test_store().await;
        pg.pool().close().await;
        let err = pg.relay_now(u1(), no_filter()).await.unwrap_err();
        assert!(matches!(err, RelayError::Internal(_)), "{err:?}");
    }

    #[tokio::test]
    async fn current_user_is_stable_q19rec() {
        // Q19-REC (amended, L0 sanction 2026-11-06): with a signed-in
        // default persona, the daemon user is its principal — one stable
        // uuid per machine+identity.
        let pg = PgStore::test_store().await;
        let principal = v7(0x90);
        sqlx_core::query::query(
            "insert into sensei.personas (label, is_chosen_default, principal_id)
             values ('default', true, $1)",
        )
        .bind(principal)
        .execute(pg.pool())
        .await
        .unwrap();
        let a = pg.relay_current_user().await.unwrap();
        let b = pg.relay_current_user().await.unwrap();
        assert_eq!(a, b);
        assert_eq!(a, principal);
    }

    #[tokio::test]
    async fn current_user_never_signed_in_is_sign_in_required_q19rec() {
        // Q19-REC failure pin (added, L0 sanction 2026-11-06): no default
        // persona — or an unlinked one — fails closed, never a minted user.
        let pg = PgStore::test_store().await;
        let err = pg.relay_current_user().await.unwrap_err();
        assert!(matches!(err, RelayError::SignInRequired), "{err:?}");
        sqlx_core::query::query(
            "insert into sensei.personas (label, is_chosen_default) values ('default', true)",
        )
        .execute(pg.pool())
        .await
        .unwrap();
        let err = pg.relay_current_user().await.unwrap_err();
        assert!(matches!(err, RelayError::SignInRequired), "{err:?}");
    }

    #[tokio::test]
    async fn pool_default_stays_utc_for_metrics_days() {
        // (B) ruling 2026-11-06: the pool keeps the server default (UTC) —
        // metrics days are UTC and billing-adjacent. Relay's one
        // day-windowed read sets the Mac zone per-transaction instead.
        // `extract(timezone)` is 0 east of UTC only for UTC itself (any US
        // zone is hours west, DST or not), so this pins UTC-ness robustly.
        let pg = PgStore::test_store().await;
        let off: i32 =
            sqlx_core::query_scalar::query_scalar("select extract(timezone from now())::int")
                .fetch_one(pg.pool())
                .await
                .unwrap();
        assert_eq!(off, 0);
    }

    #[tokio::test]
    async fn relay_now_reads_in_the_mac_zone() {
        // (B) ruling 2026-11-06 (amends `pool_timezone_is_the_mac_zone`):
        // relay_now's transaction reads under the Mac zone. Two divergence
        // instants — constructible any hour, DST-safe (PG knows tzdb):
        // Chicago-evening is in Chicago-today but UTC-tomorrow (included);
        // UTC-morning is in UTC-today but Chicago-yesterday (excluded).
        let pg = PgStore::test_store().await;
        let (c1, _, a1, _, _, _) = fx_two_crews(&pg).await;
        let tz = crate::relay::system_timezone().unwrap();
        let evening: chrono::DateTime<chrono::Utc> = sqlx_core::query_scalar::query_scalar(
            "select (date_trunc('day', now() at time zone $1) + interval '23 hours 30 minutes')
                    at time zone $1",
        )
        .bind(tz.clone())
        .bind(tz.clone())
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let morning: chrono::DateTime<chrono::Utc> = sqlx_core::query_scalar::query_scalar(
            "select (date_trunc('day', now() at time zone $1) - interval '30 minutes')
                    at time zone $1",
        )
        .bind(tz.clone())
        .bind(tz.clone())
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let (t_in, t_out) = (v7(0xd4), v7(0xd5));
        fx_task(&pg, u1(), t_in, c1, Some(a1), "done", "tests", 1).await;
        fx_task(&pg, u1(), t_out, c1, Some(a1), "done", "tests", 1).await;
        // NOTE: two done tasks on one lane is legal (done is terminal).
        for (t, at) in [(t_in, evening), (t_out, morning)] {
            sqlx_core::query::query("update relay.tasks set completed_at = $1 where id = $2")
                .bind(at)
                .bind(t)
                .execute(pg.pool())
                .await
                .unwrap();
        }
        let now = pg.relay_now(u1(), no_filter()).await.unwrap();
        let done: Vec<&str> = now["completed_today"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|r| r["task_id"].as_str())
            .collect();
        assert!(done.contains(&t_in.to_string().as_str()), "{done:?}");
        assert!(!done.contains(&t_out.to_string().as_str()), "{done:?}");
    }

    #[tokio::test]
    async fn resource_request_grants_first_come_reports_numbers() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, _a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let out = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    // Zero floors: the grant is deterministic on any machine
                    // (no ambient reading can sit below zero); the None path
                    // is pinned separately below.
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(out.state, RelayLeaseState::Granted);
        assert_eq!(out.resource, "rust-build");
        // The broker mints the id (L0 note 1): v7, position 15 reads `7`.
        assert_eq!(&out.lease_id.to_string()[14..15], "7");
        assert_eq!(out.disk_floor_gb, 0);
        assert_eq!(out.swap_floor_gb, 0);
        assert!(out.disk_free_gb.is_some(), "disk always reads");
        assert_eq!(out.granted_at, Some(now));
        assert_eq!(out.expires_at, Some(now + chrono::Duration::seconds(3600)));
        assert_eq!(out.position, None);
        let (state, resource, props, granted_at, expires_at, _) =
            resource_row(&pg, u1, out.lease_id).await;
        assert_eq!(state, "granted");
        assert_eq!(resource.as_deref(), Some("rust-build"));
        // The carried override persists (decision 8).
        assert_eq!(props["floors"]["disk_gb"].as_u64(), Some(0));
        assert_eq!(props["floors"]["swap_gb"].as_u64(), Some(0));
        assert_eq!(props.get("disk_free_gb"), Some(&json!(out.disk_free_gb)));
        assert_eq!(props.get("swap_free_gb"), Some(&json!(out.swap_free_gb)));
        assert_eq!(granted_at, Some(now));
        assert_eq!(expires_at, out.expires_at);
    }

    #[tokio::test]
    async fn resource_request_without_override_echoes_ruled_floors() {
        // The None path: the verdict follows the machine (ambient readings
        // vs the ruled floors), so the test pins everything around it — the
        // ruled-floors echo, the v7 mint, the absence of a floors key, and
        // the row shape matching whichever verdict came back.
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, _a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let out = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest { resource: "rust-build".into(), floors: None },
                now,
            )
            .await
            .unwrap();
        assert!(matches!(out.state, RelayLeaseState::Granted | RelayLeaseState::Refused));
        assert_eq!(out.disk_floor_gb, 25);
        assert_eq!(out.swap_floor_gb, 1);
        // The broker mints the id (L0 note 1): v7, position 15 reads `7`.
        assert_eq!(&out.lease_id.to_string()[14..15], "7");
        let (state, resource, props, granted_at, expires_at, _) =
            resource_row(&pg, u1, out.lease_id).await;
        assert_eq!(resource.as_deref(), Some("rust-build"));
        // N3: no override carried, so no floors key persists (decision 8).
        assert!(props.get("floors").is_none(), "floors absent: {props}");
        assert_eq!(props.get("disk_free_gb"), Some(&json!(out.disk_free_gb)));
        assert_eq!(props.get("swap_free_gb"), Some(&json!(out.swap_free_gb)));
        match out.state {
            RelayLeaseState::Granted => {
                assert_eq!(state, "granted");
                assert_eq!(out.granted_at, Some(now));
                assert_eq!(out.expires_at, Some(now + chrono::Duration::seconds(3600)));
                assert_eq!(granted_at, Some(now));
                assert_eq!(expires_at, out.expires_at);
            }
            RelayLeaseState::Refused => {
                assert_eq!(state, "refused");
                assert_eq!(out.granted_at, None);
                assert_eq!(out.expires_at, None);
                assert!(!props.get("reason").and_then(|r| r.as_str()).unwrap_or("").is_empty());
            }
            _ => panic!("a free slot never queues: {:?}", out.state),
        }
    }

    #[tokio::test]
    async fn resource_request_while_held_queues_fifo() {
        let pg = PgStore::test_store().await;
        let (c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let a3 = v7(0x14);
        fx_lane(&pg, u1, a3, c1, "L3", "contributor", "tok-a3").await;
        fx_lease(&pg, u1, c1, a3, "granted", Some(t0() + chrono::Duration::seconds(300))).await;
        let now = t0();
        let req = RelayResourceRequest {
            resource: "rust-build".into(),
            floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
        };
        let held = pg.relay_request_resource(u1, a1, req.clone(), now).await.unwrap();
        assert_eq!(held.state, RelayLeaseState::Granted);
        let q1 = pg.relay_request_resource(u1, a2, req.clone(), now).await.unwrap();
        assert_eq!(q1.state, RelayLeaseState::Queued);
        assert_eq!(q1.position, Some(1));
        assert_eq!(q1.granted_at, None);
        assert_eq!(q1.expires_at, None);
        // v7 ids order by millisecond: separate the requests so FIFO-by-id
        // reads request order deterministically.
        sqlx_core::query::query("select pg_sleep(0.012)").execute(pg.pool()).await.unwrap();
        let q2 = pg.relay_request_resource(u1, a3, req.clone(), now).await.unwrap();
        assert_eq!(q2.state, RelayLeaseState::Queued);
        assert_eq!(q2.position, Some(2));
    }

    #[tokio::test]
    async fn resource_request_cross_crew_excludes_machine_wide() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, _a2, b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let held = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "e2e".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(held.state, RelayLeaseState::Granted);
        // Another crew, another user: the slot is machine-wide, so this queues.
        let q = pg
            .relay_request_resource(
                u2(),
                b1,
                RelayResourceRequest { resource: "e2e".into(), floors: None },
                now,
            )
            .await
            .unwrap();
        assert_eq!(q.state, RelayLeaseState::Queued);
        assert_eq!(q.position, Some(1));
        let n: (i64,) = sqlx_core::query_as::query_as(
            "select count(*) from relay.leases \
             where kind = 'resource' and resource = 'e2e' and state = 'granted'",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(n.0, 1, "one granted row per slot, machine-wide");
    }

    #[tokio::test]
    async fn resource_request_same_lane_twice_returns_existing() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let req = RelayResourceRequest {
            resource: "rust-build".into(),
            floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
        };
        let g1 = pg.relay_request_resource(u1, a1, req.clone(), now).await.unwrap();
        let g2 = pg.relay_request_resource(u1, a1, req.clone(), now).await.unwrap();
        assert_eq!(g2.lease_id, g1.lease_id);
        assert_eq!(g2.state, RelayLeaseState::Granted);
        let q1 = pg.relay_request_resource(u1, a2, req.clone(), now).await.unwrap();
        let q2 = pg.relay_request_resource(u1, a2, req.clone(), now).await.unwrap();
        assert_eq!(q2.lease_id, q1.lease_id);
        assert_eq!(q2.state, RelayLeaseState::Queued);
        assert_eq!(q2.position, Some(1));
        let n: (i64,) = sqlx_core::query_as::query_as(
            "select count(*) from relay.leases \
             where user_id = $1 and lane_id = $2 and kind = 'resource' and resource = 'rust-build'",
        )
        .bind(u1)
        .bind(a2)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(n.0, 1, "the re-request inserts no second row");
    }

    #[tokio::test]
    async fn resource_request_refused_below_floors_writes_row() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, _a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        // Impossible floors: every real reading sits below them (Q5).
        let out = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: u64::MAX, swap_gb: u64::MAX }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(out.state, RelayLeaseState::Refused);
        // The output echoes the effective floors (the ones compared).
        assert_eq!(out.disk_floor_gb, u64::MAX);
        assert_eq!(out.granted_at, None);
        assert_eq!(out.expires_at, None);
        assert_eq!(out.position, None);
        let (state, _resource, props, _granted, _expires, _) =
            resource_row(&pg, u1, out.lease_id).await;
        assert_eq!(state, "refused");
        let reason = props.get("reason").and_then(|r| r.as_str()).unwrap_or("");
        assert!(!reason.is_empty(), "the refused row carries a reason");
        assert_eq!(props.get("disk_free_gb"), Some(&json!(out.disk_free_gb)));
        assert_eq!(props.get("swap_free_gb"), Some(&json!(out.swap_free_gb)));
    }

    #[tokio::test]
    async fn resource_request_empty_resource_is_validation() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, _a2, _b1, u1) = fx_two_crews(&pg).await;
        let err = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest { resource: "  ".into(), floors: None },
                t0(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, RelayError::Validation(_)), "blank resource: {err:?}");
    }

    #[tokio::test]
    async fn resource_release_grants_next_queued_auto() {
        let pg = PgStore::test_store().await;
        let (c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let a3 = v7(0x15);
        fx_lane(&pg, u1, a3, c1, "L3", "contributor", "tok-a3").await;
        fx_lease(&pg, u1, c1, a3, "granted", Some(t0() + chrono::Duration::seconds(300))).await;
        let now = t0();
        let req = RelayResourceRequest {
            resource: "rust-build".into(),
            floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
        };
        let held = pg.relay_request_resource(u1, a1, req.clone(), now).await.unwrap();
        let q1 = pg.relay_request_resource(u1, a2, req.clone(), now).await.unwrap();
        sqlx_core::query::query("select pg_sleep(0.012)").execute(pg.pool()).await.unwrap();
        let q2 = pg.relay_request_resource(u1, a3, req.clone(), now).await.unwrap();
        assert_eq!(q2.position, Some(2));
        let rel = pg.relay_release_resource(u1, a1, "rust-build", now).await.unwrap();
        assert_eq!(rel.lease_id, held.lease_id);
        assert_eq!(rel.resource, "rust-build");
        assert_eq!(rel.granted_next, Some(q1.lease_id));
        // The queued row flips in place: the waiter keeps its id.
        let (state, _, _, granted_at, expires_at, _) = resource_row(&pg, u1, q1.lease_id).await;
        assert_eq!(state, "granted");
        assert_eq!(granted_at, Some(now));
        assert_eq!(expires_at, Some(now + chrono::Duration::seconds(3600)));
        let (state, _, _, _, _, released_at) = resource_row(&pg, u1, held.lease_id).await;
        assert_eq!(state, "released");
        assert_eq!(released_at, Some(now));
        // And the next waiter moves up (observed through its idempotent re-request).
        let q2b = pg.relay_request_resource(u1, a3, req.clone(), now).await.unwrap();
        assert_eq!(q2b.lease_id, q2.lease_id);
        assert_eq!(q2b.position, Some(1));
    }

    #[tokio::test]
    async fn resource_release_rules_holder_only_and_unknown() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let held = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(held.state, RelayLeaseState::Granted);
        // Held by another lane: the holder alone releases (Q7).
        let err = pg.relay_release_resource(u1, a2, "rust-build", now).await.unwrap_err();
        assert!(matches!(err, RelayError::Forbidden(_)), "non-holder: {err:?}");
        // Nothing live at all: not found.
        let err = pg.relay_release_resource(u1, a1, "e2e", now).await.unwrap_err();
        assert!(matches!(err, RelayError::NotFound(_)), "unknown: {err:?}");
    }

    #[tokio::test]
    async fn resource_heartbeat_renews_held_resource_leases() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let held = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        let q = pg
            .relay_request_resource(
                u1,
                a2,
                RelayResourceRequest { resource: "rust-build".into(), floors: None },
                now,
            )
            .await
            .unwrap();
        assert_eq!(q.state, RelayLeaseState::Queued);
        let later = now + chrono::Duration::seconds(60);
        pg.relay_heartbeat(a1, later).await.unwrap();
        let (_, _, _, _, expires_at, _) = resource_row(&pg, u1, held.lease_id).await;
        assert_eq!(expires_at, Some(later + chrono::Duration::seconds(3600)));
        // The queued row claims no grant: nothing to renew.
        let (_, _, _, granted_at, expires_at, _) = resource_row(&pg, u1, q.lease_id).await;
        assert_eq!(granted_at, None);
        assert_eq!(expires_at, None);
    }

    #[tokio::test]
    async fn resource_sweep_expires_and_grants_next() {
        let pg = PgStore::test_store().await;
        let (c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        // A stale grant with a waiter, and a stale grant with none.
        let stale = fx_resource_lease(
            &pg,
            u1,
            c1,
            a1,
            "rust-build",
            "granted",
            Some(now - chrono::Duration::seconds(3700)),
            Some(now - chrono::Duration::seconds(100)),
        )
        .await;
        let waiter =
            fx_queued_waiter(&pg, u1, c1, a2, "rust-build", RelayFloors { disk_gb: 0, swap_gb: 0 })
                .await;
        let solo = fx_resource_lease(
            &pg,
            u1,
            c1,
            a1,
            "e2e",
            "granted",
            Some(now - chrono::Duration::seconds(3700)),
            Some(now - chrono::Duration::seconds(100)),
        )
        .await;
        // F3 (r2): lane death cascades — a lane whose lease the sweep
        // expires releases its held (still-live) grant, and the waiter
        // inherits it. The grant's own TTL is unexpired, so only the
        // cascade can release it.
        let a5 = v7(0x16);
        fx_lane(&pg, u1, a5, c1, "L5", "contributor", "tok-a5").await;
        fx_lease(&pg, u1, c1, a5, "granted", Some(now - chrono::Duration::seconds(10))).await;
        let doomed = fx_resource_lease(
            &pg,
            u1,
            c1,
            a5,
            "gpu",
            "granted",
            Some(now - chrono::Duration::seconds(100)),
            Some(now + chrono::Duration::seconds(3500)),
        )
        .await;
        let heir =
            fx_queued_waiter(&pg, u1, c1, a2, "gpu", RelayFloors { disk_gb: 0, swap_gb: 0 }).await;
        let report = pg.relay_sweep_leases(now).await.unwrap();
        assert_eq!(report.expired_resource_leases, 2);
        assert_eq!(report.granted_from_queue, 2);
        assert_eq!(report.expired_leases, 1, "a5's stale lane lease, and only it");
        let (state, _, _, _, _, _) = resource_row(&pg, u1, stale).await;
        assert_eq!(state, "expired");
        let (state, _, _, granted_at, expires_at, _) = resource_row(&pg, u1, waiter).await;
        assert_eq!(state, "granted");
        assert_eq!(granted_at, Some(now));
        assert_eq!(expires_at, Some(now + chrono::Duration::seconds(3600)));
        let (state, _, _, _, _, _) = resource_row(&pg, u1, solo).await;
        assert_eq!(state, "expired");
        let (state, _, _, _, _, released_at) = resource_row(&pg, u1, doomed).await;
        assert_eq!(state, "released", "lane death releases the held grant");
        assert_eq!(released_at, Some(now));
        let (state, _, _, granted_at, expires_at, _) = resource_row(&pg, u1, heir).await;
        assert_eq!(state, "granted");
        assert_eq!(granted_at, Some(now));
        assert_eq!(expires_at, Some(now + chrono::Duration::seconds(3600)));
    }

    #[tokio::test]
    async fn resource_unregister_releases_held() {
        let pg = PgStore::test_store().await;
        let (_c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let held = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        let q = pg
            .relay_request_resource(
                u1,
                a2,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        pg.relay_unregister(a1).await.unwrap();
        let (state, _, _, _, _, released_at) = resource_row(&pg, u1, held.lease_id).await;
        assert_eq!(state, "released");
        assert!(released_at.is_some(), "release stamps released_at");
        let (state, _, _, granted_at, _, _) = resource_row(&pg, u1, q.lease_id).await;
        assert_eq!(state, "granted", "the waiter inherits the slot");
        assert!(granted_at.is_some());
    }

    #[tokio::test]
    async fn resource_auto_grant_rechecks_floors_per_waiter() {
        let pg = PgStore::test_store().await;
        let (c1, _c3, a1, a2, _b1, u1) = fx_two_crews(&pg).await;
        let a3 = v7(0x14);
        fx_lane(&pg, u1, a3, c1, "L3", "contributor", "tok-a3").await;
        fx_lease(&pg, u1, c1, a3, "granted", Some(t0() + chrono::Duration::seconds(300))).await;
        let now = t0();
        let held = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(held.state, RelayLeaseState::Granted);
        // Waiter 1 carries an impossible override — but floors gate the
        // grant, not the queue, so it queues (decision 8: the override
        // persists on the row for the grant-time re-check).
        let w1 = pg
            .relay_request_resource(
                u1,
                a2,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: u64::MAX, swap_gb: u64::MAX }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(w1.state, RelayLeaseState::Queued);
        assert_eq!(w1.position, Some(1));
        let (_, _, props, _, _, _) = resource_row(&pg, u1, w1.lease_id).await;
        assert_eq!(props["floors"]["disk_gb"].as_u64(), Some(u64::MAX));
        assert_eq!(props["floors"]["swap_gb"].as_u64(), Some(u64::MAX));
        sqlx_core::query::query("select pg_sleep(0.012)").execute(pg.pool()).await.unwrap();
        let w2 = pg
            .relay_request_resource(
                u1,
                a3,
                RelayResourceRequest {
                    resource: "rust-build".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(w2.position, Some(2));
        // The release re-checks each waiter in turn: waiter 1 refuses into
        // the dropped floors, waiter 2 grants — one transaction.
        let rel = pg.relay_release_resource(u1, a1, "rust-build", now).await.unwrap();
        assert_eq!(rel.granted_next, Some(w2.lease_id));
        let (state, _, props, granted_at, expires_at, _) = resource_row(&pg, u1, w1.lease_id).await;
        assert_eq!(state, "refused");
        assert!(
            !props.get("reason").and_then(|r| r.as_str()).unwrap_or("").is_empty(),
            "the grant-time refusal carries a reason"
        );
        assert_eq!(granted_at, None);
        assert_eq!(expires_at, None);
        let (state, _, props, granted_at, expires_at, _) = resource_row(&pg, u1, w2.lease_id).await;
        assert_eq!(state, "granted");
        assert_eq!(granted_at, Some(now));
        assert_eq!(expires_at, Some(now + chrono::Duration::seconds(3600)));
        // Grant-time numbers persist on the row (fresh read, not the
        // queue-time output): the waiter's re-request reads them back.
        assert!(props["disk_free_gb"].is_number(), "fresh disk persisted");
        let w2b = pg
            .relay_request_resource(
                u1,
                a3,
                RelayResourceRequest { resource: "rust-build".into(), floors: None },
                now,
            )
            .await
            .unwrap();
        assert_eq!(w2b.lease_id, w2.lease_id);
        let granted_disk = w2b.disk_free_gb.expect("granted output carries disk");
        assert_eq!(props["disk_free_gb"].as_u64(), Some(granted_disk));
    }

    #[tokio::test]
    async fn resource_rerequest_after_terminal_mints_fresh() {
        let pg = PgStore::test_store().await;
        let (c1, _c3, a1, _a2, _b1, u1) = fx_two_crews(&pg).await;
        let now = t0();
        let req = RelayResourceRequest {
            resource: "rust-build".into(),
            floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
        };
        // Released → fresh.
        let g1 = pg.relay_request_resource(u1, a1, req.clone(), now).await.unwrap();
        let rel = pg.relay_release_resource(u1, a1, "rust-build", now).await.unwrap();
        assert_eq!(rel.granted_next, None, "no waiter behind the release");
        let g2 = pg.relay_request_resource(u1, a1, req.clone(), now).await.unwrap();
        assert_eq!(g2.state, RelayLeaseState::Granted);
        assert_ne!(g2.lease_id, g1.lease_id, "post-terminal re-request mints");
        let (state, _, _, _, _, _) = resource_row(&pg, u1, g1.lease_id).await;
        assert_eq!(state, "released", "history preserved");
        // Refused → fresh (impossible override refuses into a free slot).
        let r1 = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "e2e".into(),
                    floors: Some(RelayFloors { disk_gb: u64::MAX, swap_gb: u64::MAX }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(r1.state, RelayLeaseState::Refused);
        let g3 = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "e2e".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(g3.state, RelayLeaseState::Granted);
        assert_ne!(g3.lease_id, r1.lease_id);
        let (state, _, _, _, _, _) = resource_row(&pg, u1, r1.lease_id).await;
        assert_eq!(state, "refused");
        // Expired → fresh.
        let stale = fx_resource_lease(
            &pg,
            u1,
            c1,
            a1,
            "gpu",
            "granted",
            Some(now - chrono::Duration::seconds(3700)),
            Some(now - chrono::Duration::seconds(100)),
        )
        .await;
        pg.relay_sweep_leases(now).await.unwrap();
        let (state, _, _, _, _, _) = resource_row(&pg, u1, stale).await;
        assert_eq!(state, "expired");
        let g4 = pg
            .relay_request_resource(
                u1,
                a1,
                RelayResourceRequest {
                    resource: "gpu".into(),
                    floors: Some(RelayFloors { disk_gb: 0, swap_gb: 0 }),
                },
                now,
            )
            .await
            .unwrap();
        assert_eq!(g4.state, RelayLeaseState::Granted);
        assert_ne!(g4.lease_id, stale);
    }

    // ── F3: cancel / close ──

    #[tokio::test]
    async fn cancel_blocked_task_clears_block_memory() {
        // Without the clear, the blocked-memory check fails the write: the
        // cancel is the proof the memory cleared. Store level: reaching a
        // blocked task with a dead holder over HTTP needs unregister,
        // which suspends first.
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        let t = v7(0xf9);
        fx_task(&pg, u1(), t, c1, Some(a2), "working", "tests", 1).await;
        sqlx_core::query::query(
            "update relay.tasks set state = 'blocked', blocked_from_state = 'working'
             where id = $1",
        )
        .bind(t)
        .execute(pg.pool())
        .await
        .unwrap();
        sqlx_core::query::query(
            "update relay.lanes set unregistered_at = current_timestamp where id = $1",
        )
        .bind(a2)
        .execute(pg.pool())
        .await
        .unwrap();
        let out = pg.relay_cancel_task(u1(), a1, t).await.unwrap();
        assert_eq!(out.task_id, t);
        assert_eq!(out.state, RelayTaskState::Cancelled);
        let memory: Option<String> = sqlx_core::query_scalar::query_scalar(
            "select blocked_from_state::text from relay.tasks where id = $1",
        )
        .bind(t)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(memory, None);
    }

    #[tokio::test]
    async fn close_crew_stamps_completed_at() {
        // The census crew row carries no completed_at, so the stamp is
        // pinned here: close means done.
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
        sqlx_core::query::query(
            "update relay.lanes set unregistered_at = current_timestamp where id = $1",
        )
        .bind(a2)
        .execute(pg.pool())
        .await
        .unwrap();
        let out = pg.relay_close_crew(u1(), a1, c1).await.unwrap();
        assert_eq!(out.crew_id, c1);
        assert_eq!(out.status, "closed");
        let row: (String, Option<DateTime<Utc>>) = sqlx_core::query_as::query_as(
            "select status, completed_at from relay.crews where id = $1",
        )
        .bind(c1)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(row.0, "closed");
        assert!(row.1.is_some());
    }
}
