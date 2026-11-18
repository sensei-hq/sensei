// The realtime channel adapter (#414, design D4): a transport's `subscribe` over any
// client with the supabase-js channel shape. A fake client stands in for supabase-js:
// the package takes no dependency on it.
import { describe, it, expect, vi, afterEach } from 'vitest';
import type { RelayStreamEvent } from '../types.js';
import type { PostgresChangePayload } from './postgres-change.js';
import { realtimeChannel, type RealtimeClientLike } from './channel.js';

type Listener = { filter: { event: string; schema: string; table: string }; cb: (p: PostgresChangePayload) => void };

/** A supabase-js-shaped client: records each channel, its listeners and its status callback. */
function fakeClient() {
  const channels: Array<{
    topic: string;
    listeners: Listener[];
    status: ((s: string, err?: Error) => void) | null;
  }> = [];
  const removed: unknown[] = [];
  const client = {
    channel(topic: string) {
      const record = { topic, listeners: [] as Listener[], status: null as ((s: string, err?: Error) => void) | null };
      channels.push(record);
      const channel = {
        on(_type: 'postgres_changes', filter: Listener['filter'], cb: Listener['cb']) {
          record.listeners.push({ filter, cb });
          return channel;
        },
        subscribe(cb?: (s: string, err?: Error) => void) {
          record.status = cb ?? null;
          return channel;
        },
      };
      (record as Record<string, unknown>).handle = channel;
      return channel;
    },
    removeChannel: vi.fn(async (channel: unknown) => {
      removed.push(channel);
      return 'ok';
    }),
  } satisfies RealtimeClientLike;
  /** Deliver a payload to every listener on `table`, as the server would. */
  const change = (at: number, p: PostgresChangePayload) => {
    for (const l of channels[at].listeners) if (l.filter.table === p.table) l.cb(p);
  };
  return { client, channels, removed, change };
}

const lanePayload = (id = 'l-1'): PostgresChangePayload => ({
  schema: 'relay',
  table: 'lanes',
  eventType: 'UPDATE',
  new: { id, title: 'Live' },
  old: {},
});

const spec = { topic: 'relay:crew-1', schema: 'relay' };

afterEach(() => vi.restoreAllMocks());

describe('realtimeChannel — a transport subscribe over a supabase-shaped client', () => {
  it('opens one channel per subscribe, with one postgres_changes listener per relay table', () => {
    const f = fakeClient();
    const ch = realtimeChannel(f.client, spec);
    expect(f.channels).toHaveLength(0);
    ch.subscribe(vi.fn(), vi.fn());
    expect(f.channels).toHaveLength(1);
    expect(f.channels[0].topic).toBe('relay:crew-1');
    expect(f.channels[0].listeners.map((l) => l.filter)).toEqual(
      ['lanes', 'tasks', 'runs', 'items', 'decisions', 'messages'].map((table) => ({ event: '*', schema: 'relay', table })),
    );
    ch.subscribe(vi.fn(), vi.fn());
    expect(f.channels).toHaveLength(2);
  });

  it('listens on the tables the spec names, when it names them', () => {
    const f = fakeClient();
    realtimeChannel(f.client, { ...spec, tables: { relay_lanes: 'lane' } }).subscribe(vi.fn(), vi.fn());
    expect(f.channels[0].listeners.map((l) => l.filter.table)).toEqual(['relay_lanes']);
  });

  it('forwards each change as the daemon event, and each status', () => {
    const f = fakeClient();
    const on = vi.fn();
    const onStatus = vi.fn();
    realtimeChannel(f.client, spec).subscribe(on, onStatus);
    f.change(0, lanePayload());
    expect(on).toHaveBeenCalledWith({
      action: 'update',
      entity: 'lane',
      data: { id: 'l-1', title: 'Live', lane_id: 'l-1' },
    } satisfies RelayStreamEvent);
    f.channels[0].status?.('SUBSCRIBED');
    f.channels[0].status?.('CHANNEL_ERROR', new Error('dropped'));
    f.channels[0].status?.('TIMED_OUT');
    f.channels[0].status?.('CLOSED');
    expect(onStatus.mock.calls.map((c) => c[0])).toEqual(['SUBSCRIBED', 'CHANNEL_ERROR', 'TIMED_OUT', 'CLOSED']);
  });

  it('logs a payload it cannot map, skips it, and keeps forwarding', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const f = fakeClient();
    const on = vi.fn();
    realtimeChannel(f.client, { ...spec, tables: { lanes: 'lane' } }).subscribe(on, vi.fn());
    // A listener for a table the map does not name: the server sent what nothing maps.
    f.channels[0].listeners[0].cb({ ...lanePayload(), table: 'actions' });
    expect(on).not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalledTimes(1);
    expect(warn.mock.calls[0].join(' ')).toContain('relay.actions');
    f.change(0, lanePayload());
    expect(on).toHaveBeenCalledTimes(1);
  });

  it('logs a status it does not know instead of forwarding it', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const f = fakeClient();
    const onStatus = vi.fn();
    realtimeChannel(f.client, spec).subscribe(vi.fn(), onStatus);
    f.channels[0].status?.('JOINING');
    expect(onStatus).not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalledTimes(1);
  });

  it('the teardown removes the channel it opened, once, and later payloads reach no handler', () => {
    const f = fakeClient();
    const on = vi.fn();
    const onStatus = vi.fn();
    const stop = realtimeChannel(f.client, spec).subscribe(on, onStatus);
    stop();
    expect(f.client.removeChannel).toHaveBeenCalledTimes(1);
    expect(f.removed[0]).toBe((f.channels[0] as Record<string, unknown>).handle);
    stop();
    expect(f.client.removeChannel).toHaveBeenCalledTimes(1);
    f.change(0, lanePayload());
    f.channels[0].status?.('SUBSCRIBED');
    expect(on).not.toHaveBeenCalled();
    expect(onStatus).not.toHaveBeenCalled();
  });
});
