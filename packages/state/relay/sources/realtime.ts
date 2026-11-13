// The dōjō's relay source — the seam (#421; design 2026-11-12 §4). The dōjō reads the
// relay rows the desktop syncs to Supabase and listens to `postgres_changes`, mapped
// to the same StateEvent the daemon streams, so RelayState cannot tell the two apart.
// The supabase-js transport lands with the dōjō epic (#427); until then the seam is
// this interface and a canned double.

import type { ApiResult } from '../../result.js';
import type { RelaySource } from '../relay-state.svelte.js';
import type {
  RelayAddToBacklog,
  RelayAddTriage,
  RelayAnswer,
  RelayCreateRun,
  RelayCreatedRun,
  RelayFilter,
  RelayNow,
  RelayQueue,
  RelaySendMessage,
  RelayStreamEvent,
} from '../types.js';

/** A realtime channel's status, as supabase-js reports it. */
export type RelayChannelStatus = 'SUBSCRIBED' | 'TIMED_OUT' | 'CLOSED' | 'CHANNEL_ERROR';

/** What the dōjō's transport provides: the reads and writes, and a channel of relay changes. */
export interface RelayRealtimeTransport {
  readNow(filter: RelayFilter): Promise<ApiResult<RelayNow>>;
  readQueue(filter: RelayFilter): Promise<ApiResult<RelayQueue>>;
  answer(body: RelayAnswer): Promise<ApiResult<{ action_id: string }>>;
  send(body: RelaySendMessage): Promise<ApiResult<{ message_id: string }>>;
  addToBacklog(body: RelayAddToBacklog): Promise<ApiResult<{ item_id: string }>>;
  addTriage(body: RelayAddTriage): Promise<ApiResult<{ item_id: string }>>;
  createRun(body: RelayCreateRun): Promise<ApiResult<RelayCreatedRun>>;
  /** Subscribe to the channel: each change as a StateEvent, and each status change. Returns the teardown. */
  subscribe(on: (event: RelayStreamEvent) => void, onStatus: (status: RelayChannelStatus) => void): () => void;
}

/** A RelaySource over the dōjō's transport. Each SUBSCRIBED — the first, and again after a drop — is an open. */
export function relayRealtimeSource(transport: RelayRealtimeTransport): RelaySource {
  return {
    readNow: (filter) => transport.readNow(filter),
    readQueue: (filter) => transport.readQueue(filter),
    answer: (body) => transport.answer(body),
    send: (body) => transport.send(body),
    addToBacklog: (body) => transport.addToBacklog(body),
    addTriage: (body) => transport.addTriage(body),
    createRun: (body) => transport.createRun(body),
    events(on, onOpen) {
      return transport.subscribe(on, (status) => {
        if (status === 'SUBSCRIBED') onOpen();
      });
    },
  };
}

/** One write the canned transport recorded. */
export type CannedWrite =
  | { kind: 'answer'; body: RelayAnswer }
  | { kind: 'send'; body: RelaySendMessage }
  | { kind: 'backlog'; body: RelayAddToBacklog }
  | { kind: 'triage'; body: RelayAddTriage }
  | { kind: 'run'; body: RelayCreateRun };

/** A transport for specs and the bench: canned reads, recorded writes, and a channel the test drives. */
export function cannedRealtime(reads: { now: RelayNow | null; queue: RelayQueue | null }) {
  let handler: ((event: RelayStreamEvent) => void) | null = null;
  let statusHandler: ((status: RelayChannelStatus) => void) | null = null;
  const writes: CannedWrite[] = [];
  const unread = { ok: false as const, error: { status: 503, message: 'not synced from the desktop yet' } };
  const ok = <T>(data: T): ApiResult<T> => ({ ok: true, data });
  return {
    writes,
    readNow: async (): Promise<ApiResult<RelayNow>> => (reads.now ? ok(reads.now) : unread),
    readQueue: async (): Promise<ApiResult<RelayQueue>> => (reads.queue ? ok(reads.queue) : unread),
    answer: async (body: RelayAnswer) => (writes.push({ kind: 'answer', body }), ok({ action_id: `canned-${writes.length}` })),
    send: async (body: RelaySendMessage) => (writes.push({ kind: 'send', body }), ok({ message_id: `canned-${writes.length}` })),
    addToBacklog: async (body: RelayAddToBacklog) => (writes.push({ kind: 'backlog', body }), ok({ item_id: `canned-${writes.length}` })),
    addTriage: async (body: RelayAddTriage) => (writes.push({ kind: 'triage', body }), ok({ item_id: `canned-${writes.length}` })),
    createRun: async (body: RelayCreateRun): Promise<ApiResult<RelayCreatedRun>> => {
      writes.push({ kind: 'run', body });
      return { ok: false, error: { status: 501, message: 'the canned transport mints no runs' } };
    },
    subscribe(on: (event: RelayStreamEvent) => void, onStatus: (status: RelayChannelStatus) => void) {
      handler = on;
      statusHandler = onStatus;
      return () => {
        handler = null;
        statusHandler = null;
      };
    },
    /** Deliver a change, as the channel would. */
    emit(event: RelayStreamEvent) {
      handler?.(event);
    },
    /** Report a channel status, as the channel would. */
    status(status: RelayChannelStatus) {
      statusHandler?.(status);
    },
  } satisfies RelayRealtimeTransport & Record<string, unknown>;
}
