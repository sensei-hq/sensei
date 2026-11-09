import { describe, it, expect } from 'vitest';
import type { RelayAction, RelayLaneRef, RelayTaskView } from '$lib/relay-api.js';
import { buildTimeline, foldReceipts } from './relay-timeline.js';

// F8: the task timeline — one thin row per broker-known action,
// oldest first, plus the derived terminal row (a cancel writes no
// action). Every kind maps; an unmapped kind is a bug, never a gap.

const LANES: RelayLaneRef[] = [
  { id: 'lane-1', code: 'L1', title: 'Coord' },
  { id: 'lane-2', code: 'L2', title: 'Worker' },
  { id: 'lane-3', code: 'L3', title: 'Reviewer' },
];

let n = 0;
const base = (kind: string, over: Record<string, unknown> = {}) => ({
  id: `act-${kind}-${(n += 1)}`,
  lane_code: 'L2',
  round: 1,
  phase: 'impl',
  body_md: null,
  reply_to: null,
  created_at: `2026-11-06T10:${String(10 + n).padStart(2, '0')}:00Z`,
  ...over,
});

const view = (history: RelayAction[], state = 'working'): RelayTaskView =>
  ({
    task: {
      id: 'task-1',
      crew_id: 'crew-1',
      title: 'T',
      repo_key: 'github.com/o/r',
      workspace_id: null,
      phase: 'impl',
      state,
      round: 1,
      ticket: null,
      lane: null,
      assigned_at: '2026-11-06T10:00:00Z',
      completed_at: null,
      state_changed_at: '2026-11-06T10:09:00Z',
      rounds: [],
      needs_answer: false,
    },
    run: {
      repository: null,
      branch: null,
      commits: null,
      model: null,
      tokens_in: null,
      tokens_out: null,
      tokens_cache: null,
      context_tokens: null,
      compactions: null,
      peak_memory_mb: null,
      subagents: null,
      working_secs: null,
      waiting_secs: null,
      files_changed: null,
      files_outside_owned: null,
      gates_run: null,
      reported_turns: 0,
      turns: 0,
    },
    decision: null,
    handoff: null,
    history,
    lanes: LANES,
  }) as unknown as RelayTaskView;

