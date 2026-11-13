// The Now view's selectors (#422; CONTRACT §1c-2, §1c-3, §1h; COMPONENTS §3b): from the
// read to the rows LaneList and CompletedList draw. Pure — the read, a clock, the
// threshold and how a task is linked go in; row data comes out. A codename, a time or
// a resume the daemon did not give is not invented (D83).

import { crewGroups, crewRepoLabel, hhmm, providerLabel, since } from './format.js';
import { minutesSince } from './relay-selectors.js';
import type {
  RelayCompletedTask,
  RelayDecision,
  RelayLane,
  RelayNow,
  RelayQueue,
  RelayRole,
  RelayTicketProvider,
} from './types.js';

/** The tone a pill or a mark carries. */
export type RelayTone = 'accent' | 'warning' | 'danger' | 'success' | 'neutral';

/** One lane, as its row draws it (COMPONENTS §3b LaneRow). */
export interface RelayLaneRowView {
  id: string;
  code: string;
  role: RelayRole;
  /** The lane's objective, or its code until it has one. */
  name: string;
  /** "codename · assistant" (CONTRACT §1h). */
  who: string;
  /** The one task it is on, with phase and round; "Nothing assigned" when idle. */
  task: string;
  state: { label: string; tone: RelayTone };
  /** Time in that state; `over` = waiting on another lane past the threshold. */
  when: { text: string; over: boolean };
  /** The task's view; null when the lane has no task (no link, no chevron). */
  href: string | null;
}

/** The lanes of one crew, under its repository in the main window. */
export interface RelayLaneGroupView {
  id: string;
  /** null in the workspace window, which shows one crew's lanes flat. */
  label: string | null;
  rows: RelayLaneRowView[];
}

/** One task finished today, as its row draws it (COMPONENTS §3b CompletedRow). */
export interface RelayCompletedRowView {
  id: string;
  at: string;
  title: string;
  /** "objective · codename". */
  lane: string;
  outcome: { label: string; tone: 'success' };
  href: string;
}

export interface NowRowOptions {
  /** The clock. */
  at: number;
  /** lanes.md §15: a wait at or past this many minutes is too long. */
  threshold: number;
  /** A task's route in this window. */
  href: (crewId: string, taskId: string) => string;
  /** The main window groups the lanes by crew; the workspace window shows them flat. */
  byCrew: boolean;
}

// The assistant families the screen names; any other reads as the daemon wrote it, made readable.
const FAMILY_NAMES: Record<string, string> = {
  'claude-code': 'Claude Code',
  claude: 'Claude Code',
  codex: 'Codex',
  cursor: 'Cursor',
  'gemini-cli': 'Gemini CLI',
  gemini: 'Gemini CLI',
  zed: 'Zed',
  copilot: 'Copilot',
  cowork: 'Cowork',
};

/** "Claude Code 2.1": the family as people name it, and its version when reported. */
export function assistantLabel(family: string, version: string | null): string {
  const words = family.replace(/[-_]+/g, ' ').trim();
  const name = FAMILY_NAMES[family] ?? (words ? words[0].toUpperCase() + words.slice(1) : family);
  return version ? `${name} ${version}` : name;
}

/** The lane's codename, or its code until the daemon mints one (#395). */
const codename = (lane: Pick<RelayLane, 'suggested_name' | 'code'>): string => lane.suggested_name ?? lane.code;

/** The task states in which a lane waits on another lane (a reviewer, its author, a merge). */
const WAITING: Record<string, string> = {
  rework: 'verdict to pick up',
  approved: 'approved, not merged',
};

