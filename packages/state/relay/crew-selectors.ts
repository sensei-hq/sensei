// The Crew view's selectors (#424; CONTRACT §1h; COMPONENTS §3e): the run card, the
// members grouped by role with their presence, and AddMember's setup or add. Pure. What
// the daemon does not report — a heartbeat, the editor a session runs in, the catalogue
// of assistants seen on this machine — is not invented (D83).

import { crewTail, hhmm, taskHref, taskStateLabel } from './format.js';
import { taskStateTone } from './task-selectors.js';
import { assistantLabel, type RelayTone } from './now-selectors.js';
import type { RelayLane, RelayNow, RelayPresence, RelayRole, RelayRunDetail, RelayRunList, RelayRunListRow } from './types.js';

export type { RelayPresence };

/** The run, or the crew before its first run (COMPONENTS §3e RunCard). */
export interface RelayRunCardView {
  title: string;
  state: { label: string; tone: RelayTone };
  meta: string;
  facts: Array<{ label: string; value: number }>;
  /** A draft waiting to start (#419): Start (when it can) and Cancel. Null for any other card. */
  draft: { runId: string; canStart: boolean; note: string | null } | null;
}

/** One ended run under the Crew view (CONTRACT §1i; the UI half of #393). */
export interface RelayPastRunView {
  id: string;
  title: string;
  name: string;
  state: { label: string; tone: RelayTone };
  span: string;
  done: string;
  members: string;
  /** Deleted 30 days after it ended; three days or fewer reads as a warning. */
  deletes: { text: string; warning: boolean };
  /** Where the row opens the run, read-only; null without a base. */
  href: string | null;
}

/** A past run, read-only (CONTRACT §1i). */
export interface RelayArchivedRunView {
  title: string;
  name: string;
  state: { label: string; tone: RelayTone };
  banner: { text: string; warning: boolean };
  facts: Array<{ label: string; value: number }>;
  members: Array<{ id: string; codename: string; laneCode: string; role: RelayRole; family: string; model: string }>;
  tasks: Array<{ id: string; title: string; state: { label: string; tone: RelayTone }; by: string; href: string | null }>;
}

/** One member (COMPONENTS §3e MemberCard). */
export interface RelayMemberView {
  id: string;
  codename: string;
  /** "objective · code". */
  lane: string;
  laneCode: string;
  role: RelayRole;
  status: RelayPresence;
  statusText: string;
  family: string;
  model: string;
  /** The not-responding line under the card. */
  alert: string | null;
  /** The host tag: the editor the assistant runs in. Null while the daemon reports none (#396: assistant vs editor). */
  editor: { label: string; title: string } | null;
}

export interface RelayMemberGroupView {
  label: string;
  members: RelayMemberView[];
}

/** AddMember (COMPONENTS §3e): setting the crew up, or adding to it. */
export interface RelayAddMemberView {
  mode: 'setup' | 'add';
  title: string;
  lead: string;
  role: RelayRole;
  /** The coordinator is taken: contributors only. */
  roleLocked: boolean;
  command: { step: string; text: string; note: string };
  listening: string;
}

const codename = (lane: RelayLane) => lane.suggested_name ?? lane.code;
const repoName = (repo: string | null) => (repo ? (repo.split('/').pop() ?? repo) : 'no repository');

/** The run card: the open run when there is one; else a draft waiting to start; else the
 *  crew itself. Null without a crew and without a draft. */
export function toRunCard(now: RelayNow | null, runs: RelayRunList | null = null): RelayRunCardView | null {
  const crew = now?.crews[0];
  const open = crew ? now?.runs.find((r) => r.crew_id === crew.id && (r.state === 'planning' || r.state === 'active')) : undefined;
  const draft = open ? undefined : runs?.active.find((r) => r.state === 'draft');
  if (draft) return draftCard(now, draft);
  if (!now || !crew) return null;
  const coordinator = now.lanes.find((l) => l.crew_id === crew.id && l.role === 'coordinator');
  const members = now.lanes.filter((l) => l.crew_id === crew.id).length;
  const who = coordinator ? ` · coordinator ${codename(coordinator)}` : '';
  const run = now.runs.find((r) => r.crew_id === crew.id && (r.state === 'planning' || r.state === 'active'));
  if (!run) {
    return {
      title: `Crew ${crewTail(crew.id)}`,
      state: { label: 'no run yet', tone: 'neutral' },
      meta: `${repoName(crew.repo_key)} · started ${hhmm(crew.started_at)}${who}`,
      facts: [{ label: 'Members', value: members }],
      draft: null,
    };
  }
  const started = run.started_at ?? run.created_at;
  return {
    title: `Run ${run.run_id.slice(-4)}`,
    state: run.state === 'active' ? { label: 'running', tone: 'success' } : { label: 'planning', tone: 'accent' },
    meta: `${repoName(crew.repo_key)} · started ${hhmm(started)}${who} · completes, and is archived, when its last assigned task is done`,
    facts: [
      { label: 'Members', value: members },
      { label: 'Tasks', value: run.total_tasks },
      { label: 'Done', value: run.done_tasks },
    ],
    draft: null,
  };
}

