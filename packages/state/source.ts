// The stream contract every state source produces (#414, design D1), whatever the domain.
// A source's stream is subscribed with a handler for every event and one for every open
// (the first, and each reconnect: a broadcast has no replay, so the state re-reads on
// each); it returns the teardown. The daemon's SSE and the dōjō's realtime channel both
// produce it.

/** Subscribe with a handler for every event and one for every open; returns the teardown. */
export type StateStream<E> = (on: (event: E) => void, onOpen: () => void) => () => void;

/** What an SSE stream needs of a socket: the EventManager's shape. */
export interface EventStream<E> {
  onopen: (() => void) | null;
  subscribe(handler: (event: E) => void): () => void;
  destroy(): void;
}

/** A realtime channel's status, as supabase-js reports it. */
export type ChannelStatus = 'SUBSCRIBED' | 'TIMED_OUT' | 'CLOSED' | 'CHANNEL_ERROR';

/** What a realtime stream needs of a channel: each change, and each status change. Returns the teardown. */
export interface ChannelSubscriber<E> {
  subscribe(on: (event: E) => void, onStatus: (status: ChannelStatus) => void): () => void;
}

/** A stream over SSE: a fresh socket per subscription, every open reported. */
export function sseEvents<E>(stream: () => EventStream<E>): StateStream<E> {
  return (on, onOpen) => {
    const socket = stream();
    socket.onopen = () => onOpen();
    const unsubscribe = socket.subscribe(on);
    return () => {
      unsubscribe();
      socket.destroy();
    };
  };
}

/** A stream over a realtime channel: each SUBSCRIBED (the first, and again after a drop) is an open. */
export function realtimeEvents<E>(channel: ChannelSubscriber<E>): StateStream<E> {
  return (on, onOpen) =>
    channel.subscribe(on, (status) => {
      if (status === 'SUBSCRIBED') onOpen();
    });
}
