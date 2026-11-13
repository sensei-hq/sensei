// The Now view's selectors (#422; CONTRACT §1c-2, §1c-3, §1h; COMPONENTS §3b): from the
// read to the rows LaneList and CompletedList draw. Pure: the read and a clock in, row
// data out. A number or name the daemon did not give is not invented (D83).
import { describe, it, expect } from 'vitest';
import { toCompletedRows, toLaneGroups, toNeedsYou, toWaitRows, assistantLabel, type NowRowOptions } from './now-selectors.js';
import { hhmm } from './format.js';
import * as F from './fixtures.js';

const opts = (over: Partial<NowRowOptions> = {}): NowRowOptions => ({
  at: Date.now(),
  threshold: 15,
  href: (crew, task) => `/relay/${crew}/${task}`,
  byCrew: false,
  ...over,
});

const rowsOf = (now = F.now(), o = opts()) => toLaneGroups(now, o).flatMap((g) => g.rows);
const one = (lane: ReturnType<typeof F.lane>, extra: Partial<ReturnType<typeof F.now>> = {}, o = opts()) =>
  rowsOf(F.now({ lanes: [lane], ...extra }), o)[0];

describe('toLaneGroups · a lane row (CONTRACT §1c-2)', () => {
  it('on a task: objective, codename · assistant, the task with its phase and round, working, time in state, a link', () => {
    const row = one(F.lane({ suggested_name: 'cobalt-lyra' }));
    expect(row).toEqual({
      id: F.LANE.L1,
      code: 'L1',
      role: 'contributor',
      name: 'Diagrams',
      who: 'cobalt-lyra · Claude Code 2.1',
      task: 'Structure view on the shell (tests r1)',
      state: { label: 'working', tone: 'accent' },
      when: { text: '38 min', over: false },
      href: `/relay/${F.CREW_ID}/t-structure`,
      progress: { text: '2 of 5 steps done · Writing the failing specs', done: 2, total: 5 },
    });
  });

  it('step progress says only what the holder reported (#418): counts, the step, both, or nothing', () => {
    const at = (current_step: string | null, steps_done: number, steps_total: number) =>
      one(F.lane({ task: { ...F.lane().task!, current_step, steps_done, steps_total } })).progress;
    expect(at('Writing the failing specs', 2, 5)).toEqual({ text: '2 of 5 steps done · Writing the failing specs', done: 2, total: 5 });
    expect(at(null, 5, 5)).toEqual({ text: '5 of 5 steps done', done: 5, total: 5 });
    expect(at('Reading the brief', 0, 0)).toEqual({ text: 'Reading the brief', done: 0, total: 0 });
    // Nothing reported is no line — never "0 of 0".
    expect(at(null, 0, 0)).toBeNull();
    expect(one(F.lane({ task: null })).progress).toBeNull();
  });

  it('no codename yet: the lane code stands in, never an invented name (#395)', () => {
    expect(one(F.lane()).who).toBe('L1 · Claude Code 2.1');
    expect(one(F.lane({ version: null, family: 'cursor' })).who).toBe('L1 · Cursor');
    // A family the screen has no name for reads as the daemon wrote it, made readable.
    expect(assistantLabel('gemini-cli', null)).toBe('Gemini CLI');
    expect(assistantLabel('some-new-tool', '3')).toBe('Some new tool 3');
  });

  it('the lane with no objective yet is named by its code', () => {
    expect(one(F.lane({ title: null })).name).toBe('L1');
  });

  it('idle: nothing assigned, no time, no link', () => {
    const row = one(F.lane({ task: null }));
    expect(row.task).toBe('Nothing assigned');
    expect(row.state).toEqual({ label: 'idle', tone: 'neutral' });
    expect(row.when).toEqual({ text: '', over: false });
    expect(row.href).toBeNull();
  });

  it('holding a review and no task of its own: the review is its one task', () => {
    const row = one(
      F.lane({ task: null, reviewing: { id: 't-notice', title: 'Notice polish', phase: 'impl', round: 1, claimed_at: F.minutesAgo(24) } }),
    );
    expect(row.task).toBe('Review · Notice polish (impl r1)');
    expect(row.state).toEqual({ label: 'reviewing', tone: 'accent' });
    expect(row.when).toEqual({ text: '24 min', over: false });
    expect(row.href).toBe(`/relay/${F.CREW_ID}/t-notice`);
  });

  it('waiting on someone else: amber past the threshold (>=), whatever it waits for', () => {
    const inReview = (minutes: number, floating: boolean) =>
      one(
        F.lane({ task: { ...F.lane().task!, state: 'in_review', state_changed_at: F.minutesAgo(minutes) } }),
        floating ? { floating_reviews: [F.floatingReview({ task_id: 't-structure' })] } : {},
      );
    expect(inReview(26, true).state).toEqual({ label: 'waiting · no reviewer', tone: 'warning' });
    expect(inReview(26, true).when).toEqual({ text: '26 min', over: true });
    expect(inReview(15, true).when.over).toBe(true);
    expect(inReview(14, true).when.over).toBe(false);
    expect(inReview(20, false).state).toEqual({ label: 'in review', tone: 'warning' });
    const inState = (state: 'rework' | 'approved') =>
      one(F.lane({ task: { ...F.lane().task!, state, state_changed_at: F.minutesAgo(41) } }));
    expect(inState('rework').state).toEqual({ label: 'verdict to pick up', tone: 'warning' });
    expect(inState('approved').state).toEqual({ label: 'approved, not merged', tone: 'warning' });
    expect(inState('approved').when.over).toBe(true);
  });

  it('a question for the person: needs your answer, never amber (it waits on you, not on a lane)', () => {
    const row = one(F.lane({ task: { ...F.lane().task!, needs_answer: true, state_changed_at: F.minutesAgo(55) } }));
    expect(row.state).toEqual({ label: 'needs your answer', tone: 'warning' });
    expect(row.when).toEqual({ text: '55 min', over: false });
  });

  it('lease lost: danger, and the time is when it was lost', () => {
    const lost = F.minutesAgo(80);
    const row = one(F.lane({ lease_state: 'expired', lease_expires_at: lost }));
    expect(row.state).toEqual({ label: 'lease lost', tone: 'danger' });
    expect(row.when).toEqual({ text: `at ${hhmm(lost)}`, over: false });
    expect(row.href).toBe(`/relay/${F.CREW_ID}/t-structure`);
  });

  it('paused on a limit: which quota, and until when it resumes or that that is unknown (the limit-exhaustion wording)', () => {
    const resumes = F.minutesAgo(-30);
    expect(one(F.lane({ limit_scope: 'daily', limit_resumes_at: resumes })).state).toEqual({ label: 'paused', tone: 'warning' });
    expect(one(F.lane({ limit_scope: 'daily', limit_resumes_at: resumes })).when.text).toBe(`daily · until ${hhmm(resumes)}`);
    expect(one(F.lane({ limit_scope: 'weekly', limit_resumes_at: null })).when.text).toBe('weekly · resume unknown');
  });
});

