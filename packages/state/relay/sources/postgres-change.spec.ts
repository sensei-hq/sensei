// The Supabase mapping (#414, design D3): a `postgres_changes` payload on a relay table
// becomes the StateEvent the daemon streams for the same row. The parity cases apply
// one row to two RelayStates — once as the daemon's SSE event, once as the mapped
// payload — and expect the same state: the state cannot tell the planes apart.
import { describe, it, expect, vi } from 'vitest';
import type { ApiResult } from '../../result.js';
import { RelayState, type RelaySource } from '../relay-state.svelte.js';
import { RelayTaskState } from '../task-state.svelte.js';
import { RunsState } from '../entity-states.svelte.js';
import { toStats } from '../relay-selectors.js';
import type { RelayNow, RelayQueue, RelayStreamEvent } from '../types.js';
import * as F from '../fixtures.js';
import { fromPostgresChange, RELAY_TABLES, type PostgresChangePayload } from './postgres-change.js';

const payload = (
  table: string,
  eventType: PostgresChangePayload['eventType'],
  row: Record<string, unknown>,
): PostgresChangePayload => ({
  schema: 'relay',
  table,
  eventType,
  new: eventType === 'DELETE' ? {} : row,
  old: eventType === 'DELETE' ? row : {},
});

/** The relay tables as a publisher that writes the screen's rows would declare them:
 *  each row is applied as it is. Nothing in today's `relay` schema is such a table. */
const SCREEN_TABLES = Object.fromEntries(
  Object.entries(RELAY_TABLES).map(([table, entity]) => [table, { entity, screenRow: true as const }]),
);

describe('fromPostgresChange — a table row names what changed', () => {
  it('INSERT is add, UPDATE is update, DELETE is remove with the id from old', () => {
    expect(fromPostgresChange(payload('messages', 'INSERT', { id: 'm-1', body_md: 'hi' }))).toEqual({
      action: 'add',
      entity: 'message',
      data: { op: 'insert', message_id: 'm-1' },
    });
    expect(fromPostgresChange(payload('messages', 'UPDATE', { id: 'm-1' }))?.action).toBe('update');
    expect(fromPostgresChange(payload('messages', 'DELETE', { id: 'm-1' }))).toEqual({
      action: 'remove',
      entity: 'message',
      data: { op: 'delete', message_id: 'm-1' },
    });
  });

  it('names each entity by the id key the daemon uses, and carries none of the table columns', () => {
    expect(fromPostgresChange(payload('tasks', 'UPDATE', { id: 't-1', state: 'working' }))?.data).toEqual({
      op: 'update',
      task_id: 't-1',
    });
    expect(fromPostgresChange(payload('lanes', 'UPDATE', { id: 'l-1', token_hash: 'x' }))?.data).toEqual({
      op: 'update',
      lane_id: 'l-1',
    });
    expect(fromPostgresChange(payload('runs', 'DELETE', { id: 'r-1' }))?.data).toEqual({ op: 'delete', run_id: 'r-1' });
    expect(fromPostgresChange(payload('items', 'INSERT', { id: 'i-1', state: 'backlog' }))?.data).toEqual({
      op: 'insert',
      item_id: 'i-1',
    });
    expect(fromPostgresChange(payload('decisions', 'INSERT', { id: 'd-1' }))?.data).toEqual({
      op: 'insert',
      decision_id: 'd-1',
    });
  });

  it('maps every relay table to its entity, and an unmapped table to null', () => {
    expect(RELAY_TABLES).toEqual({
      lanes: 'lane',
      tasks: 'task',
      runs: 'run',
      items: 'item',
      decisions: 'decision',
      messages: 'message',
    });
    expect(fromPostgresChange(payload('actions', 'INSERT', { id: 'a-1' }))).toBeNull();
  });

  it('takes the table map as a parameter: the dōjō names its own tables', () => {
    const tables = { relay_asks: 'decision' };
    expect(fromPostgresChange(payload('relay_asks', 'INSERT', { id: 'ask-9' }), tables)?.entity).toBe('decision');
    expect(fromPostgresChange(payload('decisions', 'INSERT', { id: 'ask-9' }), tables)).toBeNull();
  });

  it('a table declared in the screen’s shape carries its row, with the daemon’s id key beside id', () => {
    const msg = F.personMessage();
    expect(fromPostgresChange(payload('messages', 'INSERT', msg), SCREEN_TABLES)).toEqual({
      action: 'add',
      entity: 'message',
      data: msg,
    });
    expect(fromPostgresChange(payload('tasks', 'UPDATE', { id: 't-1', state: 'working' }), SCREEN_TABLES)?.data).toEqual({
      id: 't-1',
      state: 'working',
      task_id: 't-1',
    });
    expect(fromPostgresChange(payload('messages', 'DELETE', { id: msg.id }), SCREEN_TABLES)).toEqual({
      action: 'remove',
      entity: 'message',
      data: { id: msg.id },
    });
  });
});

