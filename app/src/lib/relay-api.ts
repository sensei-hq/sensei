// Relay · the desktop's client contract for the daemon's relay API (D90, slice 1 item 4).
//
// The shapes came first (phase A), so the screen's tests can mock the client; the
// runtime factory below is phase B's. The screen reaches the daemon through it alone.
//
// Paths: the daemon's relay routes are L3's to publish (ruling Q9). Until they are,
// these are the ones the screen asks for — `relay/now`, `relay/queue`, `relay/tasks/
// {crew}/{task}`, and the three writes — and a change on the daemon's side is a change
// here, in one file.
//
// Shapes follow `docs/prd/modules/03-delivery/21-relay/lanes-data.md` (the views
// `relay.now` and `relay.task_history`, and the action kinds) and the interaction
// contract `coord/mockups/relay-lanes/CONTRACT.md` §3.
//
// Repository required, workspace optional (lanes-data, user 2026-11-05): every task names its
// `repo_key`; `workspace_id` is a tag and a filter, never a gate. Reads are scoped by the user and
// the crew. The main window reads every crew; the workspace window reads the same views filtered
// to its workspace.
//
// D83: every self-reported value is nullable. `null` means "not reported" and the
// screen says so; it is never coerced to 0. Times are ISO instants; the screen
// formats them.

import { apiBase, type ApiResult } from './api.js';
import { EventManager } from './events.js';
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
} from '@sensei/state';

// The wire shapes live in @sensei/state (#421); every importer of this client still finds them here.
export type * from '@sensei/state';
export { parseRelayStreamEvent };

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

/** What the screens build once from the daemon's port. */
export type RelayApiFactory = (port: number) => RelayApi;

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
export const relayApi: RelayApiFactory = (port) => {
  const base = apiBase(port);

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

/** The relay socket, owned by the route that subscribes (never by shared state). */
export const relayStream: (port: number) => EventManager<RelayStreamEvent> = (port) =>
  new EventManager<RelayStreamEvent>(`${apiBase(port)}/api/relay/events`, parseRelayStreamEvent);