describe('toLaneGroups · groups', () => {
  it('the workspace window: one group, no heading, every lane in the read’s order', () => {
    const groups = toLaneGroups(F.now(), opts());
    expect(groups).toHaveLength(1);
    expect(groups[0].label).toBeNull();
    expect(groups[0].rows.map((r) => r.code)).toEqual(['L0', 'L1']);
  });

  it('the main window: a group per crew that has lanes, headed by its repository', () => {
    const groups = toLaneGroups(F.mainNow(), opts({ byCrew: true }));
    expect(groups.map((g) => g.id)).toEqual([F.CREW_ID, F.LOOSE_CREW_ID]);
    expect(groups.map((g) => g.label)).toEqual([F.REPO, F.LOOSE_REPO]);
    expect(groups[1].rows.map((r) => r.code)).toEqual(['L8', 'L9']);
  });

  it('no read, no groups', () => {
    expect(toLaneGroups(null, opts())).toEqual([]);
  });
});

describe('toCompletedRows · Completed today (CONTRACT §1c-3)', () => {
  it('time · title · objective · codename · outcome, each a link to its task', () => {
    const now = F.now({
      lanes: [F.lane({ suggested_name: 'cobalt-lyra' })],
      completed_today: [F.completed({ lane_code: 'L1', lane_title: 'Diagrams', outcome: 'merged' })],
    });
    const [row] = toCompletedRows(now, opts());
    const c = now.completed_today[0];
    expect(row).toEqual({
      id: c.task_id,
      at: hhmm(c.completed_at),
      title: c.title,
      lane: 'Diagrams · cobalt-lyra',
      outcome: { label: 'merged', tone: 'success' },
      href: `/relay/${c.crew_id}/${c.task_id}`,
    });
  });

  it('a lane no longer in the read keeps its objective and code; approved and done read so', () => {
    const now = F.now({
      lanes: [],
      completed_today: [
        F.completed({ task_id: 'a', lane_code: 'L4', lane_title: null, outcome: 'approved' }),
        F.completed({ task_id: 'b', lane_code: 'L5', lane_title: 'Docs pages', outcome: 'done' }),
      ],
    });
    expect(toCompletedRows(now, opts()).map((r) => [r.lane, r.outcome.label])).toEqual([
      ['L4', 'approved'],
      ['Docs pages · L5', 'done'],
    ]);
  });

  it('no read, no rows', () => {
    expect(toCompletedRows(null, opts())).toEqual([]);
  });
});

