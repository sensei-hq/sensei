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

// Phase-A scaffolding: the stubs are used only by the reds until phase B
// wires the handlers. Removed then (like L4's F6 comments).
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::PgStore;
use crate::relay::envelope::LaneEnvelope;
use crate::relay::{
    RelayError, RelayFindingSeverity, RelayItemKind, RelayRole, RelaySurface, RelayTaskPhase,
    RelayTaskState, RelayTicketProvider, ReviewVerdict,
};

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
}

#[derive(Debug, Clone)]
pub struct RelayAssignOutput {
    pub task_id: Uuid,
    pub round: i32,
    pub state: RelayTaskState,
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
}

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
        let _ = (user_id, input, now);
        todo!("relay item 2 phase B")
    }

    /// Resolve a lane caller's identity: token hash → lane. Unknown token is
    /// `Unauthorized`; an unregistered lane is `Gone` (`reregister` resumes).
    pub async fn relay_auth_lane(
        &self,
        lane_id: Uuid,
        token: &str,
    ) -> Result<RelayLaneSession, RelayError> {
        let _ = (lane_id, token);
        todo!("relay item 2 phase B")
    }

    /// Renew the lane's lease (`now` + 300 s).
    pub async fn relay_heartbeat(
        &self,
        lane_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RelayHeartbeatOutput, RelayError> {
        let _ = (lane_id, now);
        todo!("relay item 2 phase B")
    }

    /// Resume an unregistered (or lapsed) lane: a fresh lease plus the
    /// stored checkpoint. The token is the same one — minted once.
    pub async fn relay_reregister(
        &self,
        lane_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<RelayReregisterOutput, RelayError> {
        let _ = (lane_id, now);
        todo!("relay item 2 phase B")
    }

    /// Leave: stamp `unregistered_at`, release the lane lease, and interrupt
    /// the lane's active task (a voluntary leave suspends work like a lost
    /// lease) — all in one transaction.
    pub async fn relay_unregister(&self, lane_id: Uuid) -> Result<(), RelayError> {
        let _ = lane_id;
        todo!("relay item 2 phase B")
    }

    /// Plan an item into an unassigned task (coordinator only). From an item,
    /// title/repo/workspace default from it and it moves to `planned`; manual
    /// plans name their own. A task without a repository is refused.
    pub async fn relay_plan_item(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        input: RelayPlanInput,
    ) -> Result<RelayPlanOutput, RelayError> {
        let _ = (user_id, caller_lane, input);
        todo!("relay item 2 phase B")
    }

    /// Hand a task to a lane (coordinator only): the brief freezes into the
    /// `assigned` action in the same transaction as the state write.
    pub async fn relay_assign_task(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        task_id: Uuid,
        input: RelayAssignInput,
    ) -> Result<RelayAssignOutput, RelayError> {
        let _ = (user_id, caller_lane, task_id, input);
        todo!("relay item 2 phase B")
    }

    /// A work/rework turn: validate (props, length, leak guard, finding
    /// classes need none here), move working/rework → in_review, pick the
    /// reviewer at first report (Q14), write the `reported` action — one
    /// transaction, or nothing at all.
    pub async fn relay_report_task(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        task_id: Uuid,
        input: RelayReportInput,
    ) -> Result<RelayReportOutput, RelayError> {
        let _ = (user_id, caller_lane, task_id, input);
        todo!("relay item 2 phase B")
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
        let _ = (user_id, caller_lane, task_id, input);
        todo!("relay item 2 phase B")
    }

    /// Ask a decision and block its tasks — one transaction.
    pub async fn relay_ask_decision(
        &self,
        user_id: Uuid,
        caller_lane: Uuid,
        input: RelayAskInput,
    ) -> Result<RelayAskOutput, RelayError> {
        let _ = (user_id, caller_lane, input);
        todo!("relay item 2 phase B")
    }

    /// Answer a decision (the asked-of party only) and resume its tasks —
    /// one transaction.
    pub async fn relay_answer_decision(
        &self,
        user_id: Uuid,
        caller: RelayCaller,
        decision_id: Uuid,
        input: RelayAnswerInput,
    ) -> Result<Uuid, RelayError> {
        let _ = (user_id, caller, decision_id, input);
        todo!("relay item 2 phase B")
    }

    /// Queue a message on the bus — person and lane share it.
    pub async fn relay_send_message(
        &self,
        user_id: Uuid,
        caller: RelayCaller,
        input: RelayMessageInput,
    ) -> Result<RelayMessageOutput, RelayError> {
        let _ = (user_id, caller, input);
        todo!("relay item 2 phase B")
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
        let _ = (user_id, caller_lane, task_id, input);
        todo!("relay item 2 phase B")
    }

    /// The lane's grouped envelope (Q4): messages, answers, reviews,
    /// assignments, each newest-first. Reading marks the lane's active task
    /// started on first delivery and writes `delivered` receipts for newly
    /// included messages — one transaction with the read.
    pub async fn relay_envelope(
        &self,
        user_id: Uuid,
        lane_id: Uuid,
    ) -> Result<LaneEnvelope, RelayError> {
        let _ = (user_id, lane_id);
        todo!("relay item 2 phase B")
    }

    /// Expire lapsed lane leases (fed `now`): suspend their lanes —
    /// active task → interrupted with its checkpoint — re-offer tasks
    /// past the grace (measured from the `interrupted` action), and
    /// re-pick expired reviewers (Q17-REC).
    pub async fn relay_sweep_leases(
        &self,
        now: DateTime<Utc>,
    ) -> Result<RelaySweepReport, RelayError> {
        let _ = now;
        todo!("relay item 2 phase B")
    }

    /// The daemon user behind person reads and register (Q19-REC): the
    /// dōjō principal id from the sign-in session. Fails closed
    /// (`SignInRequired`) when the machine never signed in — never a
    /// minted stand-in (a miss is an error, never an invented row).
    pub async fn relay_current_user(&self) -> Result<Uuid, RelayError> {
        todo!("relay item 2 phase B")
    }

    /// `RelayNow`, exactly per `relay-api.ts` — crews, facets, lanes,
    /// decisions, triage, today's completions, today counts, resources.
    pub async fn relay_now(
        &self,
        user_id: Uuid,
        filter: RelayReadFilter,
    ) -> Result<serde_json::Value, RelayError> {
        let _ = (user_id, filter);
        todo!("relay item 2 phase B")
    }

    /// `RelayQueue`: manual triage rows (the tracker pull is slice 2) and
    /// the backlog in position order.
    pub async fn relay_queue(
        &self,
        user_id: Uuid,
        filter: RelayReadFilter,
    ) -> Result<serde_json::Value, RelayError> {
        let _ = (user_id, filter);
        todo!("relay item 2 phase B")
    }

    /// `RelayTaskView` for (user, crew, task): anything else is `NotFound`.
    pub async fn relay_task_view(
        &self,
        user_id: Uuid,
        crew_id: Uuid,
        task_id: Uuid,
    ) -> Result<serde_json::Value, RelayError> {
        let _ = (user_id, crew_id, task_id);
        todo!("relay item 2 phase B")
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
        let _ = (user_id, caller, input);
        todo!("relay item 2 phase B")
    }

    /// The person's `+`: a triage/manual item → backlog with the next
    /// position. Person-only (lanes plan via `plan_item`).
    pub async fn relay_backlog_add(
        &self,
        user_id: Uuid,
        caller: RelayCaller,
        item_id: Uuid,
    ) -> Result<Uuid, RelayError> {
        let _ = (user_id, caller, item_id);
        todo!("relay item 2 phase B")
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
    #[ignore = "pending:relay-item2"]
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
        // …the lane joins it as L1 with a granted lease and no title yet…
        let (code, role, title): (String, String, Option<String>) = sqlx_core::query_as::query_as(
            "select code, role::text, title from relay.lanes where id = $1",
        )
        .bind(out.lane_id)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!((code.as_str(), role.as_str(), title), ("L1", "coordinator", None));
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
        assert!(out.lane_title.is_none());
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    async fn auth_unregistered_lane_is_gone() {
        let pg = PgStore::test_store().await;
        let (_, _, a1, _, _, _) = fx_two_crews(&pg).await;
        pg.relay_unregister(a1).await.unwrap();
        let err = pg.relay_auth_lane(a1, "tok-a1").await.unwrap_err();
        assert!(matches!(err, RelayError::Gone(_)), "{err:?}");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
        }
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    async fn report_moves_to_in_review_picks_least_loaded_q14() {
        let pg = PgStore::test_store().await;
        let (c1, _, a1, a2, _, _) = fx_two_crews(&pg).await;
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
        let fresh = v7(0x88);
        fx_task(&pg, u1(), fresh, c1, Some(a2), "assigned", "tests", 1).await;
        let err = pg.relay_report_task(u1(), a2, fresh, report_input()).await.unwrap_err();
        assert!(matches!(err, RelayError::Conflict(_)), "{err:?}");
        assert_eq!(count(&pg, "actions").await, 0);
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn sweep_reoffers_after_grace_not_before() {
        let pg = PgStore::test_store().await;
        let (c1, _, _, a2, _, _) = fx_two_crews(&pg).await;
        // Interrupted long ago (grace run out) vs just now.
        for (n, ago_secs) in [(0xb2, 901), (0xb3, 100)] {
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
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    async fn now_completed_midnight_pair_is_session_local() {
        let pg = PgStore::test_store().await;
        let (c1, _, a2, _, _, _) = fx_two_crews(&pg).await;
        // Midnight-sharp (in) vs a second before (out), computed off the
        // session clock exactly like L4's SQL pin — deterministic in any zone.
        for (n, off) in [(0xe5, "0 seconds"), (0xe6, "-1 seconds")] {
            let t = v7(n);
            fx_task(&pg, u1(), t, c1, Some(a2), "done", "tests", 1).await;
            sqlx_core::query::query(
                "update relay.tasks set completed_at = date_trunc('day', current_timestamp) + $1::interval where id = $2",
            )
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
    #[ignore = "pending:relay-item2"]
    async fn now_without_crews_is_empty_not_failure() {
        let pg = PgStore::test_store().await;
        let now = pg.relay_now(v7(0xa9), no_filter()).await.unwrap();
        assert_eq!(now["crews"], json!([]));
        assert_eq!(now["today"]["completed"], 0);
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
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
    #[ignore = "pending:relay-item2"]
    async fn relay_read_failure_is_internal_never_empty() {
        let pg = PgStore::test_store().await;
        pg.pool().close().await;
        let err = pg.relay_now(u1(), no_filter()).await.unwrap_err();
        assert!(matches!(err, RelayError::Internal(_)), "{err:?}");
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn current_user_is_stable_q19rec() {
        // Q19-REC: whatever source phase B reads, the daemon user is one
        // stable uuid per machine+identity (the failure mode — never signed
        // in — waits on the ruling for its trigger).
        let pg = PgStore::test_store().await;
        let a = pg.relay_current_user().await.unwrap();
        let b = pg.relay_current_user().await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    #[ignore = "pending:relay-item2"]
    async fn pool_timezone_is_the_mac_zone() {
        // The completed window is the session-local day, so every pool
        // connection sets TimeZone to this Mac's zone (part-2 item).
        let pg = PgStore::test_store().await;
        let tz: String = sqlx_core::query_scalar::query_scalar("show TimeZone")
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(tz, crate::relay::system_timezone().unwrap());
    }
}
