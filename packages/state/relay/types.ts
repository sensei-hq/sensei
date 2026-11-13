// The relay domain's wire shapes: what the daemon's relay API answers and what the
// person's writes send (D90). Moved from the app's `relay-api.ts` (#421) so the
// domain states here, the app and the dōjō share one copy; the app's client
// re-exports them.
//
// Shapes follow `docs/prd/modules/03-delivery/21-relay/lanes-data.md` (the views
// `relay.now` and `relay.task_history`, and the action kinds) and the interaction
// contract `coord/mockups/relay-lanes/CONTRACT.md` §3.
//
// D83: every self-reported value is nullable. `null` means "not reported" and the
// screen says so; it is never coerced to 0. Times are ISO instants; the screen
// formats them.

// ─── Vocabulary (lanes-data.md enums) ───────────────────────────────────────

export type RelayRole = 'coordinator' | 'contributor';

/** `planning` is a sprint run's Task 0 (queue cutover). */
export type RelayTaskPhase = 'analysis' | 'tests' | 'impl' | 'docs' | 'mockup' | 'planning';

/** `ready` is the pool: minted there, unclaimed, on no lane (queue cutover). */
export type RelayTaskState =
  | 'ready'
  | 'assigned'
  | 'working'
  | 'in_review'
  | 'rework'
  | 'approved'
  | 'done'
  | 'blocked'
  | 'interrupted'
  | 'cancelled';

/** A lane's liveness is its lane lease (`relay.leases`, kind `lane`). */
export type RelayLeaseState = 'queued' | 'granted' | 'released' | 'expired' | 'refused';

export type RelayItemKind = 'bug' | 'feature' | 'gap' | 'issue' | 'finding';

export type RelayTicketProvider = 'jira' | 'ado' | 'github';

/** `relay.actions.kind`. `delivered` / `read` track a message (`reply_to` = the message). */
export type RelayActionKind =
  | 'assigned'
  | 'brief_updated'
  | 'started'
  | 'reported'
  | 'reviewed'
  | 'asked'
  | 'answered'
  | 'message'
  | 'delivered'
  | 'read'
  | 'merged'
  | 'interrupted'
  | 'reoffered'
  | 'compacted'
  | 'handed_over'
  // Queue cutover: a claim, a review claim, a release, an in-turn save.
  | 'claimed'
  | 'review_claimed'
  | 'released'
  | 'checkpointed';

/** `relay.runs.state` (queue cutover): a sprint run's lifecycle. */
export type RelayRunState = 'planning' | 'active' | 'completed' | 'cancelled';

/** The queue's verdict on a claimed task's deps (queue cutover). */
export type RelayDepsStatus = 'satisfied' | 'early_claim_gated';

/** The tracker ticket an item references by its portable identity, resolved on this side. */
export interface RelayTicket {
  provider: RelayTicketProvider;
  key: string;
  url: string | null;
}

// ─── relay.now ──────────────────────────────────────────────────────────────

/** What a read is narrowed to. Empty = every crew (the main window). */
export interface RelayFilter {
  workspace_id?: string;
  /** The portable identity, e.g. `github.com/sensei-hq/sensei`. */
  repo_key?: string;
}

export interface RelayCrew {
  id: string;
  /** The crew's repository (`null` = none — Q18; the screen reads it as "no repository · crew XXXX"); the main window groups its rows under it (L0 ruling, r2). */
  repo_key: string | null;
  /** Optional: a crew may work on repositories that belong to no workspace. */
  workspace_id: string | null;
  goal: string | null;
  status: string;
  started_at: string;
  heartbeat_at: string | null;
}

/** The one active task a lane is on (one active task per lane). */
export interface RelayLaneTask {
  id: string;
  title: string;
  /** Required: the repository the task works on. */
  repo_key: string;
  workspace_id: string | null;
  phase: RelayTaskPhase;
  state: RelayTaskState;
  round: number;
  state_changed_at: string;
  /** The holder's last checkpointed step; null when none was reported. */
  current_step: string | null;
  /** Checklist steps settled (done or skipped), of `steps_total`. */
  steps_done: number;
  steps_total: number;
  /** An open `asked` action blocks it: the task reads "needs your answer". */
  needs_answer: boolean;
}

/** Which quota a limit-paused lane ran out of. */
export type RelayLimitScope = 'daily' | 'weekly' | 'other';

