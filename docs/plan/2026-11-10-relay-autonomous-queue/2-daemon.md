# Phase 2 — Daemon (senseid)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Expand each task below to red → run → green → run → commit steps when the phase starts; the code here is the contract, not every line.

**Goal:** The broker coordinates: lanes pull work and reviews atomically, reads never mutate, reviews float, reports are gated on dependencies, the person mints sprint runs whose Task 0 is a peer-reviewed plan that unpacks into the queue, the sweeper heals every abandoned claim and completes runs, and the deterministic gate merges approved work. F01–F12 close with pins.

**Architecture:** Pure rules in `crates/senseid/src/relay/mod.rs` (vocabulary, state machine, props allowlists, guidance v2) and a new `crates/senseid/src/relay/plan.rs` (plan JSON parse + DAG validation, DB-free). Store ops in `crates/senseid/src/db/pg_store/relay.rs` (one transaction per op, `FOR UPDATE SKIP LOCKED` for claims). Handlers in `crates/senseid/src/api/handlers/relay.rs`, routes in `crates/senseid/src/api/routes.rs`, events in `crates/senseid/src/api/events.rs`. Sweeper in `crates/senseid/src/tasks/relay_lease_sweep.rs` (same 60 s `relay_lease` schedule; the broker-task runner rides the same tick). Gate subprocesses behind a trait in a new `crates/senseid/src/relay/gate.rs` (like `relay_drivers/trait_def.rs`), resolved with `sensei_bootstrap::util::command_for` (launchd PATH).

**Branch:** `lane/relay-autonomous-queue` off `develop`; reds committed with `#[ignore = "pending: relay-queue"]` per the coord protocol, un-ignored when green; merged to develop only fully green.

**Tests to write first (the design §12 list, mapped to files):**

