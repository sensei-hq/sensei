// The relay sources (#421; design 2026-11-12 §2b, §4): the daemon's SSE on the
// desktop, and the seam the dōjō's Supabase realtime fills. Each is a RelaySource:
// reads, writes, and a stream that reports every open — the state reads on each, so
// nothing between the loader's read and the socket opening is missed.
import { describe, it, expect, vi } from 'vitest';
import type { RelayApi } from '../client.js';
import type { RelayStreamEvent } from '../types.js';
import * as F from '../fixtures.js';
import { relaySseSource, type RelayEventStream } from './sse.js';
import { relayRealtimeSource, cannedRealtime, type RelayRealtimeTransport } from './realtime.js';

function fakeApi(): RelayApi & Record<string, ReturnType<typeof vi.fn>> {
  return {
    tryGetNow: vi.fn(async () => F.ok(F.now())),
    tryGetQueue: vi.fn(async () => F.ok(F.queue())),
    tryGetTask: vi.fn(async () => F.ok(F.taskView())),
    trySendMessage: vi.fn(async () => F.ok({ message_id: 'm' })),
    tryAnswerDecision: vi.fn(async () => F.ok({ action_id: 'a' })),
    tryAddToBacklog: vi.fn(async () => F.ok({ item_id: 'i' })),
    tryCreateRun: vi.fn(async () => F.ok({ run_id: F.RUN_ID, planning_task_id: 't', crew_id: F.CREW_ID, status: 'planning' as const })),
  } as never;
}

/** An event stream the test opens and fires by hand (the EventManager's shape). */
function fakeStream() {
  let handler: ((e: RelayStreamEvent) => void) | null = null;
  const stream: RelayEventStream & { open(): void; fire(e: RelayStreamEvent): void; destroyed: boolean } = {
    onopen: null,
    destroyed: false,
    subscribe(h) {
      handler = h;
      return () => {
        handler = null;
      };
    },
    destroy() {
      this.destroyed = true;
    },
    open() {
      this.onopen?.();
    },
    fire(e) {
      handler?.(e);
    },
  };
  return stream;
}

const lane = (): RelayStreamEvent => ({ action: 'update', entity: 'lane', data: { lane_id: 'l1' } });

describe('relaySseSource — the daemon', () => {
  it('reads and writes through the relay client', async () => {
    const api = fakeApi();
    const source = relaySseSource(api, () => fakeStream());
    expect((await source.readNow({ workspace_id: 'w1' })).ok).toBe(true);
    expect(api.tryGetNow).toHaveBeenCalledWith({ workspace_id: 'w1' });
    await source.readQueue({});
    expect(api.tryGetQueue).toHaveBeenCalledWith({});
    const answer = { decision_id: 'd', option: 'db', note: null, surface: 'desktop' as const };
    await source.answer(answer);
    expect(api.tryAnswerDecision).toHaveBeenCalledWith(answer);
    await source.addToBacklog({ item_id: 'x' });
    expect(api.tryAddToBacklog).toHaveBeenCalledWith({ item_id: 'x' });
  });

  it('passes every event on, reports every open (the first included), and the teardown closes it', () => {
    const stream = fakeStream();
    const source = relaySseSource(fakeApi(), () => stream);
    const on = vi.fn();
    const opened = vi.fn();
    const stop = source.events(on, opened);
    expect(opened).not.toHaveBeenCalled();
    stream.open();
    expect(opened).toHaveBeenCalledTimes(1);
    stream.fire(lane());
    expect(on).toHaveBeenCalledWith(lane());
    stream.open();
    expect(opened).toHaveBeenCalledTimes(2);
    stop();
    expect(stream.destroyed).toBe(true);
    stream.fire(lane());
    expect(on).toHaveBeenCalledTimes(1);
  });
});

describe('relayRealtimeSource — the dōjō seam', () => {
  it('maps the channel status: each SUBSCRIBED (the first, and after a drop) is an open', () => {
    const t = cannedRealtime({ now: F.now(), queue: F.queue() });
    const source = relayRealtimeSource(t);
    const on = vi.fn();
    const opened = vi.fn();
    const stop = source.events(on, opened);
    t.status('SUBSCRIBED');
    expect(opened).toHaveBeenCalledTimes(1);
    t.emit(lane());
    expect(on).toHaveBeenCalledWith(lane());
    t.status('CHANNEL_ERROR');
    expect(opened).toHaveBeenCalledTimes(1);
    t.status('SUBSCRIBED');
    expect(opened).toHaveBeenCalledTimes(2);
    stop();
    t.emit(lane());
    expect(on).toHaveBeenCalledTimes(1);
  });

  it('the canned double answers its reads and records its writes', async () => {
    const t = cannedRealtime({ now: F.now(), queue: F.queue() });
    const source = relayRealtimeSource(t);
    const now = await source.readNow({});
    expect(now.ok && now.data.crews.length).toBe(1);
    await source.send({ crew_id: F.CREW_ID, to: { kind: 'crew' }, task_id: null, text: 'hi', surface: 'dojo' } as never);
    expect(t.writes.map((w) => w.kind)).toEqual(['send']);
  });

  it('a transport that cannot read says so, never an empty crew (D83)', async () => {
    const t: RelayRealtimeTransport = { ...cannedRealtime({ now: null, queue: null }) };
    const now = await relayRealtimeSource(t).readNow({});
    expect(now.ok).toBe(false);
  });
});
