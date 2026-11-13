// The Now view's selectors (#422; CONTRACT §1c-2, §1c-3, §1h; COMPONENTS §3b): from the
// read to the rows LaneList and CompletedList draw. Pure: the read and a clock in, row
// data out. A number or name the daemon did not give is not invented (D83).
import { describe, it, expect } from 'vitest';
import { toCompletedRows, toLaneGroups, assistantLabel, type NowRowOptions } from './now-selectors.js';
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
    });
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

  it('paused on a limit: until when it resumes, or that it is not known', () => {
    const resumes = F.minutesAgo(-30);
    expect(one(F.lane({ limit_scope: 'daily', limit_resumes_at: resumes })).state).toEqual({ label: 'paused', tone: 'warning' });
    expect(one(F.lane({ limit_scope: 'daily', limit_resumes_at: resumes })).when.text).toBe(`until ${hhmm(resumes)}`);
    expect(one(F.lane({ limit_scope: 'weekly', limit_resumes_at: null })).when.text).toBe('resume not reported');
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
