// The daemon's ops sources: the ops client for reads, the task socket
// (`/api/tasks/progress`) for the stream. One client serves all three
// states; each state takes its own socket so attach lifecycles stay
// independent. The schedules list unwraps `{ tasks }`, the sync rows
// `{ entities }` — the endpoints' envelopes, not the stores' rows.
import type { OpsApi } from '../client.js';
import type { JobsSource } from '../jobs-state.svelte.js';
import type { SchedulesSource } from '../schedules-state.svelte.js';
import type { SyncSource } from '../sync-state.svelte.js';
import type { OpsScheduledTask, OpsSyncStateRow } from '../types.js';
import type { StreamFrame } from '../../events.js';
import { sseEvents, type EventStream } from '../../source.js';
import type { ApiResult } from '../../result.js';

/** What a source needs of a socket: the EventManager's shape, carrying ops frames. */
export type OpsEventStream = EventStream<StreamFrame>;

/** A JobsSource over the daemon: `api` for the queue read, `stream()` for a fresh socket per subscription. */
export function jobsSseSource(api: OpsApi, stream: () => OpsEventStream): JobsSource {
  return {
    readQueue: () => api.tryGetQueue(),
    events: sseEvents(stream),
  };
}

/** A SchedulesSource over the daemon, unwrapping the `{ tasks }` envelope. */
export function schedulesSseSource(api: OpsApi, stream: () => OpsEventStream): SchedulesSource {
  return {
    readSchedules: async (): Promise<ApiResult<OpsScheduledTask[]>> => {
      const read = await api.tryGetSchedules();
      if (!read.ok) return read;
      return { ok: true, data: read.data.tasks };
    },
    events: sseEvents(stream),
  };
}

/** A SyncSource over the daemon, unwrapping the `{ entities }` envelope. */
export function syncSseSource(api: OpsApi, stream: () => OpsEventStream): SyncSource {
  return {
    readSyncState: async (): Promise<ApiResult<OpsSyncStateRow[]>> => {
      const read = await api.tryGetSyncState();
      if (!read.ok) return read;
      return { ok: true, data: read.data.entities };
    },
    events: sseEvents(stream),
  };
}