function laneRow(lane: RelayLane, now: RelayNow, o: NowRowOptions): RelayLaneRowView {
  const base = {
    id: lane.id,
    code: lane.code,
    role: lane.role,
    name: lane.title ?? lane.code,
    who: `${codename(lane)} · ${assistantLabel(lane.family, lane.version)}`,
  };
  const task = lane.task;
  const taskHref = task ? o.href(lane.crew_id, task.id) : null;
  const taskLine = task ? `${task.title} (${task.phase} r${task.round})` : 'Nothing assigned';

  // The lease first, then a quota pause, then the person's question, then the task.
  if (lane.lease_state === 'expired') {
    const when = lane.lease_expires_at ? `at ${hhmm(lane.lease_expires_at)}` : 'not reported';
    return { ...base, task: taskLine, state: { label: 'lease lost', tone: 'danger' }, when: { text: when, over: false }, href: taskHref };
  }
  if (lane.limit_scope) {
    // Which quota, and when it resumes (the limit-exhaustion wording).
    const when = `${lane.limit_scope} · ${lane.limit_resumes_at ? `until ${hhmm(lane.limit_resumes_at)}` : 'resume unknown'}`;
    return { ...base, task: taskLine, state: { label: 'paused', tone: 'warning' }, when: { text: when, over: false }, href: taskHref };
  }
  if (!task) {
    const review = lane.reviewing;
    if (review) {
      return {
        ...base,
        task: `Review · ${review.title} (${review.phase} r${review.round})`,
        state: { label: 'reviewing', tone: 'accent' },
        when: { text: since(review.claimed_at, o.at), over: false },
        href: o.href(lane.crew_id, review.id),
      };
    }
    // The daemon reports no idle-since: no time rather than an invented one.
    return { ...base, task: taskLine, state: { label: 'idle', tone: 'neutral' }, when: { text: '', over: false }, href: null };
  }

  const elapsed = { text: since(task.state_changed_at, o.at), minutes: minutesSince(task.state_changed_at, o.at) };
  if (task.needs_answer) {
    return { ...base, task: taskLine, state: { label: 'needs your answer', tone: 'warning' }, when: { text: elapsed.text, over: false }, href: taskHref };
  }
  const waitingFor =
    task.state === 'in_review'
      ? now.floating_reviews.some((r) => r.task_id === task.id)
        ? 'waiting · no reviewer'
        : 'in review'
      : WAITING[task.state];
  if (waitingFor) {
    return {
      ...base,
      task: taskLine,
      state: { label: waitingFor, tone: 'warning' },
      when: { text: elapsed.text, over: elapsed.minutes >= o.threshold },
      href: taskHref,
    };
  }
  const tone: RelayTone =
    task.state === 'blocked' || task.state === 'interrupted' || task.state === 'cancelled'
      ? 'danger'
      : task.state === 'done'
        ? 'success'
        : 'accent';
  const label = task.state === 'assigned' ? 'working' : task.state.replace(/_/g, ' ');
  return { ...base, task: taskLine, state: { label, tone }, when: { text: elapsed.text, over: false }, href: taskHref };
}

/** The lanes as LaneList draws them: one group in the workspace window, one per crew in the main window. */
export function toLaneGroups(now: RelayNow | null, o: NowRowOptions): RelayLaneGroupView[] {
  if (!now) return [];
  if (!o.byCrew) {
    return [{ id: now.crews[0]?.id ?? 'lanes', label: null, rows: now.lanes.map((l) => laneRow(l, now, o)) }];
  }
  return crewGroups(now.crews, now.lanes).map((g) => ({
    id: g.crew.id,
    label: crewRepoLabel(g.crew),
    rows: g.lanes.map((l) => laneRow(l, now, o)),
  }));
}

function completedLane(row: RelayCompletedTask, now: RelayNow): string {
  const lane = now.lanes.find((l) => l.crew_id === row.crew_id && l.code === row.lane_code);
  const who = lane ? codename(lane) : row.lane_code;
  return row.lane_title ? `${row.lane_title} · ${who}` : who;
}

/** Completed today as CompletedList draws it, in the read's order. */
export function toCompletedRows(now: RelayNow | null, o: NowRowOptions): RelayCompletedRowView[] {
  if (!now) return [];
  return now.completed_today.map((row) => ({
    id: row.task_id,
    at: hhmm(row.completed_at),
    title: row.title,
    lane: completedLane(row, now),
    outcome: { label: row.outcome, tone: 'success' as const },
    href: o.href(row.crew_id, row.task_id),
  }));
}

// ─── Waiting and Needs you (CONTRACT §1c-0 Waiting's filter, §1c-1, §3f-bis) ──────────

/** One task (or idle lane) waiting on someone, as WaitRow draws it. */
export interface RelayWaitRowView {
  /** The task's id, or the lane's when it has none. */
  id: string;
  /** How long it has waited; null when the daemon did not say (an idle lane). */
  minutes: number | null;
  minutesText: string;
  /** At or past the threshold. */
  over: boolean;
  task: string;
  /** "objective · reason · waiting on party". */
  why: string;
  href: string | null;
  /** The person's nudge to the party waited on; offered once the daemon takes it (#419). */
  nudge: { label: string; canNudge: boolean };
}

/** What Needs you shows; null when there is nothing (the section hides). */
export interface RelayNeedsYouView {
  count: number;
  decisions: RelayDecision[];
  /** The waits at or past the threshold, oldest first. */
  stale: { title: string; rows: RelayWaitRowView[] } | null;
  /** The queue's triage items, by source: the line opens the Queue. */
  triage: { count: number; from: string } | null;
  /** Tasks no lane has taken (the coordinator's to place): read-only. */
  unassigned: { count: number; from: string } | null;
}

