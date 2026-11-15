// Relay · the crew screen's socket (queue cutover, phase 4): transport owned
// by the route, never by shared state — the task route's precedent
// (`relay-task-stream.ts`). The dispatcher is pure: a run event moves the
// band and the queue (items planned), a lane or task event moves Now (the
// lanes, Needs you, the review queue), a step is the task route's business.
// The screen re-reads; it never applies a row the daemon did not answer.
import { relayStream, type RelayStreamEvent } from '$lib/relay-api.js';
import type { RelayState } from './relay-state.svelte.js';

/** Which reads one socket event invalidates. */
export function crewEventReReads(event: RelayStreamEvent): { now: boolean; queue: boolean } {
  switch (event.entity) {
    case 'run':
      return { now: true, queue: true };
    case 'lane':
    case 'task':
    case 'relay':
      return { now: true, queue: false };
    default:
      return { now: false, queue: false };
  }
}

/** Subscribe the crew route; returns the teardown (call on destroy). */
export function subscribeCrewStream(port: number, state: RelayState): () => void {
  const stream = relayStream(port);
  // Broadcast has no replay: the first open is the initial connect, later
  // opens are reconnects and re-read both halves.
  let primed = false;
  stream.onopen = () => {
    if (!primed) {
      primed = true;
      return;
    }
    void state.reReadNow();
    void state.reReadQueue();
  };
  const unsub = stream.subscribe((event) => {
    const reads = crewEventReReads(event);
    if (reads.now) void state.reReadNow();
    if (reads.queue) void state.reReadQueue();
  });
  // Cover the loader-to-subscribe gap: the loader read before the socket existed.
  void state.reReadNow();
  return () => {
    unsub();
    stream.destroy();
  };
}
