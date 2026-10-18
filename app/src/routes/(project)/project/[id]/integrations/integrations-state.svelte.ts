// The project window's Integrations screen: one issue tracker per project.
// Spec: docs/spec/screen/project-integrations.md (daemon contract: the tracker
// spec's R3, R4, R8, R9).
//
// Two promises this state keeps. A failed read is the daemon-error state, never
// "not connected" — that would invite connecting a board the daemon already
// has. And the token lives only in `token`, only until its request returns: it
// is never stored, logged, or read back.
import type { ApiResult } from '$lib/api.js';
import type { TrackerConnection, TrackerSyncCounts } from '$lib/types.js';

export interface TrackerApi {
  tryListTrackers: (projectId: string) => Promise<ApiResult<TrackerConnection[]>>;
  tryConnectTracker: (
    projectId: string,
    kind: string,
    body: Record<string, unknown>,
  ) => Promise<ApiResult<{ tracker: TrackerConnection; first_sync: { pending: boolean } }>>;
  trySyncTracker: (projectId: string) => Promise<ApiResult<{ counts: TrackerSyncCounts }>>;
  tryDisconnectTracker: (projectId: string) => Promise<ApiResult<void>>;
}

/** A failure as the screen shows it. */
export interface TrackerAlert {
  code: string;
  text: string;
  /** The form field the failure concerns, for `aria-describedby`. */
  field: 'board_url' | 'email' | 'token' | null;
  link: { href: string; label: string } | null;
}

export const CREATE_TOKEN_URL = 'https://id.atlassian.com/manage-profile/security/api-tokens';

/** Polling interval while a first sync is pending. */
export const POLL_MS = 2000;

const reach = (code: string) =>
  `Couldn’t reach Jira (${code}). Check the site address and your connection.`;
const keychain = 'The saved token can’t be read from the system keychain.';

/** Fixed daemon codes → the screen's words (the spec's error table). */
const WORDS: Record<string, string> = {
  invalid_base_url:
    'That isn’t a Jira Cloud board address. It should look like https://<your-site>.atlassian.net/…/boards/<number> (older sites: <your-site>.jira.com).',
  unauthorized: 'Jira rejected the email and token.',
  forbidden: 'This account can’t read that board.',
  email_required: 'Enter your Atlassian account email and an API token.',
  token_required: 'Enter your Atlassian account email and an API token.',
  board_project_mismatch: 'That board belongs to a different Jira project than the one given.',
  not_found: 'Jira no longer has this board (it may have been deleted).',
  rate_limited: 'Jira is rate-limiting requests. Try again in a minute.',
  transport: reach('transport'),
  http_error: reach('http_error'),
  redirect: reach('redirect'),
  decode: reach('decode'),
  token_missing: keychain,
  credential_store: keychain,
  forbidden_caller: 'The sensei daemon refused this window’s request.',
  not_connected: 'This project has no tracker connected.',
  project_not_found: 'This project no longer exists in sensei.',
  internal: 'The sensei daemon hit a database error.',
  db: 'sensei hit a database error while syncing.',
  unsupported_tracker: 'This tracker type isn’t available in this version of sensei.',
  busy: 'A sync is already running.',
};

const FIELD: Record<string, TrackerAlert['field']> = {
  invalid_base_url: 'board_url',
  board_not_found: 'board_url',
  email_required: 'email',
  token_required: 'token',
  unauthorized: 'token',
};

