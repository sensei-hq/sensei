// The relay domain state (#421; design 2026-11-12 §2b, §2c): six entity states on
// ReactiveStageContext and one composite that hydrates from a RelaySource, applies the
// stream's events, and keeps the screen's own state. Pure: canned reads and canned
// events in, data out — no DOM, no fetch.
import { describe, it, expect, vi } from 'vitest';
import type { ApiResult } from '../result.js';
import type { RelayCreateRun, RelayCreatedRun, RelayNow, RelayQueue, RelayStreamEvent } from './types.js';
import { RelayState, type RelayReads, type RelaySource } from './relay-state.svelte.js';
import * as F from './fixtures.js';

/** A source whose reads and writes are spies, and whose stream the test drives. */
function fakeSource(over: Partial<Record<keyof RelaySource, unknown>> = {}) {
  let on: ((e: RelayStreamEvent) => void) | null = null;
  let opened: (() => void) | null = null;
  const teardown = vi.fn();
  const source = {
    readNow: vi.fn(async (): Promise<ApiResult<RelayNow>> => F.ok(F.now())),
    readQueue: vi.fn(async (): Promise<ApiResult<RelayQueue>> => F.ok(F.queue())),
    answer: vi.fn(async () => F.ok({ action_id: 'act-answer' })),
    send: vi.fn(async () => F.ok({ message_id: 'msg-1' })),
    addToBacklog: vi.fn(async () => F.ok({ item_id: 'item-new' })),
    createRun: vi.fn(
      async (): Promise<ApiResult<RelayCreatedRun>> =>
        F.ok({ run_id: F.RUN_ID, planning_task_id: 't-plan', crew_id: F.CREW_ID, status: 'planning' }),
    ),
    events: vi.fn((handler: (e: RelayStreamEvent) => void, onOpen: () => void) => {
      on = handler;
      opened = onOpen;
      return teardown;
    }),
    ...over,
  };
  return {
    source: source as unknown as RelaySource & typeof source,
    emit: (e: RelayStreamEvent) => on?.(e),
    open: () => opened?.(),
    teardown,
  };
}

const reads = (now: RelayNow | null = F.now(), queue: RelayQueue | null = F.queue()): RelayReads => ({
  now,
  nowError: null,
  queue,
  queueError: null,
});

const settle = () => new Promise((r) => setTimeout(r, 0));

describe('RelayState · hydrate: the reads become the six entity states', () => {
  it('lanes, decisions and messages by their ids; runs by run_id; the screen reads back what was read', () => {
    const now = F.sprintNow();
    const s = new RelayState(fakeSource().source, {}, reads(now));
    expect(s.lanes.items.map((l) => l.id)).toEqual(now.lanes.map((l) => l.id));
    expect(s.decisions.items.map((d) => d.id)).toEqual(now.decisions.map((d) => d.id));
    expect(s.messages.items.map((m) => m.id)).toEqual(now.person_messages.map((m) => m.id));
    expect(s.runs.items.map((r) => r.id)).toEqual(now.runs.map((r) => r.run_id));
    // The screen still reads `now`: assembled from the entity states, equal to the read.
    expect(s.now).toEqual(now);
  });

  it('tasks: the floating reviews and today’s completed, each marked by where it stands', () => {
    const now = F.now({
      floating_reviews: [F.floatingReview()],
      completed_today: [F.completed()],
    });
    const s = new RelayState(fakeSource().source, {}, reads(now));
    expect(s.tasks.items.map((t) => [t.id, t.stage])).toEqual([
      [now.floating_reviews[0].task_id, 'floating'],
      [now.completed_today[0].task_id, 'completed'],
    ]);
    expect(s.now?.floating_reviews).toEqual(now.floating_reviews);
    expect(s.now?.completed_today).toEqual(now.completed_today);
  });

  it('items: triage and backlog; a ticket with no item yet is keyed by its tracker and key', () => {
    const queue = F.queue();
    const s = new RelayState(fakeSource().source, {}, reads(F.now(), queue));
    const ids = s.items.items.map((i) => [i.id, i.stage]);
    const ticket = queue.triage.find((t) => t.item_id === null);
    expect(ticket, 'the fixture holds a tracker row').toBeDefined();
    expect(ids).toContainEqual([`${ticket!.ticket!.provider}:${ticket!.ticket!.key}`, 'triage']);
    expect(ids.filter(([, stage]) => stage === 'backlog').length).toBe(queue.backlog.length);
    expect(s.queue).toEqual(queue);
    expect(s.queueCount).toBe(queue.triage.length + queue.backlog.length);
  });

  it('no read is no state (D83): now, queue, runs and reviews read null, never empty', () => {
    const s = new RelayState(fakeSource().source, {}, { now: null, nowError: F.fail(503, 'down').error, queue: null, queueError: null });
    expect(s.now).toBeNull();
    expect(s.queue).toBeNull();
    expect(s.runs.items).toEqual([]);
    expect(s.runsBand).toBeNull();
    expect(s.floatingReviews).toBeNull();
    expect(s.queueCount).toBeNull();
    expect(s.nowError?.status).toBe(503);
    expect(s.hasCrew).toBe(false);
  });
});

