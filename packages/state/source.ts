// The stream contract every state source produces (#414, design D1), whatever the domain.
// A source's stream is subscribed with a handler for every event and one for every open
// (the first, and each reconnect: a broadcast has no replay, so the state re-reads on
// each); it returns the teardown. The daemon's SSE and the dōjō's realtime channel both
// produce it.

/** Subscribe with a handler for every event, one for every open, and optionally one for
 *  every fault (the stream is down, so what the state holds may be stale); returns the
 *  teardown. The next open ends a fault. */
export type StateStream<E> = (
  on: (event: E) => void,
  onOpen: () => void,
  onFault?: (reason: string) => void,
) => () => void;

/** What an SSE stream needs of a socket: the EventManager's shape. */
export interface EventStream<E> {
  onopen: (() => void) | null;
  onerror: (() => void) | null;
  subscribe(handler: (event: E) => void): () => void;
  destroy(): void;
}

/** A realtime channel's status, as supabase-js reports it. */
export type ChannelStatus = 'SUBSCRIBED' | 'TIMED_OUT' | 'CLOSED' | 'CHANNEL_ERROR';

/** What a realtime stream needs of a channel: each change, and each status change with
 *  the error supabase-js gave, if any. Returns the teardown. */
export interface ChannelSubscriber<E> {
  subscribe(on: (event: E) => void, onStatus: (status: ChannelStatus, err?: Error) => void): () => void;
}

/** Why a channel status means the stream is down; null for SUBSCRIBED. */
export function channelFault(status: ChannelStatus, err?: Error): string | null {
  const detail = err?.message ? `: ${err.message}` : '';
  switch (status) {
    case 'SUBSCRIBED':
      return null;
    case 'CHANNEL_ERROR':
      return `the live channel failed${detail}`;
    case 'TIMED_OUT':
      return `the live channel timed out joining${detail}`;
    case 'CLOSED':
      return `the live channel closed${detail}`;
  }
}

/** A stream over SSE: a fresh socket per subscription, every open reported,
 *  every socket error reported as a fault (until the next open ends it). */
export function sseEvents<E>(stream: () => EventStream<E>): StateStream<E> {
  return (on, onOpen, onFault) => {
    const socket = stream();
    socket.onopen = () => onOpen();
    socket.onerror = () => onFault?.('the live stream dropped; reconnecting');
    const unsubscribe = socket.subscribe(on);
    return () => {
      unsubscribe();
      socket.destroy();
    };
  };
}

/** A stream over a realtime channel: each SUBSCRIBED (the first, and again after a drop)
 *  is an open; a failure, a timeout or a close is a fault, with its reason. */
export function realtimeEvents<E>(channel: ChannelSubscriber<E>): StateStream<E> {
  return (on, onOpen, onFault) =>
    channel.subscribe(on, (status, err) => {
      const fault = channelFault(status, err);
      if (fault === null) onOpen();
      else onFault?.(fault);
    });
}