export interface RelayLane {
  id: string;
  crew_id: string;
  /** `L1`…, unique per crew. */
  code: string;
  /** The lane's objective ("Diagrams"); null until the coordinator's first assignment. */
  title: string | null;
  role: RelayRole;
  family: string;
  version: string | null;
  model: string | null;
  lease_state: RelayLeaseState;
  lease_expires_at: string | null;
  /** Set exactly when the lane reported quota exhaustion (lease may be live). */
  limit_scope: RelayLimitScope | null;
  /** Quota-reset instant; null when unknown (any beat resumes). */
  limit_resumes_at: string | null;
  /** The lane's own words on the pause (audit; shown nowhere yet). */
  limit_note: string | null;
  task: RelayLaneTask | null;
  /** The name the assistant proposed for itself (queue cutover); null when it offered none. */
  suggested_name: string | null;
  /** The review this lane holds (`review_claimed_by`), beside its own task; null when none. */
  reviewing: RelayLaneReview | null;
}

/** The review a lane holds (`relay.now` lane payload `reviewing`). */
export interface RelayLaneReview {
  id: string;
  title: string;
  phase: RelayTaskPhase;
  round: number;
  claimed_at: string;
}

/** One row of `relay.now`'s `sprint_summary`: an open run with its tasks counted by state. */
export interface RelayRunSummary {
  run_id: string;
  crew_id: string;
  title: string;
  goal: string;
  state: RelayRunState;
  created_at: string;
  started_at: string | null;
  total_tasks: number;
  ready_tasks: number;
  working_tasks: number;
  in_review_tasks: number;
  floating_reviews: number;
  approved_tasks: number;
  done_tasks: number;
  blocked_tasks: number;
  interrupted_tasks: number;
}

/** One row of `relay.now`'s `floating_review`: a report waiting for a reviewer. */
export interface RelayFloatingReview {
  task_id: string;
  crew_id: string;
  run_id: string | null;
  title: string;
  phase: RelayTaskPhase;
  round: number;
  repo_key: string;
  author_lane_code: string;
  author_lane_title: string | null;
  reported_at: string;
}

export interface RelayDecisionOption {
  id: string;
  label: string;
  detail: string | null;
}

/** An open `asked` action for the person (`props.asked_of = 'person'`). */
export interface RelayDecision {
  /** The `asked` action's id; the answer replies to it. */
  id: string;
  crew_id: string;
  task_id: string | null;
  task_title: string | null;
  /** `body_md` of the `asked` action. */
  question: string;
  context: string | null;
  options: RelayDecisionOption[];
  recommended: string | null;
  asked_at: string;
}

/** An unread `message` action addressed to the person (`to.kind = 'person'`). */
export interface RelayPersonMessage {
  /** The `message` action's id. */
  id: string;
  crew_id: string;
  task_id: string | null;
  task_title: string | null;
  /** The sending lane's code; null only for person-sent echoes. */
  from_lane_code: string | null;
  /** `body_md` of the `message` action. */
  text: string;
  sent_at: string;
}

/** Triage grouped by where it came from: one row per tracker + sprint, plus manual items. */
export interface RelayTriageSource {
  provider: RelayTicketProvider | 'manual';
  /** Sprint, iteration or milestone; null for manual items. */
  sprint: string | null;
  count: number;
}

export interface RelayCompletedTask {
  task_id: string;
  crew_id: string;
  repo_key: string;
  title: string;
  lane_code: string;
  lane_title: string | null;
  /** merged = done with a merge; approved = approved, not yet merged; done = nothing to merge. */
  outcome: 'merged' | 'approved' | 'done';
  completed_at: string;
}

/** The info band's "Today". `first_pass_approved` is null when no review was reported. */
export interface RelayToday {
  completed: number;
  reviewed: number | null;
  first_pass_approved: number | null;
}

/** The footer's resource line (lanes.md §8). Leases never leave the machine. */
export interface RelayResources {
  build_lease: { resource: string; lane_code: string; granted_at: string } | null;
  disk_free_gb: number | null;
  disk_floor_gb: number;
  swap_free_gb: number | null;
  swap_floor_gb: number;
}

/** The choices the main window's filter band offers. */
export interface RelayFacets {
  repos: string[];
  workspaces: { id: string; name: string }[];
}

