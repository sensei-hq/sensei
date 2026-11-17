// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { RelayTaskState } from './task-state.svelte.js';
import * as F from './fixtures.js';
import type { RelayApi } from './client.js';
import type { RelayEvents } from './relay-state.svelte.js';
import type { RelayStreamEvent, RelayTaskStep, RelayTaskView } from './types.js';

/** The relay client, its every call a spy (the app's fakeRelayApi; a package cannot share it). */
const fakeApi = () =>
  ({
    tryGetNow: vi.fn(async () => F.ok(F.now())),
    tryGetQueue: vi.fn(async () => F.ok(F.queue())),
    tryGetTask: vi.fn(async () => F.ok(F.taskView())),
    trySendMessage: vi.fn(async () => F.ok({ message_id: 'msg-1' })),
    tryAnswerDecision: vi.fn(async () => F.ok({ action_id: 'act-answer' })),
    tryAddToBacklog: vi.fn(async () => F.ok({ item_id: 'item-new' })),
    tryAddTriage: vi.fn(async () => F.ok({ item_id: 'item-added' })),
    tryCreateRun: vi.fn(async () => F.ok({ run_id: F.RUN_ID, planning_task_id: 't-plan', crew_id: F.CREW_ID, status: 'planning' as const })),
  }) as unknown as { [K in keyof RelayApi]: ReturnType<typeof vi.fn> & RelayApi[K] };

// F8: the stream feeds the state through apply-style updaters — steps land
// directly, task notifications re-read. Anything for another task is ignored.

const step = (key: string, state: RelayTaskStep['state'] = 'open'): RelayTaskStep => ({
  key,
  title: `Step ${key}`,
  state,
  reason: null,
  updated_at: '2026-11-06T10:11:00Z',
});

/** A stream double: the state's handlers, captured, so a test can send events and opens. */
const fakeStream = () => {
  const s = {
    send: (_e: RelayStreamEvent) => {},
    open: () => {},
    stopped: 0,
    subscriptions: 0,
  };
  const events: RelayEvents = (on, onOpen) => {
    s.subscriptions += 1;
    s.send = on;
    s.open = onOpen;
    return () => {
      s.stopped += 1;
    };
  };
  return { stream: s, events };
};

const noEvents: RelayEvents = () => () => {};

const stateFor = (view: RelayTaskView | null = F.taskView()): RelayTaskState =>
  new RelayTaskState({ view, error: null }, fakeApi(), 'crew-1', 'task-1', noEvents);

describe('RelayTaskState stream updaters', () => {
  it('applyStep appends a new step in place', () => {
    const s = stateFor();
    expect(s.view!.steps).toHaveLength(0);
    s.applyStep('task-1', step('a'));
    expect(s.view!.steps.map((x) => x.key)).toEqual(['a']);
  });

  it('applyStep restates an existing key without moving it', () => {
    const s = stateFor();
    s.applyStep('task-1', step('a'));
    s.applyStep('task-1', step('b'));
    s.applyStep('task-1', { ...step('a'), state: 'done' });
    expect(s.view!.steps.map((x) => x.key)).toEqual(['a', 'b']);
    expect(s.view!.steps[0].state).toBe('done');
  });

  it('applyStep ignores another task, or no view yet', () => {
    const s = stateFor();
    s.applyStep('task-9', step('a'));
    expect(s.view!.steps).toHaveLength(0);
    const empty = stateFor(null);
    expect(() => empty.applyStep('task-1', step('a'))).not.toThrow();
    expect(empty.view).toBeNull();
  });

  it('applyTaskChanged re-reads its own task only', async () => {
    const api = fakeApi();
    const fresh = F.taskView();
    api.tryGetTask.mockResolvedValue({ ok: true, data: fresh });
    const s = new RelayTaskState({ view: F.taskView(), error: null }, api, 'crew-1', 'task-1', noEvents);
    await s.applyTaskChanged('task-9');
    expect(api.tryGetTask).not.toHaveBeenCalled();
    await s.applyTaskChanged('task-1');
    expect(api.tryGetTask).toHaveBeenCalledWith('crew-1', 'task-1');
    // ($state proxies the read — compare by value, not identity.)
    expect(s.view?.task.id).toBe(fresh.task.id);
    expect(s.view?.steps).toEqual(fresh.steps);
  });
});

// The state listens through its source, as RelayState does (#414): the page calls
// connect(), and the surface (daemon SSE, dōjō realtime) is the source's alone.
describe('RelayTaskState.connect', () => {
  const connected = () => {
    const api = fakeApi();
    const { stream, events } = fakeStream();
    const s = new RelayTaskState({ view: F.taskView(), error: null }, api, 'crew-1', 'task-1', events);
    const stop = s.connect();
    const reads = () => api.tryGetTask.mock.calls.length;
    return { api, s, stream, stop, reads };
  };
  const settle = () => new Promise((r) => setTimeout(r, 0));

  it('subscribes once, and its teardown ends the subscription', () => {
    const { stream, stop } = connected();
    expect(stream.subscriptions).toBe(1);
    stop();
    expect(stream.stopped).toBe(1);
  });

  it('reads on each open of the stream, the first included: the loader read before it opened, and a broadcast replays nothing', async () => {
    const { stream, reads } = connected();
    await settle();
    expect(reads()).toBe(0);
    stream.open();
    await settle();
    expect(reads()).toBe(1);
    stream.open();
    await settle();
    expect(reads()).toBe(2);
  });

  it("applies its own task's steps and ignores another task's", async () => {
    const { s, stream } = connected();
    await settle();
    stream.send({ action: 'add', entity: 'step', data: { task_id: 'task-9', step: step('x') } });
    expect(s.view!.steps).toHaveLength(0);
    stream.send({ action: 'add', entity: 'step', data: { task_id: 'task-1', step: step('a') } });
    expect(s.view!.steps.map((x) => x.key)).toEqual(['a']);
  });

  it("re-reads on its own task's event, and on an unregister (which names no task)", async () => {
    const { stream, reads } = connected();
    await settle();
    const at = reads();
    stream.send({ action: 'update', entity: 'task', data: { op: 'report', task_id: 'task-9' } });
    await settle();
    expect(reads()).toBe(at);
    stream.send({ action: 'update', entity: 'task', data: { op: 'report', task_id: 'task-1' } });
    await settle();
    expect(reads()).toBe(at + 1);
    stream.send({ action: 'add', entity: 'relay', data: { op: 'register' } });
    await settle();
    expect(reads()).toBe(at + 1);
    stream.send({ action: 'add', entity: 'relay', data: { op: 'unregister' } });
    await settle();
    expect(reads()).toBe(at + 2);
  });

  it('ignores entities it does not hold', async () => {
    const { s, stream, reads } = connected();
    await settle();
    const at = reads();
    stream.send({ action: 'add', entity: 'workspace', data: {} });
    await settle();
    expect(reads()).toBe(at);
    expect(s.view!.steps).toHaveLength(0);
  });
});