/** A draft: what it is, and whether the crew can start it now — a member must be answering. */
function draftCard(now: RelayNow | null, draft: RelayRunListRow): RelayRunCardView {
  const crew = now?.crews[0];
  const lanes = crew ? (now?.lanes ?? []).filter((l) => l.crew_id === crew.id) : [];
  const answering = lanes.some((l) => l.presence !== 'not_responding');
  const note = !crew ? 'Set up your crew to start it.' : answering ? null : 'Start it once a member is answering.';
  return {
    title: `Run ${draft.id.slice(-4)}`,
    state: { label: 'draft', tone: 'neutral' },
    meta: `${repoName(draft.repo_key)} · drafted ${hhmm(draft.created_at)} · ${draft.title} · starts on ${crew ? 'this crew' : 'your crew'}`,
    facts: crew ? [{ label: 'Members', value: lanes.length }] : [],
    draft: { runId: draft.id, canStart: Boolean(crew) && answering, note },
  };
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
/** "Nov 2", by the reader's calendar. */
const monthDay = (iso: string) => {
  const d = new Date(iso);
  return `${MONTHS[d.getMonth()]} ${d.getDate()}`;
};
/** Whole calendar days from `at` to `iso`, by the reader's calendar. */
const daysUntil = (iso: string, at: number) => {
  const day = (t: number) => {
    const d = new Date(t);
    return Date.UTC(d.getFullYear(), d.getMonth(), d.getDate());
  };
  return Math.round((day(Date.parse(iso)) - day(at)) / 86_400_000);
};
const plural = (n: number, one: string) => `${n} ${one}${n === 1 ? '' : 's'}`;

/** Ended runs, newest first (CONTRACT §1i): id and span · tasks done · members · when deleted. */
export function toPastRuns(runs: RelayRunList | null, at = Date.now(), base: string | null = null): RelayPastRunView[] {
  if (!runs) return [];
  return [...runs.archived]
    .sort((a, b) => (b.completed_at ?? b.created_at).localeCompare(a.completed_at ?? a.created_at))
    .map((run) => {
      const from = monthDay(run.started_at ?? run.created_at);
      const to = run.completed_at ? monthDay(run.completed_at) : from;
      const left = run.delete_after ? daysUntil(run.delete_after, at) : null;
      return {
        href: base ? runHref(base, run.id) : null,
        id: run.id,
        title: `Run ${run.id.slice(-4)}`,
        name: run.title,
        state: runState(run.state),
        span: from === to ? from : `${from} – ${to}`,
        done: `${run.done_tasks} of ${plural(run.total_tasks, 'task')} done`,
        members: plural(run.members, 'member'),
        deletes: deletion(left),
      };
    });
}

/** Where a run opens, read-only, under a relay base. */
export const runHref = (base: string, runId: string): string => `${base}/runs/${encodeURIComponent(runId)}`;

const deletion = (left: number | null) =>
  left === null
    ? { text: 'deletion date not reported', warning: false }
    : left <= 0
      ? { text: 'deleted today', warning: true }
      : { text: `deleted in ${plural(left, 'day')}`, warning: left <= 3 };

const runState = (state: string): { label: string; tone: RelayTone } =>
  state === 'completed' ? { label: 'completed', tone: 'success' } : { label: state, tone: 'neutral' };

/** A past run, read-only (CONTRACT §1i): the banner (archived when its last task was done;
 *  deleted 30 days on, by a daily job), its facts, who was on it, and what it did — done
 *  first, each task opening its task view. */
export function toArchivedRun(detail: RelayRunDetail, opts: { base: string; at?: number }): RelayArchivedRunView {
  const { run } = detail;
  const at = opts.at ?? Date.now();
  const left = run.delete_after ? daysUntil(run.delete_after, at) : null;
  const when = run.delete_after ? (left !== null && left <= 0 ? 'today' : `in ${plural(left ?? 0, 'day')}`) : 'date not reported';
  const ended = run.completed_at ? monthDay(run.completed_at) : 'on a day not reported';
  const text =
    run.state === 'cancelled'
      ? `Cancelled ${ended}. Deleted 30 days after it ended, by a daily job: ${when}.`
      : `Archived when its last task was done, ${ended}. Deleted 30 days after it completed, by a daily job: ${when}.`;
  const order = (state: string) => (state === 'done' ? 0 : 1);
  const tasks = Object.entries(detail.tasks_by_state)
    .sort(([a], [b]) => order(a) - order(b))
    .flatMap(([, list]) => list)
    .map((t) => ({
      id: t.id,
      title: t.title,
      state: { label: taskStateLabel(t.state), tone: taskStateTone(t.state) },
      by: t.lane ? (t.lane.suggested_name ?? t.lane.code) : 'on no lane',
      href: run.crew_id ? taskHref(opts.base, run.crew_id, t.id) : null,
    }));
  return {
    title: `Run ${run.id.slice(-4)}`,
    name: run.title,
    state: runState(run.state),
    banner: { text, warning: left !== null && left <= 3 },
    facts: [
      { label: 'Tasks', value: tasks.length },
      { label: 'Done', value: tasks.filter((t) => t.state.label === 'done').length },
      { label: 'Members', value: detail.members.length },
    ],
    members: detail.members.map((m) => ({
      id: m.id,
      codename: m.suggested_name ?? m.code,
      laneCode: m.code,
      role: m.role,
      family: assistantLabel(m.family, null),
      model: m.model ?? 'model not reported',
    })),
    tasks,
  };
}

const STATUS_TEXT: Record<RelayPresence, string> = { active: 'Active', idle: 'Idle', not_responding: 'Not responding' };

function member(lane: RelayLane): RelayMemberView {
  const status = lane.presence;
  return {
    id: lane.id,
    codename: codename(lane),
    lane: `${lane.title ?? lane.code} · ${lane.code}`,
    laneCode: lane.code,
    role: lane.role,
    status,
    statusText: STATUS_TEXT[status],
    family: assistantLabel(lane.family, lane.version),
    model: lane.model ?? 'model not reported',
    alert:
      status === 'not_responding'
        ? `No answer since ${lane.lease_expires_at ? hhmm(lane.lease_expires_at) : 'a time not reported'} · lease lost.`
        : null,
    // The editor the session said it runs in (#396), named as people name it; none said, none shown.
    editor: lane.host ? editorTag(assistantLabel(lane.host, null), lane.model ?? 'model not reported') : null,
  };
}

/** The host tag (COMPONENTS §3e): "in <editor>", its title saying what the editor is and
 *  whose transcripts the work counts for. */
export function editorTag(editor: string, model: string): { label: string; title: string } {
  return {
    label: `in ${editor}`,
    title: `${editor} is the editor; the assistant inside it does the work, and its transcripts count for ${model}.`,
  };
}

/** The coordinator, then the contributors with their count. */
export function toMemberGroups(now: RelayNow | null, _at: number): RelayMemberGroupView[] {
  if (!now) return [];
  const coordinators = now.lanes.filter((l) => l.role === 'coordinator').map(member);
  const contributors = now.lanes.filter((l) => l.role === 'contributor').map(member);
  const groups: RelayMemberGroupView[] = [];
  if (coordinators.length) groups.push({ label: 'Coordinator', members: coordinators });
  if (contributors.length) groups.push({ label: `Contributors · ${contributors.length}`, members: contributors });
  return groups;
}

const SETUP_LEAD =
  'Relay runs one crew at a time. Two sessions are enough: one coordinator, then one or more contributors. Each registers with a single line, gets a codename, and the run starts with the second.';

/** AddMember: set the crew up (no run and fewer than two members), or add to it. */
export function toAddMember(now: RelayNow | null, picked: RelayRole): RelayAddMemberView {
  const lanes = now?.lanes ?? [];
  const hasCoordinator = lanes.some((l) => l.role === 'coordinator');
  const hasRun = (now?.runs ?? []).some((r) => r.state === 'planning' || r.state === 'active');
  const setup = !hasRun && lanes.length < 2;
  const role: RelayRole = hasCoordinator ? 'contributor' : picked;
  // The crew named (#419), so the line works with several crews open; with no crew
  // the coordinator's register mints one.
  const crewId = now?.crews[0]?.id ?? null;
  const line = crewId ? `register with sensei as ${role} in crew ${crewId}` : `register with sensei as ${role}`;
  const command = {
    step: setup ? (role === 'coordinator' ? 'Step 1 of 2.' : 'Step 2 of 2.') : '',
    text: line,
    note: `Paste it into an assistant session open in this workspace's repository. It registers, gets a codename, and shows above.`,
  };
  if (setup) {
    return {
      mode: 'setup',
      title: 'Start a run: set up your crew',
      lead: SETUP_LEAD,
      role,
      roleLocked: hasCoordinator,
      command,
      listening: `Listening · ${lanes.length} of 2 to start a run`,
    };
  }
  return {
    mode: 'add',
    title: 'Add a member',
    lead: 'Another session joins as a contributor and takes the next piece of work.',
    role,
    roleLocked: hasCoordinator,
    command,
    listening: 'Listening · a new member shows above as it registers',
  };
}