/** `relay.now` for a filter. `crews: []` = no crew yet (not a failure). */
export interface RelayNow {
  crews: RelayCrew[];
  facets: RelayFacets;
  lanes: RelayLane[];
  /** Open sprint runs in created order (queue cutover): the band above the lanes. */
  runs: RelayRunSummary[];
  /** Reports waiting for a reviewer, oldest first (queue cutover). */
  floating_reviews: RelayFloatingReview[];
  decisions: RelayDecision[];
  person_messages: RelayPersonMessage[];
  triage: RelayTriageSource[];
  completed_today: RelayCompletedTask[];
  today: RelayToday;
  resources: RelayResources;
}

// ─── The queue (relay.items + the triage view) ──────────────────────────────

interface RelayTriageBase {
  /** A manual item may not name a repository yet. */
  repo_key: string | null;
  workspace_id: string | null;
  kind: RelayItemKind;
  title: string;
  /** Only for manual items; a ticketed row reads its tags from the tracker. */
  tags: string[];
  /** The tracker's own name for it ("sprint 47", "milestone 11"); null for a manual item. */
  sprint: string | null;
  added_by_you: boolean;
}

/** A triage row. Exactly one identity, so `+` always has something to send (r2 F10):
 *  a manual item by its id, or a tracker task (no item yet) by its ticket. */
export type RelayTriageItem =
  | (RelayTriageBase & { item_id: string; ticket: null })
  | (RelayTriageBase & { item_id: null; ticket: RelayTicket });

export interface RelayBacklogItem {
  item_id: string;
  repo_key: string | null;
  workspace_id: string | null;
  kind: RelayItemKind;
  title: string;
  tags: string[];
  sprint: string | null;
  ticket: RelayTicket | null;
  added_by_you: boolean;
  /** Backlog order; the coordinator takes the lowest next. */
  position: number;
}

export interface RelayQueue {
  triage: RelayTriageItem[];
  backlog: RelayBacklogItem[];
}

// ─── relay.task_history ─────────────────────────────────────────────────────

export interface RelayTaskLane {
  id: string;
  code: string;
  title: string | null;
  family: string;
  version: string | null;
  model: string | null;
  /** Set exactly when the lane reported quota exhaustion. */
  limit_scope: RelayLimitScope | null;
  /** Quota-reset instant; null when unknown. */
  limit_resumes_at: string | null;
  /** The lane's own words on the pause (audit; shown nowhere yet). */
  limit_note: string | null;
}

export interface RelayTaskHeader {
  id: string;
  crew_id: string;
  title: string;
  repo_key: string;
  workspace_id: string | null;
  phase: RelayTaskPhase;
  state: RelayTaskState;
  round: number;
  ticket: RelayTicket | null;
  lane: RelayTaskLane | null;
  /** The sprint run it belongs to; null outside a run (queue cutover). */
  run_id: string | null;
  /** How the queue classes its deps right now; the report is gated while early. */
  deps_status: RelayDepsStatus;
  /** The deps not done yet, in id order; empty when satisfied. */
  unmet_deps: string[];
  /** Null while the task waits in the pool (queue cutover). */
  assigned_at: string | null;
  completed_at: string | null;
  /** F8: every transition stamps it; the timeline stamps derived rows from it. */
  state_changed_at: string;
  /** Rounds by phase ("tests 1 · impl 2"). */
  rounds: { phase: RelayTaskPhase; count: number }[];
  needs_answer: boolean;
}

/** Totals over the task's turns, computed by the view (the screen adds nothing up).
 *  A total skips nulls; `reported_turns` of `turns` says how many reported. */
export interface RelayRunDetails {
  repository: string | null;
  branch: string | null;
  commits: string[] | null;
  model: string | null;
  tokens_in: number | null;
  tokens_out: number | null;
  tokens_cache: number | null;
  context_tokens: number | null;
  compactions: number | null;
  peak_memory_mb: number | null;
  subagents: number | null;
  working_secs: number | null;
  waiting_secs: number | null;
  files_changed: number | null;
  files_outside_owned: number | null;
  gates_run: number | null;
  reported_turns: number;
  turns: number;
}

/** The hand-off: the `assigned` action's `body_md`, frozen (or the last `brief_updated`). */
export interface RelayHandoff {
  body_md: string;
  docs: string[];
  at: string;
}

// ─── Action props, per kind (lanes-data.md `relay.actions` table; the daemon validates the keys) ──
// D83: every self-reported key is optional and nullable; absent or null = "not reported", never 0.

export interface RelayFile {
  path: string;
  added: number;
  removed: number;
  /** Computed by the broker from the task's `owned_paths`, not claimed by the lane. */
  outside_owned: boolean;
}

