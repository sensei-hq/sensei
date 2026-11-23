// The stream contract every state source produces (#414, design D1): subscribe with a
// handler for every event and one for every open; get the teardown back. Nothing here
// is relay's — a test event `{id, n}` rides the same adapters.
import { describe, it, expect, vi } from 'vitest';
import { sseEvents, realtimeEvents, type ChannelStatus, type EventStream, type StateStream } from './source.js';

interface Tick {
  id: string;
  n: number;
}

/** A socket the test opens and fires by hand (the EventManager's shape). */
function fakeSocket() {
  let handler: ((e: Tick) => void) | null = null;
  const socket: EventStream<Tick> & { open(): void; fail(): void; fire(e: Tick): void; destroyed: number; unsubscribed: number } = {
    onopen: null,
    onerror: null,
    destroyed: 0,
    unsubscribed: 0,
    subscribe(h) {
      handler = h;
      return () => {
        handler = null;
        socket.unsubscribed += 1;
      };
    },
    destroy() {
      socket.destroyed += 1;
    },
    open() {
      socket.onopen?.();
    },
    fail() {
      socket.onerror?.();
    },
    fire(e) {
      handler?.(e);
    },
  };
  return socket;
}

/** A realtime transport the test drives: changes and channel statuses. */
function fakeChannel() {
  let on: ((e: Tick) => void) | null = null;
  let onStatus: ((s: ChannelStatus, err?: Error) => void) | null = null;
  return {
    stopped: 0,
    subscribe(handler: (e: Tick) => void, statusHandler: (s: ChannelStatus, err?: Error) => void) {
      on = handler;
      onStatus = statusHandler;
      return () => {
        on = null;
        onStatus = null;
        this.stopped += 1;
      };
    },
    emit: (e: Tick) => on?.(e),
    status: (s: ChannelStatus, err?: Error) => onStatus?.(s, err),
  };
}

describe('sseEvents — a StateStream over a socket, for any event type', () => {
  it('passes every event on, reports every open, and the teardown unsubscribes and destroys the socket', () => {
    const socket = fakeSocket();
    const stream: StateStream<Tick> = sseEvents<Tick>(() => socket);
    const seen: Tick[] = [];
    const opened = vi.fn();
    const stop = stream((e) => seen.push(e), opened);

    socket.open();
    socket.fire({ id: 'a', n: 1 });
    socket.open();
    socket.fire({ id: 'b', n: 2 });

    expect(seen).toEqual([
      { id: 'a', n: 1 },
      { id: 'b', n: 2 },
    ]);
    expect(opened).toHaveBeenCalledTimes(2);

    stop();
    expect(socket.unsubscribed).toBe(1);
    expect(socket.destroyed).toBe(1);
    socket.fire({ id: 'c', n: 3 });
    expect(seen).toHaveLength(2);
  });

  it('opens a fresh socket per subscription', () => {
    const make = vi.fn(() => fakeSocket());
    const stream = sseEvents<Tick>(make);
    stream(() => {}, () => {})();
    stream(() => {}, () => {})();
    expect(make).toHaveBeenCalledTimes(2);
  });

  it('reports socket errors as faults', () => {
    const socket = fakeSocket();
    const stream: StateStream<Tick> = sseEvents<Tick>(() => socket);
    const faulted = vi.fn();
    const stop = stream(
      () => {},
      () => {},
      faulted,
    );
    socket.fail();
    expect(faulted).toHaveBeenCalledWith('the live stream dropped; reconnecting');
    stop();
  });
});

describe('realtimeEvents — a StateStream over a channel, for any event type', () => {
  it('counts each SUBSCRIBED as an open, and nothing else', () => {
    const channel = fakeChannel();
    const stream: StateStream<Tick> = realtimeEvents<Tick>(channel);
    const seen: Tick[] = [];
    const opened = vi.fn();
    const stop = stream((e) => seen.push(e), opened);

    channel.status('SUBSCRIBED');
    channel.emit({ id: 'a', n: 1 });
    channel.status('CHANNEL_ERROR');
    channel.status('SUBSCRIBED');
    channel.emit({ id: 'b', n: 2 });

    expect(opened).toHaveBeenCalledTimes(2);
    expect(seen.map((e) => e.n)).toEqual([1, 2]);

    stop();
    expect(channel.stopped).toBe(1);
    channel.emit({ id: 'c', n: 3 });
    expect(seen).toHaveLength(2);
  });

  it('reports a channel that failed, timed out or closed as a fault, with the reason; a SUBSCRIBED is not one', () => {
    const channel = fakeChannel();
    const faults: string[] = [];
    realtimeEvents<Tick>(channel)(() => {}, () => {}, (reason) => faults.push(reason));
    channel.status('SUBSCRIBED');
    channel.status('CHANNEL_ERROR', new Error('JWT expired'));
    channel.status('TIMED_OUT');
    channel.status('CLOSED');
    expect(faults).toHaveLength(3);
    expect(faults[0]).toContain('JWT expired');
    expect(faults[1]).toMatch(/timed out/);
    expect(faults[2]).toMatch(/closed/);
  });
});
