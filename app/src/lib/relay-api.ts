// Relay · the desktop's client contract for the daemon's relay API (D90, slice 1 item 4).
//
// The shapes came first (phase A), so the screen's tests can mock the client; the
// runtime factory below is phase B's. The screen reaches the daemon through it alone.
//
// Paths: the daemon's relay routes are L3's to publish (ruling Q9). Until they are,
// these are the ones the screen asks for — `relay/now`, `relay/queue`, `relay/tasks/
// {crew}/{task}`, and the three writes — and a change on the daemon's side is a change
// here, in one file.
//
// Shapes follow `docs/prd/modules/03-delivery/21-relay/lanes-data.md` (the views
// `relay.now` and `relay.task_history`, and the action kinds) and the interaction
// contract `coord/mockups/relay-lanes/CONTRACT.md` §3.
//
// Repository required, workspace optional (lanes-data, user 2026-11-05): every task names its
// `repo_key`; `workspace_id` is a tag and a filter, never a gate. Reads are scoped by the user and
// the crew. The main window reads every crew; the workspace window reads the same views filtered
// to its workspace.
//
// D83: every self-reported value is nullable. `null` means "not reported" and the
// screen says so; it is never coerced to 0. Times are ISO instants; the screen
// formats them.

import { apiBase, type ApiResult } from './api.js';
import { EventManager } from './events.js';

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
  | 'compacted'
  | 'handed_over';

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
  /** An open `asked` action blocks it: the task reads "needs your answer". */
  needs_answer: boolean;
}

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
  assigned_at: string;
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

// ─── The client ─────────────────────────────────────────────────────────────

export interface RelayApi {
  tryGetNow(filter: RelayFilter): Promise<ApiResult<RelayNow>>;
  tryGetQueue(filter: RelayFilter): Promise<ApiResult<RelayQueue>>;
  /** Scoped by (user, crew, task): another user's task, or one from a crew you're not in, is a 404. */
  tryGetTask(crewId: string, taskId: string): Promise<ApiResult<RelayTaskView>>;
  trySendMessage(body: RelaySendMessage): Promise<ApiResult<{ message_id: string }>>;
  tryAnswerDecision(body: RelayAnswer): Promise<ApiResult<{ action_id: string }>>;
  tryAddToBacklog(body: RelayAddToBacklog): Promise<ApiResult<{ item_id: string }>>;
}

/** What the screens build once from the daemon's port. */
export type RelayApiFactory = (port: number) => RelayApi;

/** `?workspace_id=…&repo_key=…`, empty when nothing narrows the read. */
const filterQuery = (filter: RelayFilter): string => {
  const params = new URLSearchParams();
  if (filter.workspace_id) params.set('workspace_id', filter.workspace_id);
  if (filter.repo_key) params.set('repo_key', filter.repo_key);
  const q = params.toString();
  return q ? `?${q}` : '';
};

/** The relay client: every call the daemon's answer or an `ApiResult` error, the
 *  daemon's own refusal text when it sends one (a 409's reason is the useful part). */
export const relayApi: RelayApiFactory = (port) => {
  const base = apiBase(port);

  async function call<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method,
        ...(body === undefined ? {} : { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),
      });
      if (res.status === 204) return { ok: true, data: undefined as T };
      if (res.ok) return { ok: true, data: (await res.json()) as T };
      try {
        const parsed = (await res.json()) as Record<string, unknown> | null;
        const reason = parsed && typeof parsed.error === 'string' && parsed.error ? parsed.error : res.statusText;
        return { ok: false, error: { status: res.status, message: reason } };
      } catch {
        return { ok: false, error: { status: res.status, message: res.statusText } };
      }
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  return {
    tryGetNow: (filter) => call<RelayNow>('GET', `/relay/now${filterQuery(filter)}`),
    tryGetQueue: (filter) => call<RelayQueue>('GET', `/relay/queue${filterQuery(filter)}`),
    tryGetTask: (crewId, taskId) =>
      call<RelayTaskView>('GET', `/relay/tasks/${encodeURIComponent(crewId)}/${encodeURIComponent(taskId)}`),
    trySendMessage: (body) => call<{ message_id: string }>('POST', '/relay/messages', body),
    tryAnswerDecision: (body) => call<{ action_id: string }>('POST', '/relay/answers', body),
    tryAddToBacklog: (body) => call<{ item_id: string }>('POST', '/relay/backlog', body),
  };
};

// ─── Stream (F8; the dojo seam) ─────────────────────────────────────────────

/** One relay-socket event: the shared {action, entity, data} contract.
/// Task notifications name the task (the route re-reads its view); step
/// events carry the step (applied without re-reading); op-only relay
/// events name the op (unregister re-reads). The dojo replays this same
/// contract over Supabase realtime — the route dispatches, never the transport. */
export interface RelayStreamEvent {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;
  data: { op?: string; task_id?: string; step?: RelayTaskStep };
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

/** The relay socket, owned by the route that subscribes (never by shared state). */
export const relayStream: (port: number) => EventManager<RelayStreamEvent> = (port) =>
  new EventManager<RelayStreamEvent>(`${apiBase(port)}/api/relay/events`, parseRelayStreamEvent);