// ─── Waiting and Needs you (CONTRACT §1c-0 Waiting's filter, §1c-1, §3f-bis) ──────────

describe('toWaitRows · who waits, on whom, oldest first', () => {
  const task = F.lane().task!;
  const waits = () =>
    F.now({
      lanes: [
        F.coordinatorLane({ task: null }),
        F.lane({ id: 'l4', code: 'L4', title: 'ER diagrams', suggested_name: 'iris-deneb', task: { ...task, id: 't-er', title: 'ER sidebar on the shell', phase: 'impl', state: 'approved', state_changed_at: F.minutesAgo(52) } }),
        F.lane({ id: 'l5', code: 'L5', title: 'Docs pages', suggested_name: 'coral-spica', task: { ...task, id: 't-libs', title: 'Libraries page', phase: 'docs', round: 4, state: 'rework', state_changed_at: F.minutesAgo(41) } }),
        F.lane({ id: 'l1', code: 'L1', title: 'Diagrams', task: { ...task, state: 'in_review', state_changed_at: F.minutesAgo(26) } }),
        F.lane({ id: 'l6', code: 'L6', title: 'Sync hardening', task: null }),
        F.lane({ id: 'l2', code: 'L2', title: 'Billing', task: { ...task, id: 't-b', state: 'working' } }),
      ],
      floating_reviews: [F.floatingReview({ task_id: 't-structure' })],
    });

  it('a merge pending, a verdict unread, a report with no reviewer, an idle contributor — oldest first, the idle last', () => {
    const rows = toWaitRows(waits(), opts());
    // The coordinator idle is not waiting on itself: only an idle contributor waits for work.
    expect(rows.map((r) => r.id)).toEqual(['t-er', 't-libs', 't-structure', 'l6']);
    expect(rows[0]).toEqual({
      id: 't-er',
      minutes: 52,
      minutesText: '52 min',
      over: true,
      task: 'ER sidebar on the shell (impl r1)',
      why: 'ER diagrams · approved, not merged · waiting on the coordinator',
      href: `/relay/${F.CREW_ID}/t-er`,
      nudge: { label: 'Nudge the coordinator', canNudge: false },
    });
    expect(rows[1].why).toBe('Docs pages · verdict filed, not picked up · waiting on Docs pages (coral-spica)');
    expect(rows[1].nudge.label).toBe('Nudge coral-spica');
    expect(rows[2].why).toBe('Diagrams · report has no reviewer · waiting on the coordinator');
  });

  it('an idle lane waits on the coordinator with no time: the daemon reports no idle-since (D83)', () => {
    const idle = toWaitRows(waits(), opts()).find((r) => r.id === 'l6')!;
    expect(idle).toMatchObject({ minutes: null, minutesText: '', over: false, task: 'Nothing assigned', href: null });
    expect(idle.why).toBe('Sync hardening · lane idle, nothing assigned · waiting on the coordinator');
  });

  it('nobody can nudge from the screen until the daemon takes a person’s nudge (#419)', () => {
    expect(toWaitRows(waits(), opts()).every((r) => r.nudge.canNudge === false)).toBe(true);
  });

  it('a working lane does not wait; no read, no rows', () => {
    expect(toWaitRows(waits(), opts()).some((r) => r.id === 't-b')).toBe(false);
    expect(toWaitRows(null, opts())).toEqual([]);
  });
});