describe('buildTimeline', () => {
  it('reads oldest first: the lifecycle story, not the latest scan', () => {
    const rows = buildTimeline(
      view([
        { ...base('reported'), kind: 'reported', props: {} } as unknown as RelayAction,
        { ...base('assigned'), kind: 'assigned', props: { docs: [], owned_paths: [], acceptance: [] } } as unknown as RelayAction,
      ]),
    );
    expect(rows.map((r) => r.title)).toEqual(['Assigned to L2', 'Reported — Work · impl r1']);
  });

  it.each([
    ['assigned', { docs: [], owned_paths: [], acceptance: [] }, 'Assigned to L2'],
    ['brief_updated', { docs: [] }, 'Brief updated'],
    ['started', {}, 'Started'],
    ['merged', { commits: [], into: 'develop', gates: [] }, 'Merged into develop'],
    ['interrupted', {}, 'Lease lost'],
    ['compacted', {}, 'Compacted'],
  ] as const)('kind %s maps to %s', (kind, props, title) => {
    const rows = buildTimeline(view([{ ...base(kind), kind, props } as unknown as RelayAction]));
    expect(rows).toHaveLength(1);
    expect(rows[0].title).toBe(title);
  });

  it('a full report names its turn; a partial says so', () => {
    const full = buildTimeline(view([{ ...base('reported'), kind: 'reported', props: {} } as unknown as RelayAction]));
    expect(full[0].title).toBe('Reported — Work · impl r1');
    const partial = buildTimeline(
      view([{ ...base('reported'), kind: 'reported', props: { partial: true } } as unknown as RelayAction]),
    );
    expect(partial[0].title).toBe('Partial report — Work · impl r1');
  });

  it('a rework-round report reads Rework', () => {
    const rows = buildTimeline(
      view([{ ...base('reported'), kind: 'reported', round: 2, props: {} } as unknown as RelayAction]),
    );
    expect(rows[0].title).toBe('Reported — Rework · impl r2');
  });

  it.each([
    ['approved', 'Review approved'],
    ['changes_requested', 'Changes requested'],
  ])('verdict %s reads %s', (verdict, title) => {
    const rows = buildTimeline(
      view([
        {
          ...base('reviewed'),
          kind: 'reviewed',
          props: { verdict, findings: [] },
        } as unknown as RelayAction,
      ]),
    );
    expect(rows[0].title).toBe(title);
  });

  it('a handover names both lanes by code', () => {
    const rows = buildTimeline(
      view([
        {
          ...base('handed_over'),
          kind: 'handed_over',
          props: { from_lane: 'lane-2', from_state: 'working', to_lane: 'lane-3' },
        } as unknown as RelayAction,
      ]),
    );
    expect(rows[0].title).toBe('Handed over L2 → L3');
  });

  it('a re-offer names its lane, or the queue', () => {
    const toLane = buildTimeline(
      view([{ ...base('reoffered'), kind: 'reoffered', props: { to_lane: 'lane-3' } } as unknown as RelayAction]),
    );
    expect(toLane[0].title).toBe('Re-offered to L3');
    const queued = buildTimeline(
      view([{ ...base('reoffered'), kind: 'reoffered', props: { to_lane: null } } as unknown as RelayAction]),
    );
    expect(queued[0].title).toBe('Returned to the queue');
  });

  it('an ask names who; an answer names the option', () => {
    const asked = buildTimeline(
      view([
        {
          ...base('asked'),
          kind: 'asked',
          props: { context: null, options: [], recommended: null, blocks: [], asked_of: 'lane-3' },
        } as unknown as RelayAction,
      ]),
    );
    expect(asked[0].title).toBe('Asked L3');
    const answered = buildTimeline(
      view([
        {
          ...base('answered'),
          kind: 'answered',
          props: { option: 'db', surface: 'desktop' },
        } as unknown as RelayAction,
      ]),
    );
    expect(answered[0].title).toBe('Answered: db');
  });

  it('a message names its target', () => {
    const rows = buildTimeline(
      view([
        {
          ...base('message'),
          kind: 'message',
          props: { to: { kind: 'lane', lane_id: 'lane-3' }, surface: 'lane' },
        } as unknown as RelayAction,
      ]),
    );
    expect(rows[0].title).toBe('Message to L3');
  });

  it('receipts fold away: delivered/read are never rows', () => {
    const rows = buildTimeline(
      view([
        { ...base('read'), kind: 'read', props: {}, reply_to: 'm1' } as unknown as RelayAction,
        { ...base('delivered'), kind: 'delivered', props: {}, reply_to: 'm1' } as unknown as RelayAction,
        {
          ...base('message'),
          id: 'm1',
          kind: 'message',
          props: { to: { kind: 'everyone' }, surface: 'lane' },
        } as unknown as RelayAction,
      ]),
    );
    expect(rows.map((r) => r.title)).toEqual(['Message to everyone']);
  });

  it('a cancelled task gains a derived row stamped from the header', () => {
    const rows = buildTimeline(
      view(
        [{ ...base('assigned'), kind: 'assigned', props: { docs: [], owned_paths: [], acceptance: [] } } as unknown as RelayAction],
        'cancelled',
      ),
    );
    expect(rows.map((r) => r.title)).toEqual(['Assigned to L2', 'Cancelled']);
    expect(rows[1].at).toBe('2026-11-06T10:09:00Z');
    expect(rows[1].action).toBeNull();
  });

  it('a live task gains no derived row', () => {
    const rows = buildTimeline(
      view([{ ...base('assigned'), kind: 'assigned', props: { docs: [], owned_paths: [], acceptance: [] } } as unknown as RelayAction]),
    );
    expect(rows.map((r) => r.title)).toEqual(['Assigned to L2']);
  });

  it('an unknown kind renders honestly, never silently', () => {
    const rows = buildTimeline(view([{ ...base('teleported'), kind: 'teleported', props: {} } as unknown as RelayAction]));
    expect(rows[0].title).toBe('Unknown event (teleported)');
    expect(rows[0].action).not.toBeNull();
  });

  it('the actor reads by code, or you', () => {
    const rows = buildTimeline(
      view([
        { ...base('asked'), kind: 'asked', lane_code: null, props: { context: null, options: [], recommended: null, blocks: [], asked_of: 'person' } } as unknown as RelayAction,
        { ...base('started'), kind: 'started', props: {} } as unknown as RelayAction,
      ]),
    );
    expect(rows.map((r) => r.actor)).toEqual(['L2', 'you']);
  });
});

describe('foldReceipts', () => {
  it('folds delivered/read under their message, read winning', () => {
    expect(
      foldReceipts([
        { id: 'd1', kind: 'delivered', reply_to: 'm1' } as unknown as RelayAction,
        { id: 'r1', kind: 'read', reply_to: 'm1' } as unknown as RelayAction,
        { id: 'd2', kind: 'delivered', reply_to: 'm2' } as unknown as RelayAction,
        { id: 'm1', kind: 'message', reply_to: null } as unknown as RelayAction,
      ]),
    ).toEqual({ m1: 'read', m2: 'delivered' });
  });
});
