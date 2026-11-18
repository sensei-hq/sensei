// The realtime channel adapter (#414, design D4): a transport's `subscribe` over any
// client with the supabase-js channel shape. A fake client stands in for supabase-js:
// the package takes no dependency on it.
import { describe, it, expect, vi, afterEach } from 'vitest';
import type { RelayStreamEvent } from '../types.js';
import type { PostgresChangePayload } from './postgres-change.js';
import { realtimeChannel, type RealtimeChannelLike, type RealtimeClientLike } from './channel.js';

type Listener = { filter: { event: string; schema: string; table: string; filter?: string }; cb: (p: PostgresChangePayload) => void };

/** A supabase-js-shaped client that behaves as realtime-js 2.x does: `channel(topic)`
 *  hands back the open channel for a topic it already holds (`RealtimeClient.channel`),
 *  a listener added after `subscribe()` throws (`RealtimeChannel.on`), and removing a
 *  channel lets the topic open afresh. */
function fakeClient() {
  const channels: Array<{
    topic: string;
    listeners: Listener[];
    status: ((s: string, err?: Error) => void) | null;
    subscribed: boolean;
    handle?: unknown;
  }> = [];
  const removed: unknown[] = [];
  const client = {
    channel(topic: string) {
      const open = channels.find((c) => c.topic === topic && !removed.includes(c.handle));
      if (open) return open.handle as RealtimeChannelLike;
      const record = {
        topic,
        listeners: [] as Listener[],
        status: null as ((s: string, err?: Error) => void) | null,
        subscribed: false,
        handle: undefined as unknown,
      };
      channels.push(record);
      const channel: RealtimeChannelLike = {
        on(_type: 'postgres_changes', filter: Listener['filter'], cb: Listener['cb']) {
          if (record.subscribed) throw new Error(`cannot add \`postgres_changes\` callbacks for realtime:${topic} after \`subscribe()\`.`);
          record.listeners.push({ filter, cb });
          return channel;
        },
        subscribe(cb?: (s: string, err?: Error) => void) {
          record.subscribed = true;
          record.status = cb ?? null;
          return channel;
        },
      };
      record.handle = channel;
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
  new: { id, crew_id: 'crew-1', title: 'Live' },
  old: {},
});

const spec = { topic: 'relay:crew-1', schema: 'relay', crewId: 'crew-1' };

afterEach(() => vi.restoreAllMocks());

describe('realtimeChannel — a transport subscribe over a supabase-shaped client', () => {
  it('opens its own channel per subscribe, with one postgres_changes listener per relay table', () => {
    const f = fakeClient();
    const ch = realtimeChannel(f.client, spec);
    expect(f.channels).toHaveLength(0);
    ch.subscribe(vi.fn(), vi.fn());
    expect(f.channels).toHaveLength(1);
    expect(f.channels[0].topic.startsWith('relay:crew-1')).toBe(true);
    expect(f.channels[0].listeners.map((l) => l.filter.table).sort()).toEqual(
      ['actions', 'items', 'lanes', 'leases', 'runs', 'tasks'],
    );
    expect(f.channels[0].listeners.every((l) => l.filter.event === '*' && l.filter.schema === 'relay')).toBe(true);
    ch.subscribe(vi.fn(), vi.fn());
    expect(f.channels).toHaveLength(2);
    expect(f.channels[1].topic).not.toBe(f.channels[0].topic);
  });

  it('a second subscriber on the same client and topic opens, while the first is still subscribed', () => {
    // realtime-js hands back the subscribed channel for a topic it holds, and a listener
    // added to it throws: the crew view and the task view share the dōjō's client.
    const f = fakeClient();
    const first = vi.fn();
    const second = vi.fn();
    realtimeChannel(f.client, spec).subscribe(first, vi.fn());
    expect(() => realtimeChannel(f.client, spec).subscribe(second, vi.fn())).not.toThrow();
    f.change(0, lanePayload('l-1'));
    f.change(1, lanePayload('l-2'));
    expect(first.mock.calls.map((c) => c[0].data.lane_id)).toEqual(['l-1']);
    expect(second.mock.calls.map((c) => c[0].data.lane_id)).toEqual(['l-2']);
  });

  it('a re-subscribe before the last teardown finishes removing still opens', () => {
    const f = fakeClient();
    const ch = realtimeChannel(f.client, spec);
    const stop = ch.subscribe(vi.fn(), vi.fn());
    // removeChannel is async: an effect re-runs connect() before it lands.
    f.client.removeChannel.mockImplementationOnce(() => new Promise(() => {}));
    stop();
    expect(() => ch.subscribe(vi.fn(), vi.fn())).not.toThrow();
  });

  it('asks the server for the crew’s rows only, on each table whose rows all name a crew', () => {
    const f = fakeClient();
    realtimeChannel(f.client, spec).subscribe(vi.fn(), vi.fn());
    const filters = Object.fromEntries(f.channels[0].listeners.map((l) => [l.filter.table, l.filter.filter]));
    expect(filters).toEqual({
      lanes: 'crew_id=eq.crew-1',
      leases: 'crew_id=eq.crew-1',
      tasks: 'crew_id=eq.crew-1',
      actions: 'crew_id=eq.crew-1',
      // A draft run has no crew yet, and the queue is the person's, across crews.
      runs: undefined,
      items: undefined,
    });
  });

  it('drops a change whose row names another crew, under any table map', () => {
    const f = fakeClient();
    const on = vi.fn();
    const tables = { decisions: { entity: 'decision', screenRow: true as const }, runs: 'run' };
    realtimeChannel(f.client, { ...spec, tables }).subscribe(on, vi.fn());
    const decision = (crew_id: string) => ({ id: `ask-${crew_id}`, crew_id, question: 'Which?' });
    f.change(0, { schema: 'relay', table: 'decisions', eventType: 'INSERT', new: decision('crew-2'), old: {} });
    f.change(0, { schema: 'relay', table: 'runs', eventType: 'UPDATE', new: { id: 'r-2', crew_id: 'crew-2' }, old: {} });
    expect(on).not.toHaveBeenCalled();
    f.change(0, { schema: 'relay', table: 'decisions', eventType: 'INSERT', new: decision('crew-1'), old: {} });
    // A draft run names no crew: it is the person's, and passes.
    f.change(0, { schema: 'relay', table: 'runs', eventType: 'UPDATE', new: { id: 'r-3', crew_id: null }, old: {} });
    expect(on.mock.calls.map((c) => c[0].entity)).toEqual(['decision', 'run']);
  });

  it('a channel for every crew the person sees (crewId null) filters nothing', () => {
    const f = fakeClient();
    const on = vi.fn();
    realtimeChannel(f.client, { ...spec, crewId: null }).subscribe(on, vi.fn());
    expect(f.channels[0].listeners.every((l) => l.filter.filter === undefined)).toBe(true);
    f.change(0, { ...lanePayload(), new: { id: 'l-9', crew_id: 'crew-2' } });
    expect(on).toHaveBeenCalledTimes(1);
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
      data: { op: 'update', lane_id: 'l-1' },
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

  it('logs a channel that failed, timed out or closed, with the error supabase-js gave, and forwards it', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const f = fakeClient();
    const onStatus = vi.fn();
    realtimeChannel(f.client, spec).subscribe(vi.fn(), onStatus);
    const denied = new Error('permission denied for table lanes');
    f.channels[0].status?.('CHANNEL_ERROR', denied);
    expect(onStatus).toHaveBeenLastCalledWith('CHANNEL_ERROR', denied);
    expect(warn).toHaveBeenCalledTimes(1);
    expect(warn.mock.calls[0].map(String).join(' ')).toContain('relay:crew-1');
    expect(warn.mock.calls[0]).toContain(denied);
    f.channels[0].status?.('TIMED_OUT');
    f.channels[0].status?.('CLOSED');
    expect(warn).toHaveBeenCalledTimes(3);
    f.channels[0].status?.('SUBSCRIBED');
    expect(warn).toHaveBeenCalledTimes(3);
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
