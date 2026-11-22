// The ops client: the daemon's operations routes, by base URL. One client
// for the ops family (queue, feeds, schedules, sync) rather than three
// near-identical ones. The paths are pinned in client.spec.ts.
import { EventManager } from '../event-manager.js';
import { failureFrom, type ApiResult } from '../result.js';
import type { StreamFrame } from '../events.js';
import type {
  OpsAbandonedFeed,
  OpsAlertAck,
  OpsBackfillResult,
  OpsFailedTask,
  OpsIndexError,
  OpsQueuedTaskDetail,
  OpsQueueListing,
  OpsScheduleLog,
  OpsSchedulePatch,
  OpsScheduledTask,
  OpsSyncStateFeed,
  OpsTaskDetail,
} from './types.js';

// ─── The client ─────────────────────────────────────────────────────────────

export interface OpsApi {
  tryGetQueue: (limit?: number) => Promise<ApiResult<OpsQueueListing>>;
  tryGetFailed: (days?: number, limit?: number) => Promise<ApiResult<{ tasks: OpsFailedTask[] }>>;
  tryGetAbandoned: (days?: number, limit?: number) => Promise<ApiResult<OpsAbandonedFeed>>;
  tryGetIndexErrors: () => Promise<ApiResult<OpsIndexError[]>>;
  tryGetSchedules: () => Promise<ApiResult<{ tasks: OpsScheduledTask[] }>>;
  tryPatchSchedule: (name: string, patch: OpsSchedulePatch) => Promise<ApiResult<OpsScheduledTask>>;
  tryGetScheduleLog: (name: string, lines?: number) => Promise<ApiResult<OpsScheduleLog>>;
  tryGetSyncState: () => Promise<ApiResult<OpsSyncStateFeed>>;
  tryGetTask: (id: number) => Promise<ApiResult<OpsTaskDetail | OpsQueuedTaskDetail>>;
  tryRetryTask: (id: number) => Promise<ApiResult<{ taskId: number }>>;
  tryStartBackfill: () => Promise<ApiResult<OpsBackfillResult>>;
  tryGetAlertAcks: () => Promise<ApiResult<{ acks: OpsAlertAck[] }>>;
  tryPutAlertAck: (cls: string, key: string, acked: boolean) => Promise<ApiResult<OpsAlertAck>>;
}

/** The ops client: every call the daemon's answer or an `ApiResult` error,
 *  the daemon's own refusal text when it sends one. */
export const opsClient = (base: string): OpsApi => {
  async function call<T>(
    method: 'GET' | 'PATCH' | 'POST' | 'PUT',
    path: string,
    body?: unknown,
  ): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method,
        ...(body === undefined
          ? {}
          : { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),
      });
      if (res.status === 204) return { ok: true, data: undefined as T };
      if (res.ok) return { ok: true, data: (await res.json()) as T };
      return { ok: false, error: await failureFrom(res) };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  return {
    tryGetQueue: (limit = 200) => call<OpsQueueListing>('GET', `/api/tasks/queue?limit=${limit}`),
    tryGetFailed: (days = 7, limit = 100) =>
      call<{ tasks: OpsFailedTask[] }>('GET', `/api/tasks/failed?days=${days}&limit=${limit}`),
    tryGetAbandoned: (days = 7, limit = 50) =>
      call<OpsAbandonedFeed>('GET', `/api/tasks/abandoned?days=${days}&limit=${limit}`),
    tryGetIndexErrors: () => call<OpsIndexError[]>('GET', '/api/index/errors'),
    tryGetSchedules: () => call<{ tasks: OpsScheduledTask[] }>('GET', '/api/tasks/scheduled'),
    tryPatchSchedule: (name, patch) =>
      call<OpsScheduledTask>('PATCH', `/api/tasks/scheduled/${encodeURIComponent(name)}`, patch),
    tryGetScheduleLog: (name, lines = 50) =>
      call<OpsScheduleLog>(
        'GET',
        `/api/tasks/scheduled/${encodeURIComponent(name)}/log?lines=${lines}`,
      ),
    tryGetSyncState: () => call<OpsSyncStateFeed>('GET', '/api/dojo/sync-state'),
    tryGetTask: (id) => call<OpsTaskDetail | OpsQueuedTaskDetail>('GET', `/api/tasks/${id}`),
    tryRetryTask: (id) => call<{ taskId: number }>('POST', `/api/tasks/${id}/retry`),
    tryStartBackfill: () => call<OpsBackfillResult>('POST', '/api/transcripts/backfill'),
    tryGetAlertAcks: () => call<{ acks: OpsAlertAck[] }>('GET', '/api/ops/alerts/acks'),
    tryPutAlertAck: (cls, key, acked) =>
      call<OpsAlertAck>('PUT', '/api/ops/alerts/acks', { class: cls, key, acked }),
  };
};

// ─── Stream ─────────────────────────────────────────────────────────────────

/** A validated task-stream frame: the uniform action/entity contract. */
export function parseOpsFrame(raw: string): StreamFrame {
  const parsed: unknown = JSON.parse(raw);
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    throw new Error('ops stream event is not an object');
  }
  const { action, entity, data } = parsed as Record<string, unknown>;
  if (typeof entity !== 'string' || typeof data !== 'object' || data === null || Array.isArray(data)) {
    throw new Error('ops stream event names no entity');
  }
  if (action !== 'add' && action !== 'update' && action !== 'remove' && action !== 'set') {
    throw new Error('ops stream event names no action');
  }
  return parsed as StreamFrame;
}

export const opsEvents = (base: string): EventManager<StreamFrame> =>
  new EventManager<StreamFrame>(`${base}/api/tasks/progress`, parseOpsFrame);
