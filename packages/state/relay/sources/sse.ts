// The daemon's relay source (#421; design 2026-11-12 §2b): the relay client for reads
// and writes, and the relay socket (`/api/relay/events`) for the stream. A broadcast
// has no replay, so the socket's first open is the connect and each later open a
// reconnect, which re-reads (the state decides what).

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
    createRun: (body) => api.tryCreateRun(body),
    events(on, onReconnect) {
      const socket = stream();
      let opened = false;
      socket.onopen = () => {
        if (opened) onReconnect();
        opened = true;
      };
      const unsubscribe = socket.subscribe(on);
      return () => {
        unsubscribe();
        socket.destroy();
      };
    },
  };
}
