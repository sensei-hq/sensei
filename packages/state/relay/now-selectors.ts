// The Now view's selectors (#422; CONTRACT §1c-2, §1c-3, §1h; COMPONENTS §3b): from the
// read to the rows LaneList and CompletedList draw. Pure — the read, a clock, the
// threshold and how a task is linked go in; row data comes out. A codename, a time or
// a resume the daemon did not give is not invented (D83).

import { crewGroups, crewRepoLabel, hhmm, since } from './format.js';
import { minutesSince } from './relay-selectors.js';
import type { RelayCompletedTask, RelayLane, RelayNow, RelayRole } from './types.js';

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
    const when = lane.limit_resumes_at ? `until ${hhmm(lane.limit_resumes_at)}` : 'resume not reported';
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