export interface RelayGate {
  cmd: string;
  exit: number;
  counts: string | null;
}

export interface RelaySubagent {
  role: string;
  model: string | null;
}

export interface RelayTelemetry {
  family?: string | null;
  version?: string | null;
  model?: string | null;
  tokens_in?: number | null;
  tokens_out?: number | null;
  tokens_cache?: number | null;
  started_at?: string | null;
  ended_at?: string | null;
  context_tokens?: number | null;
  compactions?: number | null;
  peak_memory_mb?: number | null;
  subagents?: RelaySubagent[] | null;
}

export type RelayFindingSeverity = 'high' | 'medium' | 'low';

export interface RelayFinding {
  severity: RelayFindingSeverity;
  /** A `relay.finding_classes` code. */
  class: string;
  text: string;
  file: string | null;
  line: number | null;
}

/** `message.props.to` (L0 ruling 3; Q2's person target is accepted for slice 2). */
export type RelayMessageTo =
  | { kind: 'coordinator' }
  | { kind: 'lane'; lane_id: string }
  | { kind: 'everyone' }
  | { kind: 'person' };

/** Where an action was taken: the desktop, the dōjō, or a lane (Q12). */
export type RelaySurface = 'desktop' | 'dojo' | 'lane';

/** Props by action kind, exactly the lanes-data table. */
export interface RelayActionProps {
  assigned: { docs: string[]; owned_paths: string[]; acceptance: string[] };
  brief_updated: { docs: string[] };
  started: Record<string, never>;
  reported: RelayTelemetry & {
    /** F1: a partial turn records progress only — the task keeps working, no reviewer picked. */
    partial?: boolean | null;
    look_at?: string | null;
    flags?: string | null;
    base?: string | null;
    branch?: string | null;
    commits?: string[];
    files?: RelayFile[];
    gates?: RelayGate[];
  };
  reviewed: RelayTelemetry & {
    verdict: 'approved' | 'changes_requested';
    findings: RelayFinding[];
    probes?: number | null;
  };
  asked: {
    /** One line: what is waiting on it (L0 ruling 1). */
    context: string | null;
    options: RelayDecisionOption[];
    recommended: string | null;
    blocks: string[];
    /** `person`, or a lane id. */
    asked_of: string;
  };
  answered: { option: string; surface: RelaySurface };
  message: { to: RelayMessageTo; surface: RelaySurface };
  /** Receipts: `reply_to` = the message, one per recipient (L0 ruling 2). */
  delivered: Record<string, never>;
  read: Record<string, never>;
  merged: { commits: string[]; into: string; gates: RelayGate[] };
  interrupted: { checkpoint?: unknown; to_lane?: string | null };
  /** `to_lane` is the lane's uuid; the screen shows its code. */
  reoffered: { checkpoint?: unknown; to_lane: string | null };
  compacted: { context_tokens_before?: number | null; context_tokens_after?: number | null };
  /** F1: uuids read as codes via the crew's lanes; `from_state` is the state word the task left. */
  handed_over: { from_lane: string; from_state: RelayTaskState; to_lane: string };
  // Queue cutover (the daemon's `validate_props` allowlists).
  claimed: { claim_type: 'task' | 'review'; deps_status?: RelayDepsStatus; unmet_deps?: string[] };
  review_claimed: Record<string, never>;
  released: { kind: 'task' | 'review'; reason?: string | null };
  checkpointed: { current_step?: string | null; notes?: string | null };
}

interface RelayActionBase {
  id: string;
  /** Who acted, by code; null = the person. */
  lane_code: string | null;
  round: number | null;
  phase: RelayTaskPhase | null;
  body_md: string | null;
  /** An answer → its question; a review → the report; a receipt → the message. */
  reply_to: string | null;
  created_at: string;
}

/** One `relay.actions` row, props unpacked, discriminated on `kind` (r2 F9). */
export type RelayAction = {
  [K in RelayActionKind]: RelayActionBase & { kind: K; props: RelayActionProps[K] };
}[RelayActionKind];

/** A crew lane by id, so a uuid in props (`to.lane_id`, `to_lane`) is shown by its code. */
export interface RelayLaneRef {
  id: string;
  code: string;
  title: string | null;
}