describe('RelayState · apply: an event with its row changes that row alone', () => {
  it('a lane row updates the lane in place, with no read', async () => {
    const { source } = fakeSource();
    const s = new RelayState(source, {}, reads());
    const lane = s.lanes.items[0];
    s.apply({ action: 'update', entity: 'lane', data: { id: lane.id, title: 'Diagrams' } as never });
    await settle();
    expect(s.lanes.get(lane.id)?.title).toBe('Diagrams');
    expect(s.now?.lanes[0].title).toBe('Diagrams');
    expect(source.readNow).not.toHaveBeenCalled();
  });

  it('a lane added and a lane removed', () => {
    const s = new RelayState(fakeSource().source, {}, reads());
    const added = F.lane({ id: 'l-new', code: 'L7', title: 'Sync' });
    s.apply({ action: 'add', entity: 'lane', data: added as never });
    expect(s.now?.lanes.map((l) => l.id)).toContain('l-new');
    s.apply({ action: 'remove', entity: 'lane', data: { id: 'l-new' } as never });
    expect(s.now?.lanes.map((l) => l.id)).not.toContain('l-new');
  });

  it('a decision answered (removed) leaves Needs you; a message added joins it', () => {
    const now = F.now({ decisions: [F.decision()], person_messages: [] });
    const s = new RelayState(fakeSource().source, {}, reads(now));
    s.apply({ action: 'remove', entity: 'decision', data: { id: now.decisions[0].id } as never });
    expect(s.now?.decisions).toEqual([]);
    const msg = F.personMessage({ id: 'msg-9' });
    s.apply({ action: 'add', entity: 'message', data: msg as never });
    expect(s.now?.person_messages.map((m) => m.id)).toEqual(['msg-9']);
  });

  it('a run row moves the band by its id', () => {
    const now = F.sprintNow();
    const s = new RelayState(fakeSource().source, {}, reads(now));
    const run = now.runs[0];
    s.apply({ action: 'update', entity: 'run', data: { id: run.run_id, done_tasks: 9 } as never });
    expect(s.now?.runs[0].done_tasks).toBe(9);
    expect(s.now?.runs[0].run_id).toBe(run.run_id);
  });
});