/** Why a task waits, and whether it waits on the coordinator or its own lane (§3f-bis). */
function waitReason(lane: RelayLane, now: RelayNow): { reason: string; onLane: boolean } | null {
  const task = lane.task;
  if (lane.lease_state === 'expired' || lane.limit_scope) return null;
  if (!task) return lane.role === 'contributor' && !lane.reviewing ? { reason: 'lane idle, nothing assigned', onLane: false } : null;
  if (task.needs_answer) return null; // a decision: its own card
  if (task.state === 'in_review' && now.floating_reviews.some((r) => r.task_id === task.id)) {
    return { reason: 'report has no reviewer', onLane: false };
  }
  if (task.state === 'rework') return { reason: 'verdict filed, not picked up', onLane: true };
  if (task.state === 'approved') return { reason: 'approved, not merged', onLane: false };
  return null;
}

function waitRow(lane: RelayLane, now: RelayNow, o: NowRowOptions): RelayWaitRowView | null {
  const w = waitReason(lane, now);
  if (!w) return null;
  const objective = lane.title ?? lane.code;
  const party = w.onLane ? `${objective} (${codename(lane)})` : 'the coordinator';
  const task = lane.task;
  const minutes = task ? minutesSince(task.state_changed_at, o.at) : null;
  return {
    id: task?.id ?? lane.id,
    minutes,
    minutesText: minutes == null ? '' : since(task!.state_changed_at, o.at),
    over: minutes != null && minutes >= o.threshold,
    task: task ? `${task.title} (${task.phase} r${task.round})` : 'Nothing assigned',
    why: `${objective} · ${w.reason} · waiting on ${party}`,
    href: task ? o.href(lane.crew_id, task.id) : null,
    nudge: { label: `Nudge ${w.onLane ? codename(lane) : 'the coordinator'}`, canNudge: false },
  };
}

/** Every wait, oldest first; one with no time (an idle lane) after them. */
export function toWaitRows(now: RelayNow | null, o: NowRowOptions): RelayWaitRowView[] {
  if (!now) return [];
  return now.lanes
    .map((l) => waitRow(l, now, o))
    .filter((r): r is RelayWaitRowView => r !== null)
    .sort((a, b) => (b.minutes ?? -1) - (a.minutes ?? -1));
}

/** "From Jira sprint 47, GitHub milestone 11, and an open item". */
function fromLine(sources: Array<{ provider: string; sprint: string | null; count: number }>): string {
  const named: string[] = [];
  let manual = 0;
  for (const s of sources) {
    if (s.provider === 'manual') {
      manual += s.count;
      continue;
    }
    const label = `${providerLabel(s.provider as RelayTicketProvider)}${s.sprint ? ` ${s.sprint}` : ''}`;
    if (!named.includes(label)) named.push(label);
  }
  if (manual > 0) named.push(manual === 1 ? 'an open item' : `${manual} open items`);
  if (named.length <= 1) return `From ${named[0] ?? 'the queue'}`;
  if (named.length === 2) return `From ${named[0]} and ${named[1]}`;
  return `From ${named.slice(0, -1).join(', ')}, and ${named[named.length - 1]}`;
}

/** What Needs you shows, or null when nothing needs the person. */
export function toNeedsYou(now: RelayNow | null, queue: RelayQueue | null, o: NowRowOptions): RelayNeedsYouView | null {
  if (!now) return null;
  const staleRows = toWaitRows(now, o).filter((r) => r.over);
  const stale = staleRows.length
    ? {
        title: `${staleRows.length} ${staleRows.length === 1 ? 'task has' : 'tasks have'} waited ${o.threshold} min or more — oldest first`,
        rows: staleRows,
      }
    : null;
  const triageItems = queue?.triage ?? [];
  const triage = triageItems.length
    ? {
        count: triageItems.length,
        from: fromLine(
          triageItems.map((t) => (t.ticket ? { provider: t.ticket.provider, sprint: t.sprint, count: 1 } : { provider: 'manual', sprint: null, count: 1 })),
        ),
      }
    : null;
  const unassignedCount = now.triage.reduce((n, t) => n + t.count, 0);
  const unassigned = unassignedCount ? { count: unassignedCount, from: fromLine(now.triage) } : null;
  const count = now.decisions.length + (stale ? 1 : 0) + (triage ? 1 : 0) + (unassigned ? 1 : 0);
  if (count === 0) return null;
  return { count, decisions: now.decisions, stale, triage, unassigned };
}