| Test | File |
|---|---|
| `claim_next_prioritizes_floating_reviews_over_ready_tasks` | `pg_store/relay.rs` tests |
| `claim_next_never_offers_an_author_its_own_review` | `pg_store/relay.rs` |
| `claim_next_offers_unblocked_before_claim_early` | `pg_store/relay.rs` |
| `claim_next_skips_locked_rows_under_two_concurrent_claims` | `pg_store/relay.rs` (two tasks racing on one pool row) |
| `claim_early_task_checkpoints_but_report_is_409_until_deps_done` | `handlers/relay.rs` |
| `get_tasks_and_wait_leave_task_state_unchanged` (F01) | `handlers/relay.rs` |
| `heartbeat_preserves_a_limit_pause_with_unknown_resume` (F03) | `pg_store/relay.rs` |
| `sweeper_ends_a_pause_at_resumes_at` | `pg_store/relay.rs` |
| `limit_resume_clears_the_pause` | `handlers/relay.rs` |
| `report_md_with_a_secret_is_422_kinds_only` (F04) | `handlers/relay.rs` |
| `review_finding_text_with_a_secret_is_422` (F04) | `handlers/relay.rs` |
| `reregister_resumes_an_interrupted_task_to_working` (F05) | `pg_store/relay.rs` |
| `wait_registers_a_waiter_row_and_clears_it_on_answer` (F06) | `handlers/relay.rs` |
| `sweeper_evicts_waiters_whose_lease_lapsed` (F06/F10) | `pg_store/relay.rs` |
| `sweeper_nudges_everyone_once_per_window_for_an_old_floating_review` | `pg_store/relay.rs` |
| `checkpoint_merges_into_the_task_and_moves_nothing` (F07) | `handlers/relay.rs` |
| `lane_message_to_a_lane_in_another_crew_is_404` (F08) | `pg_store/relay.rs` |
| `envelope_messages_are_crew_scoped` (F08) | `pg_store/relay.rs` |
| `telemetry_family_version_model_are_accepted` (F09) | `relay/mod.rs` tests |
| `grant_next_waiter_skips_a_dead_lane` (F10) | `pg_store/relay.rs` |
| `wait_query_rejects_unknown_params_and_accepts_timeout_seconds` (F11, #404) | `handlers/relay.rs` |
| `report_and_review_never_touch_the_lane_lease` (F12) | `pg_store/relay.rs` |
| `sweeper_clears_a_review_claim_whose_lane_lapsed_60s_ago` | `pg_store/relay.rs` |
| `sweeper_reoffers_an_interrupted_task_to_the_pool` | `pg_store/relay.rs` |
| `create_run_mints_task0_in_the_pool_and_plans_its_items` | `handlers/relay.rs` |
| `approving_task0_unpacks_the_plan_and_activates_the_run` | `pg_store/relay.rs` |
| `rejecting_task0_sends_it_to_rework_with_the_critique` | `pg_store/relay.rs` |
| `plan_parse_rejects_cycles_unknown_deps_duplicate_keys_bad_phase` | `relay/plan.rs` tests |
| `run_completes_when_every_task_is_done` | `pg_store/relay.rs` |
| `register_defaults_to_contributor_and_joins_the_single_active_crew` (#402) | `handlers/relay.rs` |
| `register_with_zero_or_two_active_crews_is_409_with_a_code` | `handlers/relay.rs` |
| `approval_inserts_a_broker_gate_task_and_the_gate_marks_done_on_pass` | `pg_store/relay.rs` + `relay/gate.rs` (fake runner) |
| `gate_failure_moves_the_task_to_rework_with_a_build_failure_finding` | `relay/gate.rs` |
| `release_returns_a_task_to_the_pool_and_a_review_to_the_float` | `handlers/relay.rs` |
| `sweep_events_name_released_reviews_resumed_lanes_evicted_waiters_completed_runs` | `tasks/relay_lease_sweep.rs` |

---

## Task 2.1: DDL cutover (the destructive half) + rename blast radius

**Files:** `database/ddl/table/relay/tasks.ddl`, `database/ddl/view/relay/now.ddl`, `database/tests/relay/relay_tasks.sql`, `relay_now.sql`, `relay_task_queue.sql`, `relay_tasks_cutover.sql.pending → .sql`, `database/tests/fixtures/relay_apply.ddl` (unchanged list), every `reviewer_lane_id` / `reviewer_lane` reference (`crates/senseid/src/db/pg_store/relay.rs` ×39, `crates/senseid/src/api/handlers/relay.rs` ×8, `crates/mcp/src/lib.rs` ×2, `app/e2e/tests/relay-timeline.spec.ts` ×1).

DDL: drop `reviewer_lane_id`, `tasks_reviewer_fk`, `tasks_no_self_review`; rename `tasks_no_self_review_claim` → keep as the one self-review check; `state … default 'ready'`; `assigned_at timestamptz` (nullable, no default); add `constraint tasks_ready_lane_coherence check ((state = 'ready') = (lane_id is null))`; replace `tasks_one_active_per_lane` with:
```sql
create unique index if not exists tasks_one_active_worker_per_lane
  on relay.tasks (user_id, lane_id) where lane_id is not null
  and state in ('assigned', 'working', 'rework', 'blocked');
```
`now.ddl` lane join: `and t.state in ('assigned', 'working', 'rework', 'blocked')`. The SQL fixtures that insert `assigned` with a null lane (relay_tasks.sql T5/T6 "unassigned working pair", relay_now.sql T6) become `ready`/null or `working`/laned as each pin intends.

Rust: `RelayAssignInput.reviewer_lane` is removed (assign no longer pre-picks); `relay_report_task` stops picking; `relay_pick_reviewer`, `RelayPickQuery`, `pick_reviewer` deleted; `relay_submit_review` authorises on `review_claimed_by`; `relay_handover_task` clears `review_claimed_by/_at`; `relay_sweep_leases` drops the re-pick block (replaced in 2.7); `relay_task_view` serves `review_claimed_by` as `reviewer` (same wire key the app reads, lane code resolved); `relay_plan_item` inserts `ready` with no lane; `relay_assign_task` accepts a `ready` task (`ready → assigned`, lane set) and keeps `approved → working` reassign; `relay_cancel_task` cancels `ready` (lane null stays null — the coherence check allows `cancelled` with null lane); `relay_envelope` assignment query reads `assigned_at` as `Option<DateTime<Utc>>`.

Converge: `make test-template`; `dropdb sensei_test_<lane>`; `./run.sh relay`. Live DB waits for 2.10.

Commit: `feat(relay): queue cutover — review claims replace the picked reviewer, in_review frees the lane, ready is the pool default`.

**Steps (expanded 2026-11-12, on `lane/relay-autonomous-queue` in `lanes/relay-queue`):**

Two deviations from the task text, decided at expansion: (1) `claim_review` (store op `relay_claim_review`, route `POST /api/relay/tasks/{id}/claim_review`, action kind `review_claimed`) is pulled in from 2.4/2.9 — once `relay_submit_review` authorises on `review_claimed_by`, nothing else can set it, so without the claim the checkpoint would ship a daemon in which no review can ever be submitted and the report→review→merge pins would sit ignored. (2) The coherence check is `tasks_lane_coherence`: a `ready` task has no lane; a laneless task is `ready`, `cancelled`, or broker-executed (`(state = 'ready') = (lane_id is null)` would refuse a cancelled pool task and every broker task).

- [x] **S1 (SQL red):** `git mv relay_tasks_cutover.sql.pending relay_tasks_cutover.sql`; `relay_tasks.sql` — T1 drops `reviewer_lane_id`, T5/T6 become the ready pool pair, the in_review-beside-working probe flips to accepted, the `reviewer_lane_id` self-review and FK probes become `review_claimed_by` ones, the defaults pin reads `ready`/no lane/`assigned_at` null and a laned no-state insert is refused, a laneless `cancelled` is accepted; `relay_now.sql` — T6 `ready`, L5/L6 lane rows read `task` null; `relay_task_history.sql` — T2 `ready`; `relay_task_queue.sql` — Q7 back beside Q4 on L1. `rt.sh relay/*.sql` → cutover, tasks, now, task_queue FAIL.
- [x] **S2 (SQL green):** `tasks.ddl` (drop `reviewer_lane_id` + its FK + `tasks_no_self_review`; `state default 'ready'`; `assigned_at` nullable no default; `tasks_lane_coherence`; `tasks_one_active_worker_per_lane`), `now.ddl` lane join without `in_review`. `rt.sh relay/*.sql` → 14 ok.
- [x] **S3 (Rust red, compiles against the old code, fails at runtime):** store — `plan_item` lands `ready`; `assign` accepts a `ready` task and stamps `assigned_at`; `review` needs the claim (claimant ok, author and unclaimed lane 403); envelope to-perform reads the claim; sweep leaves an `in_review` task alone when the author's lease lapses; handler — report → `claim_review` → review → merge flow; `claim_review` refuses the author (422), a second claim (409 by the index), a lane with no live lease (409); handover to the claimant 409. `cargo test -p senseid relay -- --test-threads=1` → those red. Commit `test(relay): queue cutover pins (red)`.
- [x] **S4 (Rust green):** `RelayTaskState::Ready`, `RelayActionKind::ReviewClaimed` (+ empty props allowlist); delete `pick_reviewer`/`relay_pick_reviewer`/`RelayPickQuery` and their tests; `RelayAssignInput.reviewer_lane` and `RelayReportOutput.reviewer_lane_id` removed (handler body, response, mcp args, e2e body follow); `report` floats (clears the claim); `submit_review` authorises on the claim and clears it; `handover` reads/clears the claim; `sweep` drops the re-pick block, `repicked_reviewers`, `ReviewerRepicked`, and no longer interrupts `in_review`; `task_view` serves `reviewer {id, code}`; envelope `assigned_at` optional; `relay_claim_review` + route + `emit_task("claim_review")`. `cargo test -p senseid relay`, `cargo test -p sensei-mcp`, `cargo clippy -p senseid -p sensei-mcp -- -D warnings`, `cargo fmt` → green/clean.
- [ ] **S5 (converge + commit):** `make test-template`; `dropdb sensei_test_relay_autonomous_queue`; `./run.sh relay`; `make test-fast`; commit (message above); `/sensei:checkpoint`.

## Task 2.2: Vocabulary, state machine, props, errors (`relay/mod.rs`)

- `RelayTaskState::Ready`; `RelayTaskPhase::Planning`; `RelayActionKind::{Claimed, ReviewClaimed, Released, Checkpointed}`; new `RelayRunState {Planning, Active, Completed, Cancelled}`; `RelayExecutor {Agent, Broker}`.
- `TaskEvent::{Claimed, Released, Resumed, Unpacked}`; `transition`: `(Ready, Claimed) → Working`; `(Ready | Assigned, Assigned-by-coordinator)` stays `Assigned` (store-level, no event); `(Assigned | Working, Released) → Ready`; `(Interrupted, Reoffered) → Ready` (was `Assigned`); `(Interrupted, Resumed) → Working`; `(Approved, Unpacked) → Done` (Task 0); `(Ready, Cancelled) → Cancelled` already covered by the catch-all.
- `validate_props`: `Claimed` allows `["claim_type", "deps_status", "unmet_deps"]` (`claim_type` ∈ `task|review` required); `ReviewClaimed` allows `[]`; `Released` allows `["kind", "reason"]` (`kind` ∈ `task|review` required); `Checkpointed` allows `["current_step", "notes"]`; `Reported`/`Reviewed` allowlists gain `"family", "version", "model"` (F09); `Reviewed` findings gain optional `"detail"` (gate output tail).
- `RelayError::DepsUnmet(String)` → 409, code `relay_upstream_deps_unmet`; `RelayError::NoActiveCrew` → 409 `relay_no_active_crew`; `RelayError::CrewAmbiguous` → 409 `relay_crew_ambiguous`; `RelayError::NothingToClaim` is NOT an error (claim_next answers `{"claim_type": "none"}` with 200).
- Constants: `REVIEW_CLAIM_GRACE_SECS: u64 = 60`; `NUDGE_AFTER_SECS: u64 = 900`; `REOFFER_GRACE_SECS` stays 900 (the design's 120 s is for the *lane's* re-register window; keep one grace, document).
- Guidance v2: `RELAY_GUIDANCE_VERSION = "2"`; `RELAY_GUIDANCE_GENERIC` rewritten around the loop:
  1. beat every 60 s (`heartbeat`; it never ends a quota pause);
  2. `POST /api/relay/tasks/claim_next` → a review (`claim_type: review`), a task (`task`, with `deps_status: satisfied | early_claim_gated` and `unmet_deps`), or `none` → then `GET /api/relay/tasks/wait` (`timeout_ms` or `timeout_seconds`, ≤ 50 s) and claim again when it answers;
  3. work in your worktree; `POST /tasks/{id}/checkpoint` between turns; `POST /tasks/{id}/steps` for the checklist;
  4. `POST /tasks/{id}/report` — 409 `relay_upstream_deps_unmet` until every dep is done; a full report floats the task on the review queue and frees you;
  5. reviewing: inspect the branch, run the acceptance, `POST /tasks/{id}/review` with `approved` or `changes_requested` + findings; approval hands the task to the broker's gate; `POST /tasks/{id}/release` gives a claim back;
  6. quota: `limit_pause` when the provider says so; `limit_resume` when you are back early; the pause ends by itself at `resumes_at`;
  7. caps, resume (`reregister` resumes interrupted work to `working`), silence — as v1.
  Family notes updated to name `claim_next` instead of "the coordinator assigns".

Pins: transition table rows above; props allowlists; guidance mentions every new route (a test that greps the constant for each path).

**Steps (expanded 2026-11-12):** Guidance v2 moves to task 2.9 — it would describe routes (`claim_next`, `checkpoint`, `release`, `limit_resume`) that do not exist until then, and a register response must not name a route the daemon 404s. 2.2 stays the DB-free vocabulary: `Planning`, `Claimed`/`Released`/`Checkpointed` (`ReviewClaimed` landed in 2.1), `RelayRunState`, `RelayExecutor`, the four `TaskEvent`s and their rows, the three props allowlists + F09 keys + the nudge keys on `Message`, the three errors, the two constants. The `Message` nudge keys come here too (2.7 reads them). `(Ready, Claimed) → Working` and `(Interrupted, Reoffered) → Ready` (the latter shipped in 2.1).
- [x] **S1 (red):** `mod.rs` pins — `queue_vocabulary_serializes_as_db_labels`, `transition_queue_events_claim_release_resume_unpack`, `validate_props_queue_kinds`, `validate_props_telemetry_identity_f09`, `validate_props_message_nudge_keys`, the error and constant pins. `cargo test -p senseid relay::` → fails to compile (new symbols).
- [x] **S2 (green):** the enums, events, transition rows, props arms, errors (+ the handler's `err()` match), constants. `cargo test -p senseid relay`, clippy, fmt → green. Commit the pair.

## Task 2.3: Plan parse + DAG validation (`relay/plan.rs`, DB-free)

```rust
pub struct PlanTask { pub key: String, pub title: String, pub phase: RelayTaskPhase,
    pub owned_paths: Vec<String>, pub acceptance: Vec<String>, pub deps: Vec<String>,
    pub resources: Vec<String>, pub repo_key: Option<String> }
pub struct PlanSpec { pub tasks: Vec<PlanTask> }
pub enum PlanError { NoJsonBlock, Json(String), DuplicateKey(String), UnknownDep { task: String, dep: String },
    Cycle(Vec<String>), EmptyTitle(String), NoAcceptance(String), Empty }
/// The fenced ```json block in `report_md` (the last one wins), else the whole body when it parses.
pub fn parse_plan(report_md: &str) -> Result<PlanSpec, PlanError>;
/// Keys unique, deps resolve, no cycles (Kahn), every task titled with ≥1 acceptance line.
pub fn validate_plan(spec: &PlanSpec) -> Result<(), PlanError>;
/// Topological order for insertion (deps before dependants), stable by input order.
pub fn topo_order(spec: &PlanSpec) -> Vec<usize>;
```
Pins: the §7.2 example parses to two tasks with `t2.deps == ["t1"]`; a self-dep is a cycle; `deps: ["t9"]` is `UnknownDep`; two `t1` keys is `DuplicateKey`; no acceptance is `NoAcceptance`; a prose-only report is `NoJsonBlock`.

**Executed 2026-11-12:** red `5877cf1e8` (pins + signature stubs) → green (bodies). Two additions beyond the task text: `PlanError::BadPhase` (a plan never nests a `planning` task) and `#[serde(deny_unknown_fields)]` on both structs (a planner's typo is an error naming the key, not a dropped field); `PlanError::message()` is the 409 line.

## Task 2.4: Store — claims, release, checkpoint, limit resume

Signatures (all one transaction, `now` fed):
```rust
pub enum RelayClaim { Review(RelayClaimedReview), Task(RelayClaimedTask), None }
pub struct RelayClaimedTask { pub task_id: Uuid, pub run_id: Option<Uuid>, pub title: String, pub phase: RelayTaskPhase,
    pub round: i32, pub repo_key: String, pub owned_paths: Vec<String>, pub acceptance: Vec<String>,
    pub deps_status: DepsStatus /* Satisfied | EarlyClaimGated */, pub unmet_deps: Vec<Uuid>, pub checkpoint: serde_json::Value }
pub struct RelayClaimedReview { pub task_id: Uuid, pub title: String, pub phase: RelayTaskPhase, pub round: i32,
    pub author_lane_code: String, pub report_md: Option<String>, pub summary: String, pub branch: Option<String>, pub files: serde_json::Value }
pub async fn relay_claim_next(&self, user_id, lane_id, now) -> Result<RelayClaim, RelayError>;
pub async fn relay_claim_task(&self, user_id, lane_id, task_id, now) -> Result<RelayClaimedTask, RelayError>;
pub async fn relay_claim_review(&self, user_id, lane_id, task_id, now) -> Result<RelayClaimedReview, RelayError>;
pub async fn relay_release_task(&self, user_id, lane_id, task_id, reason: Option<String>, now) -> Result<RelayReleaseOutput, RelayError>;
pub async fn relay_checkpoint_task(&self, user_id, lane_id, task_id, input: RelayCheckpointInput) -> Result<DateTime<Utc>, RelayError>;
pub async fn relay_limit_resume(&self, lane_id, now) -> Result<(), RelayError>;
```
Guards shared by the claims (`relay_require_claimable_lane`): live granted lease (`expires_at > now`), `limit_paused_at is null`, crew open; a task claim additionally requires no row in `tasks_one_active_worker_per_lane`'s set for the lane (409 `lane is already busy`); a review claim requires no active review claim (409) and `lane_id <> author`.

The priority-1 select:
```sql
select id from relay.tasks
 where user_id = $1 and crew_id = $2 and executor = 'agent'
   and state = 'in_review' and review_claimed_by is null and lane_id <> $3
 order by state_changed_at, id
 for update skip locked limit 1
```
then `update relay.tasks set review_claimed_by = $3, review_claimed_at = $4 where user_id = $1 and id = $5` and a `review_claimed` action (lane = claimant, reply_to = latest `reported`).

The priority-2/3 select (one query, ordered so unblocked rows come first):
```sql
select t.id, not exists (select 1 from relay.tasks d
          where d.user_id = t.user_id and d.id = any (t.deps) and d.state <> 'done') as unblocked
  from relay.tasks t
 where t.user_id = $1 and t.crew_id = $2 and t.executor = 'agent' and t.state = 'ready'
 order by unblocked desc, t.state_changed_at, t.id
 for update of t skip locked limit 1
```
then `update … set lane_id = $3, state = 'working', assigned_at = $4, state_changed_at = $4` and a `claimed` action with `{claim_type: "task", deps_status, unmet_deps}`.

`relay_release_task`: holder of `assigned|working` → `ready`, `lane_id = null`, `state_changed_at = now`, `released {kind: task, reason}`; the `review_claimed_by` holder of an `in_review` task → claim cleared, `released {kind: review}`; anyone else 403.

`relay_checkpoint_task`: holder only; state `working|rework`; `checkpoint = checkpoint || $input` (jsonb merge of `current_step`, `notes`, plus any caller keys); each `steps[]` entry goes through the existing `relay_post_step` path; action `checkpointed`.

**Steps (expanded 2026-11-12):** `relay_claim_review` shipped in 2.1; this task adds the rest and the shared guard (`relay_require_claimable_lane`: lane exists → not unregistered → not limit-paused → live lease → crew open), which `claim_review` is refactored onto. `DepsStatus` and `ReleaseKind` live in `relay/mod.rs` (wire vocabulary). A task claim stamps `assigned_at`/`state_changed_at` at the fed `now` (the sweeper's precedent), not `current_timestamp`. `claim_next` still offers a floating review to a lane that holds a worker task (reviewing beside working is the point of floating reviews); only the pool is skipped for a busy lane.
- [x] **S1 (red):** store types (`RelayClaimedTask`, `RelayClaim`, `RelayReleaseOutput`, `RelayCheckpointInput`) + signature stubs + nine pins: `claim_next_prioritizes_floating_reviews_over_ready_tasks`, `claim_next_never_offers_an_author_its_own_review`, `claim_next_offers_unblocked_before_claim_early`, `claim_next_skips_locked_rows_under_two_concurrent_claims`, `claim_next_skips_the_pool_for_a_busy_lane_but_still_offers_a_review`, `claim_next_refuses_a_paused_dead_or_unregistered_lane`, `claim_task_takes_a_named_ready_task_and_refuses_busy_and_non_ready`, `release_returns_a_task_to_the_pool_and_a_review_to_the_float`, `checkpoint_merges_into_the_task_and_moves_nothing`, `limit_resume_clears_the_pause`. `cargo test -p senseid relay -- --test-threads=1` → those red on the stubs.
- [x] **S2 (green):** the guard, the claim write path shared by `claim_next`/`claim_task` (and the review path shared with `claim_review`), release, checkpoint, limit_resume. Relay suite, clippy, fmt → green. Commit the pair. Two decisions at green: a released task's `assigned_at` is nulled with its lane (the pool has no assignment); `relay_limit_resume(lane_id)` takes no clock and returns whether a pause actually ended, so the handler can emit `LaneChange::Resumed` only when one did.

## Task 2.5: Store — runs, Task 0, unpack, completion

```rust
pub struct RelayCreateRunInput { pub crew_id: Option<Uuid>, pub title: String, pub goal: String, pub item_ids: Vec<Uuid>,
    pub repo_key: Option<String>, pub workspace_id: Option<Uuid>, pub config: serde_json::Value }
pub struct RelayCreateRunOutput { pub run_id: Uuid, pub planning_task_id: Uuid, pub crew_id: Uuid }
pub async fn relay_create_run(&self, user_id, input, now) -> Result<RelayCreateRunOutput, RelayError>;
pub(crate) async fn relay_unpack_plan(tx, user_id, run_id, task0_id, spec: &PlanSpec, now) -> Result<Vec<Uuid>, RelayError>;
pub(crate) async fn relay_complete_runs(tx, now) -> Result<Vec<(Uuid, Uuid)>, RelayError>; // (run_id, crew_id)
```
`relay_create_run`: crew = `input.crew_id` or the single active crew (`NoActiveCrew` / `CrewAmbiguous`); `title`/`goal` through `check_summary`; every item must be `triage|backlog` for this user (404/409 otherwise) and gets `state = 'planned'`; `run_items` positions follow `item_ids` order; `repo_key` = input, else the crew's, else the first item's (`Validation("a run needs a repository")` when none); Task 0 = `relay.tasks` with `phase = 'planning'`, `state = 'ready'`, `run_id`, `title = format!("Plan: {}", title)`, `acceptance = ["report_md carries a ```json plan block: tasks[{key,title,phase,owned_paths,acceptance,deps}]", "no cycles; every dep names a key; owned_paths disjoint across parallel tasks", "every task sized to 1–3 turns"]`, `deps = {}`; `config` stored as given (the handler validates keys `base_checkout`, `base_branch`, `verify_cmd`, `auto_merge`).

`relay_submit_review` on `approved` of a `planning` task: `parse_plan(latest reported.report_md)` → `validate_plan` → `relay_unpack_plan` (inserts one `ready` task per spec entry with `run_id`, `repo_key` = entry's or the run's, `deps` mapped key → uuid in `topo_order`; Task 0 → `done`, `completed_at = now`; run → `active`, `started_at = now`); a parse/validation failure is a 409 `relay_plan_invalid` on the review call (the reviewer approved an unparseable plan — the review is refused, the task stays `in_review`, the reviewer gets the error and files `changes_requested` instead).

`relay_complete_runs` (sweeper): `update relay.runs r set state = 'completed', completed_at = $1 where state = 'active' and not exists (select 1 from relay.tasks t where t.user_id = r.user_id and t.run_id = r.id and t.state not in ('done', 'cancelled')) and exists (select 1 from relay.tasks t where t.user_id = r.user_id and t.run_id = r.id) returning id, crew_id`.

**Steps (expanded 2026-11-12):** `RelayError::PlanInvalid(String)` → 409 `relay_plan_invalid` is added here (the task text named the code without the variant). The active-crew rule reads `relay.crews.status <> 'closed'` (closing stamps `completed_at`). A run with no items is allowed when a repository comes from the input or the crew (the goal alone can be planned). Task 0's acceptance is `RELAY_TASK0_ACCEPTANCE`. `relay_submit_review` parses and validates the plan BEFORE any write, so a refused approval leaves the claim and the review untouched.
- [x] **S1 (red):** types (`RelayCreateRunInput`/`Output`, `RELAY_TASK0_ACCEPTANCE`), signature stubs for `relay_create_run`, `relay_unpack_plan`, `relay_complete_runs`, the error pins, and six pins: `create_run_mints_task0_in_the_pool_and_plans_its_items`, `create_run_refuses_bad_crews_items_and_a_run_without_a_repo`, `approving_task0_unpacks_the_plan_and_activates_the_run`, `rejecting_task0_sends_it_to_rework_with_the_critique`, `approving_task0_with_an_invalid_plan_is_refused_and_leaves_the_review_open`, `run_completes_when_every_task_is_done`.
- [x] **S2 (green):** the three bodies and the Task 0 hook in `relay_submit_review` (approved + planning → parse/validate first, then approve + unpack in the one transaction). Relay suite, clippy, fmt → green. Commit the pair. Also landed here (2.7 item 10, pulled forward because clippy refuses an unused store op): `relay_sweep_leases` calls `relay_complete_runs` before its commit, `RelaySweepReport.completed_runs`, and `sweep_events` emits `RunChange::Completed` per run (crew-keyed until 2.9 extends `relay_run` with the run id).

## Task 2.6: Store — report gate, envelope read-only, heartbeat, reregister, messages

- `relay_report_task` (non-partial): before the state write, `select array_agg(d.id) from relay.tasks d where d.user_id = $1 and d.id = any ($deps) and d.state <> 'done'` → non-empty → `Err(RelayError::DepsUnmet(format!("upstream {} not done: {}", …)))`; `guard_markdown(report_md)` when present (F04); on success `review_claimed_by = null, review_claimed_at = null` (floats).
- `relay_submit_review`: `guard_markdown` over every `finding.text` (F04); `changes_requested` → `rework` + claim cleared; `approved` (non-planning, `executor = agent`) → `approved` + insert the gate broker task (2.8) when `run.config.auto_merge != false` and the latest report named a `branch`; `approved` with no branch/commits → `done` at once (nothing to merge, the mockup/analysis case).
- `relay_envelope`: delete the `if state == "assigned"` block (F01); messages query gains `and crew_id = $crew` (F08); the to-perform reviews read `review_claimed_by = $lane`; assignments read `state in ('assigned','working','rework')` (unchanged) with `assigned_at` optional.
- `relay_send_message` to `Lane{lane_id}`: the addressed lane must share the caller's crew (person callers: the task's crew when `task_id` is given) else `NotFound("lane not found")` (F08).
- `relay_heartbeat`: delete the pause-clearing update; `RelayHeartbeatOutput` loses `pause_ended` (the handler's `LimitPauseEnded` emit moves to the sweeper).
- `relay_reregister`: after the lease insert: `update relay.tasks set state = 'working', state_changed_at = $now where user_id = $1 and lane_id = $2 and state = 'interrupted' returning id` → `RelayReregisterOutput.resumed_tasks: Vec<Uuid>`; a `started` action per task (F05).
- `relay_report_limit_pause` unchanged; new `relay_limit_resume`.
- `relay_register`: `role` defaults to `Contributor`; `crew_id` optional for both roles → single active crew (`select id from relay.crews where user_id = $1 and completed_at is null`; 0 → `NoActiveCrew` unless role is coordinator (mint, as today); >1 → `CrewAmbiguous`); `suggested_name` stored; `lane_title` = `suggested_name` when given, else `<code> · <family>`.
- `grant_next_waiter`: the waiter's lane must hold a granted, unexpired lane lease (F10); dead waiters are skipped (their queued row is `refused` with reason `lane_dead`).

**Steps (expanded 2026-11-12):** Three decisions beyond the task text. (1) With F01 gone nothing started an `assigned` task (there is no start tool), so the holder's first write starts it: `relay_post_step`, `relay_checkpoint_task` and `relay_report_task` accept `assigned` and apply `Started` (one `started` row) before their own write. (2) Unregister leaves an `in_review` task alone: the report floats on the review queue and is no longer the lane's to lose (the #404 wipe). (3) Approval of a task whose latest report names no branch ends it `done` at once (nothing to merge); with a branch it stays `approved` — the gate task is 2.8's. `RelayHeartbeatOutput.pause_ended` goes; the handler's `LaneChange::Resumed(LimitPauseEnded)` emit moves to the sweeper (2.7). `register_mode` gains `JoinActive` (contributor, no crew_id; the variants are `Mint | Join | JoinActive` — clippy refused a shared `Crew` suffix).
- [x] **S1 (red):** `RegisterMode::JoinActive`, `RelayRegisterInput.suggested_name`, `RelayReregisterOutput.resumed_tasks` (vocabulary + fields); store pins `report_is_gated_until_upstream_deps_are_done`, `report_md_with_a_secret_is_refused_kinds_only`, `review_finding_text_with_a_secret_is_refused`, `approving_a_task_with_nothing_to_merge_ends_it_done`, `envelope_read_leaves_an_assigned_task_assigned_and_the_first_write_starts_it`, `envelope_messages_are_crew_scoped`, `lane_message_to_a_lane_in_another_crew_is_404`, `heartbeat_preserves_a_limit_pause_with_unknown_resume`, `reregister_resumes_an_interrupted_task_to_working`, `unregister_leaves_a_floating_review_alone`, `grant_next_waiter_skips_a_dead_lane`, `register_defaults_to_contributor_and_joins_the_single_active_crew`; handler pins `heartbeat_never_ends_a_pause`, `heartbeat_on_a_paused_lane_emits_nothing`; `working_task` starts by a step. Red on the current store. (`d0db800bd`: 15 red.)
- [x] **S2 (green):** the store changes above + the handler drops the heartbeat emit. Relay suite, clippy, fmt → green. Commit the pair. Executed: `register_mode` is now infallible (every pair has a mode); the single-active-crew lookup is one helper shared by register and run-create (`status <> 'closed'`, the house "open" test, not `completed_at is null`); `relay_start_if_assigned` is the one first-write hook (step, checkpoint, report); the deps gate names the unmet task ids; the sweeper step for `LimitPauseEnded` is 2.7's.

## Task 2.7: Sweeper (`relay_sweep_leases` + `relay_lease_sweep.rs`)

In order, one transaction, fed `now`:
1. expire lanes, interrupt their worker tasks (as today; the active set is `assigned|working|rework|blocked`, not `in_review`);
2. paused lanes with a live lease: interrupt their worker tasks (as today);
3. **end pauses** whose `limit_resumes_at <= now` (`update relay.lanes set limit_paused_at = null, limit_scope = null, limit_resumes_at = null, limit_note = null … returning id, crew_id`) → `LaneChange::Resumed(LimitPauseEnded)`;
4. **re-offer to the pool**: interrupted past `REOFFER_GRACE_SECS` → `ready`, `lane_id = null`, `reoffered` action (no `to_lane`);
5. **abandoned review claims**: `in_review` with `review_claimed_by` whose newest lane lease has `expires_at <= now - 60 s` (or lane unregistered) → claim cleared, `released {kind: review, reason: "lease lapsed"}` with `lane_id` = the lapsed reviewer;
6. **waiters**: delete `relay.lane_waiters` whose lane is unregistered or whose newest lane lease expired;
7. **nudges**: for every floating review with `state_changed_at <= now - NUDGE_AFTER_SECS` and no `message` action with `props.nudge = true and props.task_id = <id>` newer than `now - NUDGE_AFTER_SECS`, insert one `message` to `everyone` (`props: {to: {kind: everyone}, surface: lane, nudge: true, reason: review_floating, waiting_min}`; `validate_props(Message)` allowlist gains `nudge`, `reason`, `waiting_min`, `task_id`);
8. resource leases (as today, with F10);
9. **broker tasks**: `select id, deterministic from relay.tasks where executor = 'broker' and state = 'ready' and not exists (unmet deps) order by state_changed_at for update skip locked limit 1` → run the gate (2.8) outside the transaction after commit, one per tick;
10. **complete runs** (2.5) → `RunChange::Completed` events carrying `run_id` and `crew_id`.

`RelaySweepReport` gains `released_reviews`, `resumed_lanes: Vec<(Uuid, Uuid)>`, `evicted_waiters`, `nudged`, `completed_runs: Vec<(Uuid, Uuid)>`, `gate_tasks: Vec<Uuid>`; `sweep_events` names each.

**Steps (expanded 2026-11-12):** Decisions beyond the task text. (1) Step 9 selects and reports; it moves nothing — the gate (2.8) writes the outcome in its own transaction after the commit, so a daemon lost mid-gate leaves no stuck `working` broker task (the next pass re-selects, and the gate is idempotent). (2) A released review claim is a task change (`RelaySweptTask::ReviewReleased` → `relay_task("release", id)`); a resumed lane rides `expired_lanes`' shape as `resumed_lanes: (crew_id, lane_id)`. (3) The nudge's `body_md` is the daemon's own text (title + minutes waiting), `lane_id` null, `task_id` the review; `waiting_min` is whole minutes since `state_changed_at`. (4) `relay_unregister` leaves a review claim held by the leaving lane to the sweeper (step 5: unregistered ⇒ released), one path for both lapses.
- [x] **S1 (red):** report fields + `RelaySweptTask::ReviewReleased` as stubs (empty); store pins `sweep_ends_a_pause_at_resumes_at_not_before`, `sweep_releases_an_abandoned_review_claim_after_the_grace`, `sweep_evicts_waiters_of_dead_lanes`, `sweep_nudges_everyone_about_a_floating_review_once_per_window`, `sweep_reports_a_ready_broker_task_whose_deps_are_done_one_per_pass`; ticker pin `sweep_events_name_every_lane_task_and_run_change` grows the resumed lane and the released review. Red on the current sweeper. (`d5f9c1952`: 6 red.)
- [x] **S2 (green):** steps 3, 5, 6, 7, 9 in `relay_sweep_leases` (fed `now`, one transaction), `sweep_events` + the tick's log line. Relay suite, clippy, fmt → green. Commit the pair. Executed: one `LIMIT_PAUSE_CLEAR` set-clause shared by the sweeper, `limit_resume` and `reregister`; the nudge's props go through `validate_props` (a shape error is `Internal`, a bug); the nudge pin's later passes keep the young review re-floated and the reviewer's lease live, since by the third pass both would have become stale/abandoned on their own — the sweeper was right, the fixture leaked.

## Task 2.8: Deterministic gate (`relay/gate.rs`)

```rust
pub struct GateInput { pub base_checkout: PathBuf, pub base_branch: String, pub branch: String, pub verify_cmd: Option<String>, pub timeout: Duration }
pub enum GateOutcome { Merged { sha: String }, Conflict { detail: String }, VerifyFailed { cmd: String, exit: i32, tail: String }, Error(String) }
pub trait GateRunner: Send + Sync { fn run(&self, input: &GateInput) -> GateOutcome; }
pub struct GitGateRunner; // real: git merge-tree --write-tree; git worktree add <tmp> <branch>; git merge <base>; <verify_cmd>; git -C base merge --no-ff <branch>; git worktree remove <tmp>
pub struct FakeGateRunner(pub GateOutcome); // tests
```
The broker task row: `deterministic = {"action": "gate", "task_id": "<approved task>", "branch": "<from the report>", "base_checkout": …, "base_branch": …, "verify_cmd": …}`, `deps = [approved task]`, `title = "gate: <task title>"`, `repo_key` = the task's. Outcome handling (one transaction): `Merged` → agent task `done` + `merged` action (`lane_id = null`, props `{commits: [sha], into: base_branch, gates: [{cmd, exit: 0}]}`), broker task `done`; `Conflict`/`VerifyFailed` → agent task `rework` (round + 1) + `reviewed` action (`lane_id = null`, `verdict: changes_requested`, one finding `{severity: high, class: merge_conflict | build_failure, text, detail: tail}`), broker task `done`; `Error` → broker task `blocked` with an `asked` to the person (`asked_of: person`, options `retry | skip_gate`), agent task stays `approved`. Seed the two finding classes in `database/import/staging/finding_classes.*` and the `staging.import_finding_classes` path (`dbd import` on the live DB).

**Steps (expanded 2026-11-12):** Decisions beyond the task text. (1) The gate task carries **no deps**: `deps = [approved task]` would wait for `done`, which only the gate produces — its precondition (approval) is met the moment it is minted; `deterministic.task_id` names the task. (2) `GateRunner::run` is async in the house style (`relay_drivers/trait_def.rs`: return-position `impl Future`, static dispatch); the sweeper's `spawn` takes the runner, tests pass `FakeGateRunner`. (3) `GitGateRunner` refuses to touch a base checkout that is dirty or not on `base_branch` (`Error`), checks `git merge-tree --write-tree` first (`Conflict`), verifies in a detached temporary worktree with base merged in (`VerifyFailed {cmd, exit, tail}`), then `git merge --no-ff` on the base (`Merged {sha}`); the worktree is always removed; one `timeout` budget covers the whole gate (`deterministic.timeout_seconds`, default 900). (4) A gate finding's `detail` (the tail) rides the `reviewed` action's `body_md`; `text` stays a summary (the finding shape allows no `detail` key). (5) Answering the error ask: `retry` resumes the broker task to `ready` (the next pass re-selects it); `skip_gate` cancels it and leaves the agent task `approved` for a manual merge. (6) The gate runs in the tick after the sweep commits, one task per pass, and announces `relay_task("gate", <agent task>)`; the run completes on the following pass. (7) No gate task when the task has no run, the run's config lacks `base_checkout`/`base_branch`, or `auto_merge` is `false` — the task stays `approved` for the manual `merge`.
- [x] **S1 (red):** `relay/gate.rs` (`GateInput`, `GateOutcome`, `GateRunner`, `FakeGateRunner`, `GateSpec` parse of `deterministic`, `GitGateRunner` stub); `TaskEvent::GateFailed` (`Approved → Rework`) and `Ready → Blocked` (a gate that cannot run blocks its broker task in the pool); store stubs `relay_gate_task`/`relay_gate_outcome`, ticker `pass` stub; pins: `gate_spec_parses_and_refuses_bad_shapes`, `git_gate_merges_a_clean_branch`, `git_gate_reports_a_conflict`, `git_gate_reports_a_failed_verify`, `git_gate_refuses_a_dirty_or_wrong_base`, `approving_a_branched_task_in_a_gated_run_mints_the_gate_task`, `approval_mints_no_gate_without_a_run_config_or_with_auto_merge_off`, `gate_merged_ends_the_task_done_with_a_merged_row`, `gate_conflict_reworks_with_a_merge_conflict_finding`, `gate_verify_failure_reworks_with_a_build_failure_finding`, `gate_error_blocks_the_gate_task_and_asks_the_person`, `answering_retry_reoffers_and_skip_gate_cancels_the_gate_task`, `pass_runs_the_handed_over_gate_and_announces_it`; SQL `relay_finding_classes.sql` pins twelve classes. Red: 12 Rust pins fail (the spec/fake/no-gate pins hold as guards).
- [ ] **S2 (green):** the runner, the mint on approval, the outcome write, the answer options, the tick wiring, the seed rows. Relay suite (+ `make test-template` for the seed), SQL relay suite, clippy, fmt → green. Commit the pair.

## Task 2.9: Handlers, routes, events

Routes (`routes.rs`): `POST /api/relay/tasks/claim_next`, `POST /api/relay/tasks/{id}/claim`, `POST /api/relay/tasks/{id}/claim_review`, `POST /api/relay/tasks/{id}/checkpoint`, `POST /api/relay/tasks/{id}/release`, `POST /api/relay/limit_resume`, `GET /api/relay/queue` (lane; `relay.task_queue` for the lane's crew), `POST /relay/runs` (person).

Bodies: `ClaimBody {lane_id}`; `CheckpointBody {lane_id, current_step: Option<String>, notes: Option<String>, steps: Vec<StepBody-without-lane_id>}`; `ReleaseBody {lane_id, reason: Option<String>}`; `CreateRunBody {crew_id: Option<String>, title, goal, item_ids: Vec<String>, repo_key: Option<String>, workspace_id: Option<String>, config: Option<RunConfigBody>}`, `RunConfigBody {base_checkout: Option<String>, base_branch: Option<String>, verify_cmd: Option<String>, auto_merge: Option<bool>}` with `#[serde(deny_unknown_fields)]`; `RegisterBody.role: Option<String>`, `.suggested_name: Option<String>`; `WaitQuery {lane_id, timeout_ms: Option<u64>, timeout_seconds: Option<u64>}` and `TasksQuery` both `#[serde(deny_unknown_fields)]` (F11, #404).

Responses: `claim_next` → `{claim_type: review|task|none, task: {...}}` per design §6.3; `report` 409 → `{error: "relay_upstream_deps_unmet", message}`; `POST /relay/runs` → 201 `{run_id, planning_task_id, crew_id, status: "planning"}`.

`wait_for_task`: insert the waiter row before the loop (`insert … on conflict (user_id, lane_id) do update set last_poll_at = excluded.last_poll_at`), delete it on every exit path (answer, timeout, error) — a `scopeguard`-style drop or explicit deletes; the row's `waiting_since` is kept across consecutive empty holds (upsert does not touch it).

Events: `StateEvent::relay_task("claim" | "claim_review" | "checkpoint" | "release" | "unpack" | "gate", id)`; `RunChange::{Created, Activated, Completed, Archived, Deleted}` and `relay_run(change, run_id, crew_id)` (data carries both; the existing `close_crew` passes the crew id as both, keeping the §3k contract); `LaneChange::Resumed(LimitPauseEnded)` emitted by the sweeper.

## Task 2.10: Converge the live DB, install, smoke

- `cd database && dbd reconcile --dry-run -d postgresql://localhost:5432/sensei` → read the plan; then `dbd reconcile --allow-destructive -d …/sensei` (drops `reviewer_lane_id` after `update relay.tasks set review_claimed_by = reviewer_lane_id where state = 'in_review'` — zero rows today, run anyway). For the two index-predicate changes reconcile may skip: `psql …/sensei -c "drop index if exists relay.tasks_one_active_per_lane; drop index if exists relay.lanes_one_coordinator;"` then `dbd reconcile` again (creates the new ones from the files). `dbd import` for the finding classes.
- `make crates && make install-service`; `launchctl kickstart -k` the daemon per the install target's output; `GET /health` green.
- New `scripts/relay-queue-smoke.sh`: register two lanes (no role, no crew_id → both join the active crew); `POST /relay/runs` with one backlog item; lane A `claim_next` → Task 0; A reports a two-task plan JSON; lane B `claim_next` → the review; B approves → run active, two ready tasks; A and B each `claim_next` → one task each (the second early-claim gated); B reports → 409 until A's task is done; A reports → floats; B `claim_next` → A's review (never its own); B approves → gate task appears; the sweeper tick runs the gate (`auto_merge: false` in the smoke run → `done` without merge); run completes when both are done. Every step asserts the HTTP status and the `relay.now` section that should have moved.
- Commit series on the branch; merge to develop green; `/sensei:checkpoint`.
