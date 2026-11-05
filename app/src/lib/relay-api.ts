// Relay · the desktop's client contract for the daemon's relay API (D90, slice 1 item 4).
//
// TYPES ONLY in phase A. The daemon routes do not exist yet (item 2 builds the store
// and the MCP tools); this file fixes what the Relay screen reads and writes so the
// screen's tests can mock it. Phase B adds the runtime factory here,
//
//     export const relayApi: RelayApiFactory = (port) => ({ … });
//
// and the screen reaches the daemon through it alone.
//
// Shapes follow `docs/prd/modules/03-delivery/21-relay/lanes-data.md` (the views
// `relay.now` and `relay.task_history`, and the action kinds) and the interaction
// contract `coord/mockups/relay-lanes/CONTRACT.md` §3.
//
// D83: every self-reported value is nullable. `null` means "not reported" and the
// screen says so; it is never coerced to 0. Times are ISO instants; the screen
// formats them.

import type { ApiResult } from './api.js';

// ─── Vocabulary (lanes-data.md enums) ───────────────────────────────────────

export type RelayRole = 'coordinator' | 'contributor';

export type RelayTaskPhase = 'analysis' | 'tests' | 'impl' | 'docs' | 'mockup';

export type RelayTaskState =
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
  | 'compacted';

/** The tracker ticket an item references by its portable identity, resolved on this side. */
export interface RelayTicket {
  provider: RelayTicketProvider;
  key: string;
  url: string | null;
}

// ─── relay.now ──────────────────────────────────────────────────────────────

export interface RelayCrew {
  id: string;
  workspace_id: string;
  goal: string | null;
  status: string;
  started_at: string;
  heartbeat_at: string | null;
}

/** The one active task a lane is on (one active task per lane). */
export interface RelayLaneTask {
  id: string;
  title: string;
  phase: RelayTaskPhase;
  state: RelayTaskState;
  round: number;
  state_changed_at: string;
  /** An open `asked` action blocks it: the task reads "needs your answer". */
  needs_answer: boolean;
}

export interface RelayLane {
  id: string;
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
  task: RelayLaneTask | null;
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
  task_id: string | null;
  task_title: string | null;
  /** `body_md` of the `asked` action. */
  question: string;
  context: string | null;
  options: RelayDecisionOption[];
  recommended: string | null;
  asked_at: string;
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

/** `relay.now` for one workspace. `crew: null` = no crew yet (not a failure). */
export interface RelayNow {
  crew: RelayCrew | null;
  lanes: RelayLane[];
  decisions: RelayDecision[];
  triage: RelayTriageSource[];
  completed_today: RelayCompletedTask[];
  today: RelayToday;
  resources: RelayResources;
}

// ─── The queue (relay.items + the triage view) ──────────────────────────────

/** A triage row: a tracker task in an active sprint with no item yet, or a manual item. */
export interface RelayTriageItem {
  /** Set for a manual item; null for a tracker task that has no item yet. */
  item_id: string | null;
  kind: RelayItemKind;
  title: string;
  /** Only for manual items; a ticketed row reads its tags from the tracker. */
  tags: string[];
  sprint: string | null;
  ticket: RelayTicket | null;
  added_by_you: boolean;
}

export interface RelayBacklogItem {
  item_id: string;
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
}

export interface RelayTaskHeader {
  id: string;
  title: string;
  phase: RelayTaskPhase;
  state: RelayTaskState;
  round: number;
  ticket: RelayTicket | null;
  lane: RelayTaskLane | null;
  assigned_at: string;
  completed_at: string | null;
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

/** One `relay.actions` row, props unpacked. `lane_code: null` = the person acted. */
export interface RelayAction {
  id: string;
  kind: RelayActionKind;
  lane_code: string | null;
  round: number | null;
  phase: RelayTaskPhase | null;
  body_md: string | null;
  props: Record<string, unknown>;
  reply_to: string | null;
  created_at: string;
}

export interface RelayTaskView {
  task: RelayTaskHeader;
  run: RelayRunDetails;
  decision: RelayDecision | null;
  handoff: RelayHandoff | null;
  /** Newest first, down to `assigned`, as the view orders it. */
  history: RelayAction[];
}

// ─── Writes (person → broker) ───────────────────────────────────────────────

/** CONTRACT §3h-bis `send_message`. */
export interface RelaySendMessage {
  crew_id: string;
  to: { lane_id: string } | { role: 'coordinator' } | { all: true };
  /** "About": null = no particular task. */
  task_id: string | null;
  /** ≤ 600 characters, plain text. */
  text: string;
  surface: 'desktop' | 'dojo';
}

/** CONTRACT §3e `answer_decision`. */
export interface RelayAnswer {
  decision_id: string;
  option: string;
  note: string | null;
  surface: 'desktop' | 'dojo';
}

/** CONTRACT §3h `add_to_backlog`: a manual item by id, a tracker task by its identity. */
export type RelayAddToBacklog =
  | { item_id: string }
  | { ticket_provider: RelayTicketProvider; ticket_key: string };

// ─── The client ─────────────────────────────────────────────────────────────

export interface RelayApi {
  tryGetNow(workspaceId: string): Promise<ApiResult<RelayNow>>;
  tryGetQueue(workspaceId: string): Promise<ApiResult<RelayQueue>>;
  tryGetTask(taskId: string): Promise<ApiResult<RelayTaskView>>;
  trySendMessage(body: RelaySendMessage): Promise<ApiResult<{ message_id: string }>>;
  tryAnswerDecision(body: RelayAnswer): Promise<ApiResult<{ action_id: string }>>;
  tryAddToBacklog(body: RelayAddToBacklog): Promise<ApiResult<{ item_id: string }>>;
}

/** What phase B exports as `relayApi`, from the daemon's port. */
export type RelayApiFactory = (port: number) => RelayApi;