/** The alert for a failed request: `status` 0 is the daemon not answering. */
export function errorFor(status: number, body: Record<string, unknown> | undefined): TrackerAlert {
  if (status === 0) {
    return { code: 'unreachable', text: 'The sensei daemon didn’t respond.', field: null, link: null };
  }
  const code = typeof body?.error === 'string' ? body.error : `http_${status}`;
  const field = FIELD[code] ?? null;
  if (code === 'board_not_found') {
    return {
      code,
      text: `No board #${String(body?.board_ref ?? '')} on ${String(body?.base_url ?? '')}.`,
      field,
      link: null,
    };
  }
  if (code === 'board_already_connected') {
    const project = (body?.project ?? {}) as { id?: string; name?: string };
    return {
      code,
      text: `This board is already connected to project ${project.name ?? ''}.`,
      field,
      link: project.id ? { href: `/project/${project.id}/integrations`, label: project.name ?? project.id } : null,
    };
  }
  const link = code === 'unauthorized' ? { href: CREATE_TOKEN_URL, label: 'create a new token' } : null;
  return { code, text: WORDS[code] ?? `Unexpected error from sensei (${code}).`, field, link };
}

/** "just now", "2 min ago", "3 h ago", "2 days ago". */
export function elapsedLabel(iso: string, now: Date): string {
  const ms = now.getTime() - new Date(iso).getTime();
  if (Number.isNaN(ms)) return '';
  const min = Math.floor(ms / 60_000);
  if (min < 1) return 'just now';
  if (min < 60) return `${min} min ago`;
  const h = Math.floor(min / 60);
  if (h < 48) return `${h} h ago`;
  return `${Math.floor(h / 24)} days ago`;
}

const lastErrorText = (code: string | null) => (code ? errorFor(502, { error: code }).text : '');

export type View = 'loading' | 'daemon_error' | 'not_connected' | 'connected';

export class TrackerIntegration {
  #api: TrackerApi;
  #projectId: string;
  #now: () => Date;
  #schedule: (fn: () => void, ms: number) => () => void;
  #cancelPoll: (() => void) | null = null;
  #disposed = false;

  view = $state<View>('loading');
  tracker = $state<TrackerConnection | null>(null);
  /** Why the read failed, in the daemon-error view. */
  loadError = $state<string | null>(null);

  boardUrl = $state('');
  email = $state('');
  token = $state('');
  advanced = $state(false);
  siteUrl = $state('');
  boardId = $state('');

  busy = $state<null | 'connect' | 'sync' | 'disconnect'>(null);
  /** The live `role="status"` line. */
  statusText = $state('');
  alert = $state<TrackerAlert | null>(null);
  /** Bumped on every new alert so the view can move focus to it. */
  alertSeq = $state(0);
  replacing = $state(false);
  confirmingDisconnect = $state(false);
  /** True from a connect until its first sync settles: drives the status line. */
  #awaitingFirstSync = false;

  constructor(
    api: TrackerApi,
    projectId: string,
    opts: { now?: () => Date; schedule?: (fn: () => void, ms: number) => () => void } = {},
  ) {
    this.#api = api;
    this.#projectId = projectId;
    this.#now = opts.now ?? (() => new Date());
    this.#schedule =
      opts.schedule ??
      ((fn, ms) => {
        const t = setTimeout(fn, ms);
        return () => clearTimeout(t);
      });
  }

  /** Board URL (or Site + Board id), email and token are all given. */
  get canConnect(): boolean {
    const given = (v: string) => v.trim().length > 0;
    const board = this.advanced ? given(this.siteUrl) && given(this.boardId) : given(this.boardUrl);
    return board && given(this.email) && given(this.token);
  }