describe('RelayState · apply: an event that names ids only (today’s daemon, until #418) re-reads', () => {
  it('a task, lane or relay event re-reads Now once; a run event re-reads Now and the queue', async () => {
    const { source } = fakeSource();
    const s = new RelayState(source, {}, reads());
    s.apply({ action: 'update', entity: 'task', data: { task_id: 't1' } } as RelayStreamEvent);
    await settle();
    expect(source.readNow).toHaveBeenCalledTimes(1);
    expect(source.readQueue).not.toHaveBeenCalled();
    s.apply({ action: 'add', entity: 'run', data: { run_id: F.RUN_ID, crew_id: F.CREW_ID } } as RelayStreamEvent);
    await settle();
    expect(source.readNow).toHaveBeenCalledTimes(2);
    expect(source.readQueue).toHaveBeenCalledTimes(1);
  });

  it('events while a read is in flight share it: one read, not one per event', async () => {
    const d = F.deferred<ApiResult<RelayNow>>();
    const { source } = fakeSource({ readNow: vi.fn(() => d.promise) });
    const s = new RelayState(source, {}, reads());
    for (const id of ['t1', 't2', 't3']) s.apply({ action: 'update', entity: 'task', data: { task_id: id } } as RelayStreamEvent);
    expect(source.readNow).toHaveBeenCalledTimes(1);
    d.resolve(F.ok(F.now({ lanes: [] })));
    await settle();
    expect(s.now?.lanes).toEqual([]);
  });

  it('a step is the task screen’s, and an unknown entity changes nothing', async () => {
    const { source } = fakeSource();
    const s = new RelayState(source, {}, reads());
    s.apply({ action: 'add', entity: 'step', data: { task_id: 't1' } } as RelayStreamEvent);
    s.apply({ action: 'add', entity: 'weather', data: {} } as RelayStreamEvent);
    await settle();
    expect(source.readNow).not.toHaveBeenCalled();
  });

  it('a failed re-read keeps what was shown and says why (D83)', async () => {
    const { source } = fakeSource({ readNow: vi.fn(async () => F.fail(500, 'boom')) });
    const s = new RelayState(source, {}, reads());
    const before = s.now;
    s.apply({ action: 'update', entity: 'lane', data: { lane_id: 'l1' } } as RelayStreamEvent);
    await settle();
    expect(s.now).toEqual(before);
    expect(s.nowError?.message).toBe('boom');
  });
});

describe('RelayState · connect: the stream is the source’s, the state applies it', () => {
  it('subscribes once; each open of the stream (the first too) reads Now and the queue; events apply; teardown', async () => {
    const f = fakeSource();
    const s = new RelayState(f.source, { workspace_id: 'w1' }, reads());
    const stop = s.connect();
    expect(f.source.events).toHaveBeenCalledTimes(1);
    // Nothing is read until the socket is open: the read then covers everything before it.
    await settle();
    expect(f.source.readNow).not.toHaveBeenCalled();
    f.open();
    await settle();
    expect(f.source.readNow).toHaveBeenCalledWith({ workspace_id: 'w1' });
    expect(f.source.readQueue).toHaveBeenCalledWith({ workspace_id: 'w1' });
    const lane = s.lanes.items[0];
    f.emit({ action: 'update', entity: 'lane', data: { id: lane.id, title: 'Live' } } as never);
    expect(s.lanes.get(lane.id)?.title).toBe('Live');
    f.open();
    await settle();
    expect(f.source.readNow).toHaveBeenCalledTimes(2);
    expect(f.source.readQueue).toHaveBeenCalledTimes(2);
    stop();
    expect(f.teardown).toHaveBeenCalledTimes(1);
  });
});

