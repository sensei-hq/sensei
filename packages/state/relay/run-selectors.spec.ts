// Now's lanes grouped by run, and by agent (#422): pure selectors from the read to the
// groups the Lanes section draws. Waiting tasks in priority order, done ones in the order
// they ran, each attributed to the lane and model that did it and the lane that reviewed it.
import { describe, it, expect } from 'vitest';
import { toAgentGroups, toRunGroups } from './run-selectors.js';
import { hhmm } from './format.js';
import * as F from './fixtures.js';
import type { NowRowOptions } from './now-selectors.js';

const opts: NowRowOptions = {
  at: Date.now(),
  threshold: 30,
  href: (crew, task) => `/relay/${crew}/${task}`,
  byCrew: false,
};
const titles = (rows: { title: string }[]) => rows.map((r) => r.title);

describe('toRunGroups · one group per open run', () => {
  it('the run is headed by its summary (title, state, progress: the band)', () => {
    const [crew] = toRunGroups(F.runNow(), opts);
    expect(crew.label).toBeNull();
    expect(crew.runs).toHaveLength(1);
    expect(crew.runs[0].id).toBe(F.RUN_ID);
    expect(crew.runs[0].run).toMatchObject({ run_id: F.RUN_ID, title: 'Sprint 14: Broker hardening', done_tasks: 2, total_tasks: 6 });
  });

  it('waiting tasks in priority order: Task 0 first, then backlog position, then the plan', () => {
    const now = F.runNow();
    now.run_tasks = [
      F.runTask({ task_id: 'w-unranked', title: 'Unranked' }),
      F.runTask({ task_id: 'w-3', title: 'Third', position: 3 }),
      F.runTask({ task_id: 'w-plan', title: 'Plan', phase: 'planning' }),
      F.runTask({ task_id: 'w-1', title: 'First', position: 1 }),
      F.runTask({ task_id: 'w-1b', title: 'First too', position: 1 }),
    ];
    const run = toRunGroups(now, opts)[0].runs[0];
    expect(titles(run.waiting)).toEqual(['Plan', 'First', 'First too', 'Third', 'Unranked']);
    expect(run.waiting.map((r) => r.priority)).toEqual(['Task 0', '#1 in the backlog', '#1 in the backlog', '#3 in the backlog', null]);
    expect(run.waiting.every((r) => r.who === null)).toBe(true);
  });

  it('a held task names its lane — codename, assistant, model — and its step', () => {
    const run = toRunGroups(F.runNow(), opts)[0].runs[0];
    expect(titles(run.active)).toEqual(['Structure view on the shell']);
    const held = run.active[0];
    expect(held.who).toBe('cobalt-lyra · Claude Code 2.1 · claude-opus-5-5');
    expect(held.detail).toBe('Writing the failing specs');
    expect(held.state).toEqual({ label: 'working', tone: 'accent' });
    expect(held.href).toBe(`/relay/${F.CREW_ID}/t-structure`);
  });

  it('done tasks in the order they ran, each naming who did it and who reviewed it', () => {
    const run = toRunGroups(F.runNow(), opts)[0].runs[0];
    expect(titles(run.done)).toEqual(['Plan: Sprint 14', 'Sidebar on the shell', 'Header on the shell']);
    const [plan, b, a] = run.done;
    expect(a.who).toBe('cobalt-lyra · Claude Code 2.1 · claude-opus-5-5 — reviewed by coral-spica');
    expect(a.detail).toBe(`${hhmm('2026-11-16T10:00:00Z')} → ${hhmm('2026-11-16T10:30:00Z')}`);
    // A model nobody reported says so (D83).
    expect(b.who).toBe('coral-spica · Claude Code 2.1 · model not reported — reviewed by cobalt-lyra');
    // No review yet: no reviewer.
    expect(plan.who).toBe('atlas-rigel · Claude Code 2.1 · claude-opus-5-5');
  });

  it('a lane with no codename reads its code', () => {
    const now = F.runNow();
    now.run_tasks = [F.runTask({ state: 'working', lane: F.runTaskLane({ suggested_name: null }) })];
    expect(toRunGroups(now, opts)[0].runs[0].active[0].who).toBe('L1 · Claude Code 2.1 · claude-opus-5-5');
  });

  it('lanes on no run task are listed after the runs, as their rows', () => {
    const [crew] = toRunGroups(F.runNow(), opts);
    expect(crew.idle.map((r) => r.code)).toEqual(['L0']);
  });

  it('a crew with no open run lists its lanes alone, with no run group', () => {
    const [crew] = toRunGroups(F.now({ lanes: [F.coordinatorLane(), F.lane()] }), opts);
    expect(crew.runs).toEqual([]);
    expect(crew.idle.map((r) => r.code)).toEqual(['L0', 'L1']);
  });

  it('the main window: each crew is an outer group under its repository, its runs inside', () => {
    const now = F.mainNow();
    now.runs = [F.sprint()];
    now.run_tasks = [F.runTask({ task_id: 't-x', title: 'Only on the workspace crew' })];
    const crews = toRunGroups(now, { ...opts, byCrew: true });
    expect(crews.map((c) => c.id)).toEqual([F.CREW_ID, F.LOOSE_CREW_ID]);
    expect(crews[0].label).not.toBeNull();
    expect(crews[0].runs.map((r) => r.id)).toEqual([F.RUN_ID]);
    expect(crews[1].runs).toEqual([]);
    expect(crews[1].idle.map((r) => r.code)).toEqual(['L8', 'L9']);
  });

  it('a stat filter keeps the held tasks whose lane it keeps, and drops waiting and done', () => {
    const run = toRunGroups(F.runNow(), { ...opts, only: 'working' })[0].runs[0];
    expect(titles(run.active)).toEqual(['Structure view on the shell']);
    expect(run.waiting).toEqual([]);
    expect(run.done).toEqual([]);
  });

  it('no read, no groups', () => {
    expect(toRunGroups(null, opts)).toEqual([]);
  });
});

describe('toAgentGroups · one group per lane', () => {
  it('each lane: its codename, assistant and model, its state, then the run tasks it ran in run order and what it holds', () => {
    const [crew] = toAgentGroups(F.runNow(), opts);
    expect(crew.agents.map((a) => a.code)).toEqual(['L0', 'L1']);
    const [l0, l1] = crew.agents;
    expect(l0.name).toBe('atlas-rigel');
    expect(l0.who).toBe('Claude Code 2.1 · claude-opus-5-5');
    expect(titles(l0.tasks)).toEqual(['Plan: Sprint 14']);
    expect(l1.name).toBe('cobalt-lyra');
    expect(l1.state.label).toBe('working');
    expect(titles(l1.tasks)).toEqual(['Header on the shell', 'Structure view on the shell']);
  });

  it('a lane that ran nothing in a run has no tasks', () => {
    const now = F.runNow();
    now.run_tasks = [];
    expect(toAgentGroups(now, opts)[0].agents.every((a) => a.tasks.length === 0)).toBe(true);
  });

  it('a lane reporting no model says so', () => {
    const now = F.runNow();
    now.lanes = [F.lane({ model: null, suggested_name: null })];
    expect(toAgentGroups(now, opts)[0].agents[0]).toMatchObject({ name: 'L1', who: 'Claude Code 2.1 · model not reported' });
  });
});