/** One flat-checklist step (F8): the holder's turn narrated step by step. */
export interface RelayTaskStep {
  key: string;
  title: string;
  state: 'open' | 'done' | 'skipped';
  reason: string | null;
  updated_at: string;
}

export interface RelayTaskView {
  task: RelayTaskHeader;
  run: RelayRunDetails;
  decision: RelayDecision | null;
  /** The latest brief: the `assigned` one, or the last `brief_updated`. */
  handoff: RelayHandoff | null;
  /** Newest first, down to `assigned`, as the view orders it. */
  history: RelayAction[];
  /** The flat checklist, stored insertion order, served as-is. */
  steps: RelayTaskStep[];
  /** The crew's lanes, for the codes behind uuids in props. */
  lanes: RelayLaneRef[];
}

// ─── Writes (person → broker) ───────────────────────────────────────────────

/** CONTRACT §3h-bis `send_message`. */
export interface RelaySendMessage {
  crew_id: string;
  /** The shape the `message` action stores (L0 ruling 3). */
  to: RelayMessageTo;
  /** "About": null = no particular task. */
  task_id: string | null;
  /** ≤ 600 characters, plain text. */
  text: string;
  /** Person-only: 'lane' is set by the daemon, never sent (Q12). */
  surface: 'desktop' | 'dojo';
}

/** CONTRACT §3e `answer_decision`. */
export interface RelayAnswer {
  decision_id: string;
  option: string;
  note: string | null;
  /** No `by`: the daemon records the person as `lane_id = null` (lanes-data), CONTRACT §3e's `by: 'person'`. */
  /** Person-only: 'lane' is set by the daemon, never sent (Q12). */
  surface: 'desktop' | 'dojo';
}

/** CONTRACT §3h `add_to_backlog`: a manual item by id, a tracker task by its identity. */
export type RelayAddToBacklog =
  | { item_id: string }
  | { ticket_provider: RelayTicketProvider; ticket_key: string };

/** An open item the person adds to triage (CONTRACT §1d, §3h; `POST /relay/triage`). */
export interface RelayAddTriage {
  title: string;
  kind: RelayItemKind;
  tags: string[];
  /** A manual item may not name a repository yet. */
  repo_key: string | null;
  workspace_id: string | null;
}

/** The gate's knobs on a run (`relay.runs.config`); an unknown knob is a 422. */
export interface RelayRunConfig {
  base_checkout: string | null;
  base_branch: string | null;
  verify_cmd: string | null;
  auto_merge: boolean;
}

/** `POST /relay/runs` (queue cutover): a sprint run over backlog items. `crew_id`
 *  null = the one active crew (a 409 names the refusal when there are none or several). */
export interface RelayCreateRun {
  crew_id: string | null;
  title: string;
  goal: string;
  item_ids: string[];
  repo_key: string | null;
  workspace_id: string | null;
  config: RelayRunConfig;
}

/** The daemon's 201: the run, its Task 0 waiting in the pool, and the crew. */
export interface RelayCreatedRun {
  run_id: string;
  planning_task_id: string;
  crew_id: string;
  status: RelayRunState;
}

// ─── Stream (F8; the dōjō seam) ─────────────────────────────────────────────

/** One relay-socket event: the shared {action, entity, data} contract.
/// Task notifications name the task (the route re-reads its view); step
/// events carry the step (applied without re-reading); op-only relay
/// events name the op (unregister re-reads). The dojo replays this same
/// contract over Supabase realtime — the route dispatches, never the transport. */
export interface RelayStreamEvent {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;
  /** Run events carry `run_id` + `crew_id`; lane events `lane_id` + `crew_id` (queue cutover). */
  data: {
    op?: string;
    task_id?: string;
    step?: RelayTaskStep;
    run_id?: string;
    crew_id?: string;
    lane_id?: string;
    reason?: string;
  };
}

/** Shape-check one socket payload; throws (the manager skips it). */
export function parseRelayStreamEvent(raw: string): RelayStreamEvent {
  const parsed: unknown = JSON.parse(raw);
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    throw new Error('relay stream event is not an object');
  }
  const { action, entity, data } = parsed as Record<string, unknown>;
  if (typeof entity !== 'string' || typeof data !== 'object' || data === null || Array.isArray(data)) {
    throw new Error('relay stream event names no entity');
  }
  if (action !== 'add' && action !== 'update' && action !== 'remove' && action !== 'set') {
    throw new Error('relay stream event names no action');
  }
  return { action, entity, data: data as RelayStreamEvent['data'] };
}