/** A source that is never read: the parity cases move rows without a read. */
function quietSource(): RelaySource & { readNow: ReturnType<typeof vi.fn>; readQueue: ReturnType<typeof vi.fn> } {
  const never = vi.fn(async (): Promise<ApiResult<never>> => F.fail(500, 'not read in parity'));
  return {
    readNow: vi.fn(async (): Promise<ApiResult<RelayNow>> => F.fail(500, 'not read in parity')),
    readQueue: vi.fn(async (): Promise<ApiResult<RelayQueue>> => F.fail(500, 'not read in parity')),
    answer: never,
    send: never,
    addToBacklog: never,
    addTriage: never,
    createRun: never,
    readRuns: never,
    startRun: never,
    cancelRun: never,
    events: () => () => {},
  } as never;
}

const reads = (now: RelayNow) => ({ now, nowError: null, queue: F.queue(), queueError: null });
const settle = () => new Promise((r) => setTimeout(r, 0));

/** Apply the same change through both planes; return the two states. */
async function both(now: RelayNow, sse: RelayStreamEvent, pg: PostgresChangePayload) {
  const viaSse = new RelayState(quietSource(), {}, reads(now));
  const viaPg = new RelayState(quietSource(), {}, reads(now));
  viaSse.apply(sse);
  const mapped = fromPostgresChange(pg, SCREEN_TABLES);
  expect(mapped).not.toBeNull();
  viaPg.apply(mapped as RelayStreamEvent);
  await settle();
  return { viaSse, viaPg };
}

function expectSame({ viaSse, viaPg }: { viaSse: RelayState; viaPg: RelayState }) {
  expect(viaPg.now).toEqual(viaSse.now);
  expect(viaPg.queue).toEqual(viaSse.queue);
  const at = Date.now();
  expect(toStats(viaPg.now, at)).toEqual(toStats(viaSse.now, at));
  expect(viaPg.source.readNow).not.toHaveBeenCalled();
  expect(viaPg.source.readQueue).not.toHaveBeenCalled();
}

// Parity holds only for a table that publishes the screen's rows: SCREEN_TABLES declares
// that. Today's `relay` rows are not those rows; the next block covers them.
describe('parity — a screen-shaped table and the SSE event leave the same state', () => {
  it('a lane', async () => {
    const lane = { ...F.lane(), title: 'Renamed', lease_state: 'expired' as const };
    const sse = { action: 'update', entity: 'lane', data: { ...lane, op: 'suspended', lane_id: lane.id } } as RelayStreamEvent;
    const states = await both(F.now(), sse, payload('lanes', 'UPDATE', lane));
    expect(states.viaPg.lanes.get(lane.id)?.title).toBe('Renamed');
    expectSame(states);
  });

  it('a run', async () => {
    const now = F.sprintNow();
    const row = RunsState.row({ ...now.runs[0], done_tasks: 9 });
    const { run_id: _runId, ...pgRow } = row;
    const sse = { action: 'update', entity: 'run', data: row } as unknown as RelayStreamEvent;
    const states = await both(now, sse, payload('runs', 'UPDATE', pgRow));
    expect(states.viaPg.now?.runs[0].done_tasks).toBe(9);
    expectSame(states);
  });

  it('an item', async () => {
    const row = { ...F.backlogItem({ item_id: 'item-99' }), id: 'item-99', stage: 'backlog' };
    const sse = { action: 'add', entity: 'item', data: row } as unknown as RelayStreamEvent;
    const states = await both(F.now(), sse, payload('items', 'INSERT', row));
    expect(states.viaPg.items.get('item-99')).toBeDefined();
    expectSame(states);
  });

  it('a decision', async () => {
    const now = F.now({ decisions: [F.decision()] });
    const sse = { action: 'remove', entity: 'decision', data: { id: 'ask-1' } } as unknown as RelayStreamEvent;
    const states = await both(now, sse, payload('decisions', 'DELETE', { id: 'ask-1' }));
    expect(states.viaPg.now?.decisions).toEqual([]);
    expectSame(states);
  });

  it('a message', async () => {
    const msg = F.personMessage({ id: 'msg-9' });
    const sse = { action: 'add', entity: 'message', data: msg } as unknown as RelayStreamEvent;
    const states = await both(F.now({ person_messages: [] }), sse, payload('messages', 'INSERT', msg));
    expect(states.viaPg.now?.person_messages.map((m) => m.id)).toEqual(['msg-9']);
    expectSame(states);
  });
});