describe('toNeedsYou · decisions, the long waits, triage, the unassigned', () => {
  it('the decisions; the waits at or past the threshold, oldest first, with the count in words', () => {
    const now = F.now({
      decisions: [F.decision()],
      lanes: [F.lane({ task: { ...F.lane().task!, state: 'approved', state_changed_at: F.minutesAgo(52) } }), F.lane({ id: 'l9', code: 'L9', task: { ...F.lane().task!, id: 't9', state: 'approved', state_changed_at: F.minutesAgo(5) } })],
      triage: [],
    });
    const n = toNeedsYou(now, F.queue({ triage: [] }), opts());
    expect(n?.decisions.map((d) => d.id)).toEqual([now.decisions[0].id]);
    expect(n?.stale?.title).toBe('1 task has waited 15 min or more — oldest first');
    expect(n?.stale?.rows.map((r) => r.id)).toEqual(['t-structure']);
    expect(n?.triage).toBeNull();
    expect(n?.count).toBe(2);
  });

  it('triage counts the queue’s triage items, by where they came from, and opens the Queue (the #379 brief)', () => {
    const n = toNeedsYou(F.now({ triage: [] }), F.queue(), opts());
    expect(n?.triage).toEqual({ count: 2, from: 'From Jira sprint 47 and an open item' });
    expect(n?.count).toBe(1);
  });

  it('tasks no lane has taken are their own read-only row, apart from triage', () => {
    const now = F.now({ triage: [{ provider: 'github', sprint: 'milestone 11', count: 3 }, { provider: 'manual', sprint: null, count: 1 }] });
    const n = toNeedsYou(now, F.queue({ triage: [] }), opts());
    expect(n?.unassigned).toEqual({ count: 4, from: 'From GitHub milestone 11 and an open item' });
    expect(n?.triage).toBeNull();
  });

  it('nothing to do is no section (null); no read is null too', () => {
    expect(toNeedsYou(F.now({ triage: [], decisions: [] }), F.queue({ triage: [] }), opts())).toBeNull();
    expect(toNeedsYou(null, null, opts())).toBeNull();
  });
});

describe('toNeedsYou · messages to the person', () => {
  it('a lane’s message to the person needs them too, and counts', () => {
    const n = toNeedsYou(F.now({ triage: [], decisions: [], person_messages: [F.personMessage()] }), F.queue({ triage: [] }), opts());
    expect(n?.messages.map((m) => m.id)).toEqual([F.personMessage().id]);
    expect(n?.count).toBe(1);
  });
});

describe('toLaneGroups · a stat’s filter narrows the lanes (CONTRACT §1c-0)', () => {
  const now = () =>
    F.now({
      lanes: [
        F.lane(),
        F.lane({ id: 'l2', code: 'L2', lease_state: 'expired', task: { ...F.lane().task!, id: 't2' } }),
        F.lane({ id: 'l3', code: 'L3', task: { ...F.lane().task!, id: 't3', needs_answer: true } }),
        F.lane({ id: 'l4', code: 'L4', task: null }),
      ],
    });
  const codes = (only: NowRowOptions['only']) => toLaneGroups(now(), opts({ only })).flatMap((g) => g.rows.map((r) => r.code));

  it('Working: the lanes on a task or holding a review', () => {
    expect(codes('working')).toEqual(['L1']);
  });

  it('Blocked: a lost lease, or a question for the person', () => {
    expect(codes('blocked')).toEqual(['L2', 'L3']);
  });

  it('no filter: every lane', () => {
    expect(codes(undefined)).toEqual(['L1', 'L2', 'L3', 'L4']);
  });
});
