// Now's Lanes section, grouped by run or by agent (#422): pure selectors from the read to
// the groups the screen draws. Within a run, waiting tasks go in priority order (Task 0,
// then the item's backlog position, then the plan's order), held tasks in the plan's
// order, done tasks in the order they ran; every task names the lane and model that did
// it and, once reviewed, the lane that reviewed it. Nothing the daemon did not report is
// invented (D83).

import { crewGroups, crewRepoLabel, hhmm, taskStateLabel } from './format.js';
import { assistantLabel, codename, laneIn, laneRow, type NowRowOptions, type RelayLaneRowView, type RelayTone } from './now-selectors.js';
import { taskStateTone } from './task-selectors.js';
import type { RelayCrew, RelayLane, RelayNow, RelayRunSummary, RelayNowRunTask, RelayNowRunTaskLane } from './types.js';

/** One task of a run, as its row draws it. */
export interface RelayRunTaskRowView {
  id: string;
  title: string;
  href: string;
  /** Waiting rows: "Task 0", "#1 in the backlog", or null for an unranked item. */
  priority: string | null;
  state: { label: string; tone: RelayTone };
  /** "cobalt-lyra · Claude Code 2.1 · claude-opus-5-5", a done row adding "— reviewed by …"; null while it waits. */
  who: string | null;
  /** A held task: its open step; a done task: "10:00 → 10:30"; else null. */
  detail: string | null;
}

/** One open run: its summary (the band that heads it), then its tasks waiting, in progress and done. */
export interface RelayRunGroupView {
  id: string;
  run: RelayRunSummary;
  waiting: RelayRunTaskRowView[];
  active: RelayRunTaskRowView[];
  done: RelayRunTaskRowView[];
}

/** A crew's runs, then its lanes on no run task (as their rows). */
export interface RelayCrewRunsView {
  id: string;
  /** The crew's repository in the main window; null in the workspace window. */
  label: string | null;
  runs: RelayRunGroupView[];
  idle: RelayLaneRowView[];
}

/** One lane and the run tasks it ran, in the order it ran them, then what it holds. */
export interface RelayAgentGroupView {
  id: string;
  code: string;
  /** The codename, or the code until there is one. */
  name: string;
  /** "Claude Code 2.1 · claude-opus-5-5". */
  who: string;
  /** The lane's state as its row says it (working, idle, lease lost…). */
  state: { label: string; tone: RelayTone };
  tasks: RelayRunTaskRowView[];
}

export interface RelayCrewAgentsView {
  id: string;
  label: string | null;
  agents: RelayAgentGroupView[];
}

const modelOf = (model: string | null) => model ?? 'model not reported';
const whoOf = (l: RelayNowRunTaskLane) => `${codename(l)} · ${assistantLabel(l.family, l.version)} · ${modelOf(l.model)}`;

type Stage = 'waiting' | 'active' | 'done';
const stageOf = (t: RelayNowRunTask): Stage => (t.state === 'done' ? 'done' : t.lane ? 'active' : 'waiting');

/** Priority: Task 0 first, then the backlog position (unranked last); the plan's order breaks ties. */
function byPriority(tasks: RelayNowRunTask[]): RelayNowRunTask[] {
  const rank = (t: RelayNowRunTask) => (t.phase === 'planning' ? -1 : (t.position ?? Number.MAX_SAFE_INTEGER));
  return tasks.map((t, i) => ({ t, i })).sort((a, b) => rank(a.t) - rank(b.t) || a.i - b.i).map(({ t }) => t);
}

/** Run order: when each was claimed, then completed (an unknown time last); the plan's order breaks ties. */
function byRunOrder(tasks: RelayNowRunTask[]): RelayNowRunTask[] {
  const time = (s: string | null) => (s ? new Date(s).getTime() : Number.MAX_SAFE_INTEGER);
  return tasks
    .map((t, i) => ({ t, i }))
    .sort((a, b) => time(a.t.claimed_at) - time(b.t.claimed_at) || time(a.t.completed_at) - time(b.t.completed_at) || a.i - b.i)
    .map(({ t }) => t);
}

