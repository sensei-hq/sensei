// The daemon's relay source (#421; design 2026-11-12 §2b): the relay client for reads
// and writes, and the relay socket (`/api/relay/events`) for the stream. A broadcast
// has no replay, so every open is reported — the first and each reconnect — and the
// state reads on it: nothing before the socket opened is missed.

import type { RelayApi } from '../client.js';
import type { RelaySource } from '../relay-state.svelte.js';
import type { RelayStreamEvent } from '../types.js';

/** What the source needs of a socket: the EventManager's shape. */
export interface RelayEventStream {
  onopen: (() => void) | null;
  subscribe(handler: (event: RelayStreamEvent) => void): () => void;
  destroy(): void;
}

/** A RelaySource over the daemon: `api` for reads and writes, `stream()` for a fresh socket per subscription. */
export function relaySseSource(api: RelayApi, stream: () => RelayEventStream): RelaySource {
  return {
    readNow: (filter) => api.tryGetNow(filter),
    readQueue: (filter) => api.tryGetQueue(filter),
    answer: (body) => api.tryAnswerDecision(body),
    send: (body) => api.trySendMessage(body),
    addToBacklog: (body) => api.tryAddToBacklog(body),
    addTriage: (body) => api.tryAddTriage(body),
    createRun: (body) => api.tryCreateRun(body),
    events(on, onOpen) {
      const socket = stream();
      socket.onopen = () => onOpen();
      const unsubscribe = socket.subscribe(on);
      return () => {
        unsubscribe();
        socket.destroy();
      };
    },
  };
}
