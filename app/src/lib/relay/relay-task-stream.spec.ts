import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { RelayTaskState } from '@sensei/state';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import { subscribeTaskStream } from './relay-task-stream.js';

// F8: the route owns the socket and dispatches by entity — steps apply
// directly, task notifications re-read their task, unregister re-reads
// (its interrupts name no task), everything else is ignored.

class MockEventSource {
  static instances: MockEventSource[] = [];
  readyState = 0;
  onmessage: ((e: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  onopen: (() => void) | null = null;
  constructor(public url: string) {
    MockEventSource.instances.push(this);
  }
  close() {
    this.readyState = 2;
  }
  send(payload: unknown) {
    this.onmessage?.({ data: JSON.stringify(payload) });
  }
  open() {
    this.readyState = 1;
    this.onopen?.();
  }
}

beforeEach(() => {
  MockEventSource.instances = [];
  vi.stubGlobal('EventSource', MockEventSource);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

const wired = () => {
  const api = F.fakeRelayApi();
  api.tryGetTask.mockResolvedValue({ ok: true, data: F.taskView() });
  const state = new RelayTaskState({ view: F.taskView(), error: null }, api, 'crew-1', 'task-1');
  const stop = subscribeTaskStream(7744, state, 'task-1');
  const socket = MockEventSource.instances[0];
  return { api, state, stop, socket };
};

describe('subscribeTaskStream', () => {
  it('opens the stream narrowed to the task’s crew (#418)', () => {
    const { stop, socket } = wired();
    expect(socket.url).toBe('http://127.0.0.1:7744/api/relay/events?crew_id=crew-1');
    stop();
  });

  it('applies step events for its task, ignoring others', () => {
    const { state, stop, socket } = wired();
    const apply = vi.spyOn(state, 'applyStep');
    socket.send({ action: 'add', entity: 'step', data: { task_id: 'task-9', step: { key: 'x' } } });
    expect(apply).not.toHaveBeenCalled();
    socket.send({
      action: 'add',
      entity: 'step',
      data: { task_id: 'task-1', step: { key: 'a', title: 'A', state: 'open', reason: null, updated_at: 'x' } },
    });
    expect(apply).toHaveBeenCalledWith('task-1', expect.objectContaining({ key: 'a' }));
    stop();
  });

  it('re-reads on task notifications for its task only', async () => {
    const { api, stop, socket } = wired();
    const reads = () => (api.tryGetTask as unknown as { mock: { calls: unknown[] } }).mock.calls.length;
    const atSubscribe = reads();
    socket.send({ action: 'update', entity: 'task', data: { op: 'report', task_id: 'task-9' } });
    await Promise.resolve();
    expect(reads()).toBe(atSubscribe);
    socket.send({ action: 'update', entity: 'task', data: { op: 'report', task_id: 'task-1' } });
    await Promise.resolve();
    expect(reads()).toBe(atSubscribe + 1);
    stop();
  });

  it('re-reads on unregister (op-only), ignores other op-only events', async () => {
    const { api, stop, socket } = wired();
    const reads = () => (api.tryGetTask as unknown as { mock: { calls: unknown[] } }).mock.calls.length;
    const atSubscribe = reads();
    socket.send({ action: 'add', entity: 'relay', data: { op: 'register' } });
    await Promise.resolve();
    expect(reads()).toBe(atSubscribe);
    socket.send({ action: 'add', entity: 'relay', data: { op: 'unregister' } });
    await Promise.resolve();
    expect(reads()).toBe(atSubscribe + 1);
    stop();
  });

  it('covers the loader-to-subscribe gap, then re-reads on reconnect only', async () => {
    const { api, stop, socket } = wired();
    const reads = () => (api.tryGetTask as unknown as { mock: { calls: unknown[] } }).mock.calls.length;
    // The covering re-read fires at subscribe…
    expect(reads()).toBe(1);
    // …the first open is the initial connect, not a reconnect…
    socket.open();
    await Promise.resolve();
    expect(reads()).toBe(1);
    // …later opens re-read (broadcast has no replay).
    socket.open();
    await Promise.resolve();
    expect(reads()).toBe(2);
    stop();
  });

  it('ignores unknown entities', () => {
    const { state, stop, socket } = wired();
    const apply = vi.spyOn(state, 'applyStep');
    const changed = vi.spyOn(state, 'applyTaskChanged');
    socket.send({ action: 'add', entity: 'workspace', data: {} });
    expect(apply).not.toHaveBeenCalled();
    expect(changed).not.toHaveBeenCalled();
    stop();
  });
});