/** Rows with the columns `database/ddl/table/relay/*.ddl` gives them, and nothing else. */
const tableRow = {
  item: (id: string) => ({
    user_id: 'u-1',
    id,
    workspace_id: 'w1',
    repo_key: F.REPO,
    ticket_provider: null,
    ticket_key: null,
    title: 'Gap',
    kind: 'gap',
    tags: null,
    state: 'backlog',
    position: 3,
    added_by: null,
    added_at: '2026-11-18T10:00:00Z',
  }),
  run: (id: string) => ({
    user_id: 'u-1',
    id,
    crew_id: F.CREW_ID,
    title: 'Sprint',
    goal: 'Ship it',
    state: 'active',
    repo_key: F.REPO,
    workspace_id: 'w1',
    config: {},
    created_at: '2026-11-18T10:00:00Z',
    started_at: '2026-11-18T10:05:00Z',
    completed_at: null,
  }),
  lane: (id: string) => ({
    user_id: 'u-1',
    id,
    crew_id: F.CREW_ID,
    code: 'L1',
    title: 'Renamed in the table',
    role: 'contributor',
    family: 'claude-code',
    token_hash: 'not-for-the-screen',
    registered_at: '2026-11-18T10:00:00Z',
  }),
};

/** A source whose reads answer: the re-read is how a table row reaches the screen. */
function readingSource(now: RelayNow, queue: RelayQueue) {
  const source = quietSource();
  source.readNow.mockImplementation(async () => F.ok(now));
  source.readQueue.mockImplementation(async () => F.ok(queue));
  return source;
}

describe('a relay table row (today’s schema) re-reads what it names', () => {
  it('an INSERT on relay.items re-reads the queue, and the item shows in the backlog', async () => {
    const added = F.backlogItem({ item_id: 'item-new', title: 'Gap', position: 3 });
    const queue = F.queue({ backlog: [...F.queue().backlog, added] });
    const source = readingSource(F.now(), queue);
    const s = new RelayState(source, {}, reads(F.now()));
    s.apply(fromPostgresChange(payload('items', 'INSERT', tableRow.item('item-new'))) as RelayStreamEvent);
    await settle();
    expect(source.readQueue).toHaveBeenCalledTimes(1);
    expect(s.items.backlog.map((b) => b.item_id)).toContain('item-new');
    expect(s.items.items.every((i) => i.stage === 'triage' || i.stage === 'backlog')).toBe(true);
  });

  it('an UPDATE on relay.runs re-reads Now, and the run keeps its task counts', async () => {
    const now = F.sprintNow();
    const after = { ...now, runs: [{ ...now.runs[0], done_tasks: 9 }] };
    const source = readingSource(after, F.queue());
    const s = new RelayState(source, {}, reads(now));
    s.apply(fromPostgresChange(payload('runs', 'UPDATE', tableRow.run(now.runs[0].run_id))) as RelayStreamEvent);
    await settle();
    expect(source.readNow).toHaveBeenCalled();
    expect(s.now?.runs[0].done_tasks).toBe(9);
    expect(s.now?.runs[0].total_tasks).toBeTypeOf('number');
  });

  it('an UPDATE on relay.lanes re-reads Now and puts no table column on the lane', async () => {
    const now = F.now();
    const id = now.lanes[0].id;
    const source = readingSource(now, F.queue());
    const s = new RelayState(source, {}, reads(now));
    s.apply(fromPostgresChange(payload('lanes', 'UPDATE', tableRow.lane(id))) as RelayStreamEvent);
    await settle();
    expect(source.readNow).toHaveBeenCalled();
    expect(s.lanes.get(id)).not.toHaveProperty('token_hash');
    expect(s.lanes.get(id)).toHaveProperty('presence');
  });
});

describe('a task change reaches the task view', () => {
  it('an UPDATE on tasks for the viewed task carries task_id and re-reads it', async () => {
    const readTask = vi.fn(async () => F.ok(F.taskView()));
    const source = { readTask, answer: vi.fn(), events: () => () => {} };
    const s = new RelayTaskState({ view: F.taskView(), error: null }, source as never, F.CREW_ID, 't-1');
    const mapped = fromPostgresChange(payload('tasks', 'UPDATE', { id: 't-1', state: 'in_review' }));
    expect(mapped?.data.task_id).toBe('t-1');
    s.apply(mapped as RelayStreamEvent);
    await settle();
    expect(readTask).toHaveBeenCalledWith(F.CREW_ID, 't-1');
  });
});
