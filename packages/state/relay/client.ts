// The relay client: the daemon's relay routes, by base URL (#421). The desktop app
// builds it from the daemon's port (`relayApi(port)` in the app's relay-api.ts);
// the paths are pinned there, in its spec, so a route change is a loud one.
import { EventManager } from '../event-manager.js';
import type { ApiResult } from '../result.js';
import {
  parseRelayStreamEvent,
  type RelayAddToBacklog,
  type RelayAnswer,
  type RelayCreateRun,
  type RelayCreatedRun,
  type RelayFilter,
  type RelayNow,
  type RelayQueue,
  type RelaySendMessage,
  type RelayStreamEvent,
  type RelayTaskView,
} from './types.js';

// ─── The client ─────────────────────────────────────────────────────────────

export interface RelayApi {
  tryGetNow(filter: RelayFilter): Promise<ApiResult<RelayNow>>;
  tryGetQueue(filter: RelayFilter): Promise<ApiResult<RelayQueue>>;
  /** Scoped by (user, crew, task): another user's task, or one from a crew you're not in, is a 404. */
  tryGetTask(crewId: string, taskId: string): Promise<ApiResult<RelayTaskView>>;
  trySendMessage(body: RelaySendMessage): Promise<ApiResult<{ message_id: string }>>;
  tryAnswerDecision(body: RelayAnswer): Promise<ApiResult<{ action_id: string }>>;
  tryAddToBacklog(body: RelayAddToBacklog): Promise<ApiResult<{ item_id: string }>>;
  /** Mint a sprint run (queue cutover); the daemon's refusal text says why not. */
  tryCreateRun(body: RelayCreateRun): Promise<ApiResult<RelayCreatedRun>>;
}

/** `?workspace_id=…&repo_key=…`, empty when nothing narrows the read. */
const filterQuery = (filter: RelayFilter): string => {
  const params = new URLSearchParams();
  if (filter.workspace_id) params.set('workspace_id', filter.workspace_id);
  if (filter.repo_key) params.set('repo_key', filter.repo_key);
  const q = params.toString();
  return q ? `?${q}` : '';
};

/** The relay client: every call the daemon's answer or an `ApiResult` error, the
 *  daemon's own refusal text when it sends one (a 409's reason is the useful part). */
export const relayClient = (base: string): RelayApi => {

  async function call<T>(method: 'GET' | 'POST', path: string, body?: unknown): Promise<ApiResult<T>> {
    try {
      const res = await fetch(`${base}${path}`, {
        method,
        ...(body === undefined ? {} : { headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) }),
      });
      if (res.status === 204) return { ok: true, data: undefined as T };
      if (res.ok) return { ok: true, data: (await res.json()) as T };
      try {
        const parsed = (await res.json()) as Record<string, unknown> | null;
        const reason = parsed && typeof parsed.error === 'string' && parsed.error ? parsed.error : res.statusText;
        return { ok: false, error: { status: res.status, message: reason } };
      } catch {
        return { ok: false, error: { status: res.status, message: res.statusText } };
      }
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }

  return {
    tryGetNow: (filter) => call<RelayNow>('GET', `/relay/now${filterQuery(filter)}`),
    tryGetQueue: (filter) => call<RelayQueue>('GET', `/relay/queue${filterQuery(filter)}`),
    tryGetTask: (crewId, taskId) =>
      call<RelayTaskView>('GET', `/relay/tasks/${encodeURIComponent(crewId)}/${encodeURIComponent(taskId)}`),
    trySendMessage: (body) => call<{ message_id: string }>('POST', '/relay/messages', body),
    tryAnswerDecision: (body) => call<{ action_id: string }>('POST', '/relay/answers', body),
    tryAddToBacklog: (body) => call<{ item_id: string }>('POST', '/relay/backlog', body),
    tryCreateRun: (body) => call<RelayCreatedRun>('POST', '/relay/runs', body),
  };
};

// ─── Stream (F8; the dojo seam) ─────────────────────────────────────────────

/** The relay socket at `base`. */
export const relayEvents = (base: string): EventManager<RelayStreamEvent> =>
  new EventManager<RelayStreamEvent>(`${base}/api/relay/events`, parseRelayStreamEvent);