function taskRow(t: RelayNowRunTask, o: NowRowOptions): RelayRunTaskRowView {
  const stage = stageOf(t);
  const priority = stage !== 'waiting' ? null : t.phase === 'planning' ? 'Task 0' : t.position != null ? `#${t.position} in the backlog` : null;
  const reviewed = stage === 'done' && t.reviewer ? ` — reviewed by ${codename(t.reviewer)}` : '';
  const detail =
    stage === 'active' ? t.step : stage === 'done' && t.claimed_at && t.completed_at ? `${hhmm(t.claimed_at)} → ${hhmm(t.completed_at)}` : null;
  return {
    id: t.task_id,
    title: t.title,
    href: o.href(t.crew_id, t.task_id),
    priority,
    state: { label: taskStateLabel(t.state), tone: taskStateTone(t.state) },
    who: t.lane ? `${whoOf(t.lane)}${reviewed}` : null,
    detail,
  };
}

function runGroup(run: RelayRunSummary, tasks: RelayNowRunTask[], lanes: RelayLane[], o: NowRowOptions): RelayRunGroupView {
  const mine = tasks.filter((t) => t.run_id === run.run_id);
  const of = (stage: Stage) => mine.filter((t) => stageOf(t) === stage);
  // A stat's filter narrows to held tasks whose lane it keeps (CONTRACT §1c-0).
  const kept = (t: RelayNowRunTask) => {
    const lane = lanes.find((l) => l.id === t.lane?.id);
    return lane ? laneIn(lane, o.only) : false;
  };
  return {
    id: run.run_id,
    run,
    waiting: o.only ? [] : byPriority(of('waiting')).map((t) => taskRow(t, o)),
    active: of('active').filter((t) => !o.only || kept(t)).map((t) => taskRow(t, o)),
    done: o.only ? [] : byRunOrder(of('done')).map((t) => taskRow(t, o)),
  };
}

/** The crews as Now draws them: one in the workspace window, each under its repository in the main window. */
function crewsOf(now: RelayNow, o: NowRowOptions): { crew: RelayCrew | null; lanes: RelayLane[]; label: string | null }[] {
  if (!o.byCrew) return [{ crew: now.crews[0] ?? null, lanes: now.lanes, label: null }];
  return crewGroups(now.crews, now.lanes).map((g) => ({ crew: g.crew, lanes: g.lanes, label: crewRepoLabel(g.crew) }));
}

/** Now's lanes by run (#422): each crew's open runs, then its lanes on no run task. */
export function toRunGroups(now: RelayNow | null, o: NowRowOptions): RelayCrewRunsView[] {
  if (!now) return [];
  return crewsOf(now, o).map(({ crew, lanes, label }) => {
    const crewId = crew?.id ?? null;
    const tasks = now.run_tasks.filter((t) => crewId === null || t.crew_id === crewId);
    const holding = new Set(tasks.filter((t) => stageOf(t) === 'active').map((t) => t.lane!.id));
    return {
      id: crewId ?? 'lanes',
      label,
      runs: now.runs.filter((r) => crewId === null || r.crew_id === crewId).map((r) => runGroup(r, tasks, lanes, o)),
      idle: lanes.filter((l) => !holding.has(l.id) && laneIn(l, o.only)).map((l) => laneRow(l, now, o)),
    };
  });
}

/** Now's lanes by agent (#422): each lane, the run tasks it ran in run order, then what it holds. */
export function toAgentGroups(now: RelayNow | null, o: NowRowOptions): RelayCrewAgentsView[] {
  if (!now) return [];
  return crewsOf(now, o).map(({ crew, lanes, label }) => ({
    id: crew?.id ?? 'lanes',
    label,
    agents: lanes
      .filter((l) => laneIn(l, o.only))
      .map((l) => {
        const mine = now.run_tasks.filter((t) => t.lane?.id === l.id);
        const ran = byRunOrder(mine.filter((t) => stageOf(t) === 'done'));
        const holds = mine.filter((t) => stageOf(t) === 'active');
        return {
          id: l.id,
          code: l.code,
          name: codename(l),
          who: `${assistantLabel(l.family, l.version)} · ${modelOf(l.model)}`,
          state: laneRow(l, now, o).state,
          // The group names the lane; a row adds only who reviewed it.
          tasks: [...ran, ...holds].map((t) => ({
            ...taskRow(t, o),
            who: stageOf(t) === 'done' && t.reviewer ? `reviewed by ${codename(t.reviewer)}` : null,
          })),
        };
      }),
  }));
}