  get cardTitle(): string {
    const t = this.tracker;
    if (!t) return '';
    const site = t.base_url.replace(/^https:\/\//, '');
    const board = t.board_name ? `board "${t.board_name}" (#${t.board_ref})` : `board #${t.board_ref}`;
    return `Jira · ${site} · project ${t.external_scope} · ${board}`;
  }

  /** The status as text (R9); the dot beside it is decoration. */
  get cardStatus(): string {
    const t = this.tracker;
    if (!t) return '';
    const counts = `${t.counts.sprints} sprints · ${t.counts.tasks} tickets`;
    const since = t.last_success_at ? elapsedLabel(t.last_success_at, this.#now()) : '';
    switch (t.status) {
      case 'connected':
        return `connected · synced ${since} · ${counts}`;
      case 'stale':
        return `stale · last synced ${since} · ${counts}`;
      case 'pending':
        return 'pending · first sync running…';
      case 'error':
        return `error · ${lastErrorText(t.last_error)}`;
    }
  }

  async load() {
    const res = await this.#api.tryListTrackers(this.#projectId);
    if (!res.ok) {
      this.#stopPolling();
      this.loadError = errorFor(res.error.status, 'body' in res.error ? res.error.body : undefined).text;
      this.view = 'daemon_error';
      return;
    }
    this.loadError = null;
    this.#show(res.data[0] ?? null);
  }

  async connect() {
    if (!this.canConnect || this.busy) return;
    const body: Record<string, unknown> = this.advanced
      ? { base_url: this.siteUrl.trim(), board_id: this.boardId.trim() }
      : { board_url: this.boardUrl.trim() };
    body.email = this.email.trim();
    body.api_token = this.token;
    this.busy = 'connect';
    this.alert = null;
    this.statusText = 'Checking credentials and board…';
    const res = await this.#api.tryConnectTracker(this.#projectId, 'jira', body);
    // The token's only job is done, whatever the answer.
    this.token = '';
    this.busy = null;
    if (!res.ok) {
      this.statusText = '';
      this.#fail(res.error);
      return;
    }
    this.replacing = false;
    this.#awaitingFirstSync = true;
    this.#show(res.data.tracker);
  }

  async sync() {
    if (this.busy) return;
    this.busy = 'sync';
    this.alert = null;
    const res = await this.#api.trySyncTracker(this.#projectId);
    this.busy = null;
    if (!res.ok) {
      this.#fail(res.error);
      return;
    }
    await this.load();
  }

  /** Re-open the form for a new token, the connection's own values filled. */
  startReplace() {
    const t = this.tracker;
    if (!t) return;
    this.advanced = true;
    this.siteUrl = t.base_url;
    this.boardId = t.board_ref;
    this.boardUrl = '';
    this.email = t.account;
    this.token = '';
    this.alert = null;
    this.replacing = true;
  }

  cancelReplace() {
    this.replacing = false;
    this.token = '';
    this.alert = null;
  }

  askDisconnect() {
    this.confirmingDisconnect = true;
  }

  cancelDisconnect() {
    this.confirmingDisconnect = false;
  }

  async confirmDisconnect() {
    this.confirmingDisconnect = false;
    this.busy = 'disconnect';
    this.alert = null;
    const res = await this.#api.tryDisconnectTracker(this.#projectId);
    this.busy = null;
    if (!res.ok) {
      this.#fail(res.error);
      return;
    }
    this.#stopPolling();
    this.statusText = '';
    this.#show(null);
  }

  /** Stop polling; the screen is closing. */
  dispose() {
    this.#disposed = true;
    this.#stopPolling();
  }

  #fail(error: { status: number; message: string; body?: Record<string, unknown> }) {
    this.alert = errorFor(error.status, error.body ?? { error: error.message });
    this.alertSeq += 1;
  }

  #show(tracker: TrackerConnection | null) {
    this.tracker = tracker;
    this.view = tracker ? 'connected' : 'not_connected';
    this.#stopPolling();
    if (!tracker) return;
    if (tracker.status === 'pending') {
      if (this.#awaitingFirstSync) this.statusText = 'Connected · first sync running…';
      this.#poll();
      return;
    }
    if (this.#awaitingFirstSync) {
      this.#awaitingFirstSync = false;
      this.statusText =
        tracker.status === 'error'
          ? `Connected, but the first sync failed: ${lastErrorText(tracker.last_error)}`
          : `Connected · ${tracker.counts.sprints} sprints · ${tracker.counts.tasks} tickets`;
    }
  }

  #poll() {
    if (this.#disposed) return;
    this.#cancelPoll = this.#schedule(() => {
      this.#cancelPoll = null;
      void this.load();
    }, POLL_MS);
  }

  #stopPolling() {
    this.#cancelPoll?.();
    this.#cancelPoll = null;
  }
}