describe('RelayState · the person’s writes', () => {
  it('an answer goes to the source and the event it causes confirms it: no read of its own', async () => {
    const { source } = fakeSource();
    const s = new RelayState(source, {}, reads(F.now({ decisions: [F.decision()] })));
    const reason = await s.answer(s.decisions.items[0], 'db');
    expect(reason).toBeNull();
    expect(source.answer).toHaveBeenCalledWith({ decision_id: s.decisions.items[0].id, option: 'db', note: null, surface: 'desktop' });
    expect(source.readNow).not.toHaveBeenCalled();
  });

  it('a refused answer says why', async () => {
    const { source } = fakeSource({ answer: vi.fn(async () => F.fail(409, 'already answered')) });
    const s = new RelayState(source, {}, reads(F.now({ decisions: [F.decision()] })));
    expect(await s.answer(s.decisions.items[0], 'db')).toBe('already answered');
  });

  it('a message goes to the source; its reason when refused', async () => {
    const { source } = fakeSource();
    const s = new RelayState(source, {}, reads());
    const body = { crew_id: F.CREW_ID, to: { kind: 'crew' }, task_id: null, text: 'Try the other parser.' } as never;
    expect(await s.send(body)).toBeNull();
    expect(source.send).toHaveBeenCalledWith(body);
  });

  it('adding to the backlog re-reads the queue: the daemon emits nothing for it yet (#418)', async () => {
    const { source } = fakeSource();
    const queue = F.queue();
    const s = new RelayState(source, {}, reads(F.now(), queue));
    const item = queue.triage[0];
    expect(await s.addToBacklog(item)).toBeNull();
    expect(source.addToBacklog).toHaveBeenCalledTimes(1);
    expect(source.readQueue).toHaveBeenCalledTimes(1);
  });
});

// Queue cutover (phase 4), carried over from the app's RelayState: the person picks
// backlog items and runs a sprint. One write per press; a refusal says why and
// leaves the selection as it was.
const sprintBody = (): Omit<RelayCreateRun, 'item_ids' | 'crew_id'> => ({
  title: 'Sprint 15: Queue screen',
  goal: 'Run a sprint from the Relay screen',
  repo_key: null,
  workspace_id: null,
  config: { base_checkout: '/w/sensei', base_branch: 'develop', verify_cmd: 'make test-fast', auto_merge: true },
});

describe('RelayState · the sprint selection and createSprint', () => {
  it('toggleSelected adds then removes; clearSelection empties', () => {
    const s = new RelayState(fakeSource().source, {}, reads());
    s.toggleSelected('item-a');
    s.toggleSelected('item-b');
    expect(s.selectedCount).toBe(2);
    s.toggleSelected('item-a');
    expect(s.isSelected('item-a')).toBe(false);
    s.clearSelection();
    expect(s.selectedCount).toBe(0);
  });

  it('is single-flight, names the crew, reads Now and the queue back, and clears the selection', async () => {
    const d = F.deferred<ApiResult<RelayCreatedRun>>();
    const { source } = fakeSource({ createRun: vi.fn(() => d.promise) });
    const s = new RelayState(source, {}, reads());
    s.toggleSelected('item-a');
    s.toggleSelected('item-b');
    const first = s.createSprint(sprintBody());
    const second = s.createSprint(sprintBody());
    expect(source.createRun).toHaveBeenCalledTimes(1);
    const body = source.createRun.mock.calls[0][0] as RelayCreateRun;
    expect(body.item_ids).toEqual(['item-a', 'item-b']);
    expect(body.crew_id).toBe(F.CREW_ID);
    d.resolve(F.ok({ run_id: F.RUN_ID, planning_task_id: 't-plan', crew_id: F.CREW_ID, status: 'planning' }));
    expect(await first).toBe(F.RUN_ID);
    expect(await second).toBe(F.RUN_ID);
    // The run's own event re-reads Now; nothing tells the queue its items were planned (#418).
    expect(source.readQueue).toHaveBeenCalled();
    expect(s.selectedCount).toBe(0);
    expect(s.sprintError).toBeNull();
  });

  it('keeps the selection and says why on a refusal; an empty selection makes no call', async () => {
    const { source } = fakeSource({ createRun: vi.fn(async () => F.fail(409, 'more than one active crew: name crew_id')) });
    const s = new RelayState(source, {}, reads());
    expect(await s.createSprint(sprintBody())).toBeNull();
    expect(source.createRun).not.toHaveBeenCalled();
    s.toggleSelected('item-a');
    expect(await s.createSprint(sprintBody())).toBeNull();
    expect(s.sprintError).toContain('more than one active crew');
    expect(s.selectedCount).toBe(1);
  });
});
