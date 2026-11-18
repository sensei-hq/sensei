// The dōjō's relay source — the seam (#421; design 2026-11-12 §4). The dōjō reads the
// relay rows the desktop syncs to Supabase and listens to `postgres_changes`, mapped
// to the same StateEvent the daemon streams, so RelayState cannot tell the two apart.
// The channel half is `realtimeChannel` (./channel.ts) over the dōjō's supabase-js
// client; the reads and writes against the dōjō tables land with the dōjō epic (#427).
// Until then the transport is this interface and a canned double.

import type { ApiResult } from '../../result.js';
import type { RelaySource } from '../relay-state.svelte.js';
import type { RelayTaskSource } from '../task-state.svelte.js';
import { realtimeEvents, type ChannelStatus } from '../../source.js';
import type {
  RelayAddToBacklog,
  RelayAddTriage,
  RelayAnswer,
  RelayCancelledRun,
  RelayCreateRun,
  RelayCreatedRun,
  RelayRunList,
  RelayFilter,
  RelayNow,
  RelayQueue,
  RelaySendMessage,
  RelayStreamEvent,
  RelayTaskView,
} from '../types.js';

/** A realtime channel's status, as supabase-js reports it. */
export type RelayChannelStatus = ChannelStatus;

/** What the dōjō's transport provides: the reads and writes, and a channel of relay changes. */
export interface RelayRealtimeTransport {
  readNow(filter: RelayFilter): Promise<ApiResult<RelayNow>>;
  readQueue(filter: RelayFilter): Promise<ApiResult<RelayQueue>>;
  /** One task's view (#414): what the task page reads. */
  readTask(crewId: string, taskId: string): Promise<ApiResult<RelayTaskView>>;
  answer(body: RelayAnswer): Promise<ApiResult<{ action_id: string }>>;
  send(body: RelaySendMessage): Promise<ApiResult<{ message_id: string }>>;
  addToBacklog(body: RelayAddToBacklog): Promise<ApiResult<{ item_id: string }>>;
  addTriage(body: RelayAddTriage): Promise<ApiResult<{ item_id: string }>>;
  createRun(body: RelayCreateRun): Promise<ApiResult<RelayCreatedRun>>;
  readRuns(): Promise<ApiResult<RelayRunList>>;
  startRun(runId: string, crewId: string | null): Promise<ApiResult<RelayCreatedRun>>;
  cancelRun(runId: string): Promise<ApiResult<RelayCancelledRun>>;
  /** Subscribe to the channel: each change as a StateEvent, and each status change with
   *  the error supabase-js gave, if any. Returns the teardown. */
  subscribe(
    on: (event: RelayStreamEvent) => void,
    onStatus: (status: RelayChannelStatus, err?: Error) => void,
  ): () => void;
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
    readRuns: () => transport.readRuns(),
    startRun: (runId, crewId) => transport.startRun(runId, crewId),
    cancelRun: (runId) => transport.cancelRun(runId),
    events: realtimeEvents(transport),
  };
}

/** A RelayTaskSource over the dōjō's transport (#414): one task's read and answer, and
 *  the channel, where each SUBSCRIBED is an open. */
export function relayRealtimeTaskSource(
  transport: Pick<RelayRealtimeTransport, 'readTask' | 'answer' | 'subscribe'>,
): RelayTaskSource {
  return {
    readTask: (crewId, taskId) => transport.readTask(crewId, taskId),
    answer: (body) => transport.answer(body),
    events: realtimeEvents(transport),
  };
}

/** One write the canned transport recorded. */
export type CannedWrite =
  | { kind: 'answer'; body: RelayAnswer }
  | { kind: 'send'; body: RelaySendMessage }
  | { kind: 'backlog'; body: RelayAddToBacklog }
  | { kind: 'triage'; body: RelayAddTriage }
  | { kind: 'run'; body: RelayCreateRun }
  | { kind: 'start'; runId: string; crewId: string | null }
  | { kind: 'cancel'; runId: string };

/** A transport for specs and the bench: canned reads, recorded writes, and a channel the test drives. */
export function cannedRealtime(reads: { now: RelayNow | null; queue: RelayQueue | null; task?: RelayTaskView | null }) {
  let handler: ((event: RelayStreamEvent) => void) | null = null;
  let statusHandler: ((status: RelayChannelStatus, err?: Error) => void) | null = null;
  const writes: CannedWrite[] = [];
  const unread = { ok: false as const, error: { status: 503, message: 'not synced from the desktop yet' } };
  const ok = <T>(data: T): ApiResult<T> => ({ ok: true, data });
  return {
    writes,
    readNow: async (): Promise<ApiResult<RelayNow>> => (reads.now ? ok(reads.now) : unread),
    readQueue: async (): Promise<ApiResult<RelayQueue>> => (reads.queue ? ok(reads.queue) : unread),
    readTask: async (_crewId: string, _taskId: string): Promise<ApiResult<RelayTaskView>> =>
      reads.task ? ok(reads.task) : unread,
    answer: async (body: RelayAnswer) => (writes.push({ kind: 'answer', body }), ok({ action_id: `canned-${writes.length}` })),
    send: async (body: RelaySendMessage) => (writes.push({ kind: 'send', body }), ok({ message_id: `canned-${writes.length}` })),
    addToBacklog: async (body: RelayAddToBacklog) => (writes.push({ kind: 'backlog', body }), ok({ item_id: `canned-${writes.length}` })),
    addTriage: async (body: RelayAddTriage) => (writes.push({ kind: 'triage', body }), ok({ item_id: `canned-${writes.length}` })),
    createRun: async (body: RelayCreateRun): Promise<ApiResult<RelayCreatedRun>> => {
      writes.push({ kind: 'run', body });
      return { ok: false, error: { status: 501, message: 'the canned transport mints no runs' } };
    },
    readRuns: async (): Promise<ApiResult<RelayRunList>> => unread,
    startRun: async (runId: string, crewId: string | null): Promise<ApiResult<RelayCreatedRun>> => {
      writes.push({ kind: 'start', runId, crewId });
      return { ok: false, error: { status: 501, message: 'the canned transport starts no runs' } };
    },
    cancelRun: async (runId: string): Promise<ApiResult<RelayCancelledRun>> => {
      writes.push({ kind: 'cancel', runId });
      return { ok: false, error: { status: 501, message: 'the canned transport cancels no runs' } };
    },
    subscribe(on: (event: RelayStreamEvent) => void, onStatus: (status: RelayChannelStatus, err?: Error) => void) {
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
    /** Report a channel status, and its error, as the channel would. */
    status(status: RelayChannelStatus, err?: Error) {
      statusHandler?.(status, err);
    },
  } satisfies RelayRealtimeTransport & Record<string, unknown>;
}
