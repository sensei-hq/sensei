// The workspace window's Integrations screen: the workspace's issue-tracker
// boards, each at a level (D40, docs/spec/tracker-levels.md).
// Spec: docs/spec/screen/workspace-integrations.md (daemon contract: the tracker
// spec's R3, R4, R8, R9).
//
// Two promises this state keeps. A failed read is the daemon-error state, never
// "not connected" — that would invite connecting a board the daemon already
// has. And the token lives only in `token`, only until its request returns: it
// is never stored, logged, or read back.
import type { ApiResult } from '$lib/api.js';
import type { TrackerConnection, TrackerLevel, TrackerSyncCounts } from '$lib/types.js';
import { errorFor, type TrackerAlert } from '$lib/tracker-errors.js';

export { CREATE_TOKEN_URL, errorFor, type TrackerAlert } from '$lib/tracker-errors.js';

export interface TrackerApi {
  tryListTrackers: (workspaceId: string) => Promise<ApiResult<TrackerConnection[]>>;
  tryConnectTracker: (
    workspaceId: string,
    kind: string,
    level: TrackerLevel,
    body: Record<string, unknown>,
  ) => Promise<ApiResult<{ tracker: TrackerConnection; first_sync: { pending: boolean } }>>;
  trySyncTracker: (
    workspaceId: string,
    trackerId: string,
  ) => Promise<ApiResult<{ counts: TrackerSyncCounts }>>;
  tryDisconnectTracker: (workspaceId: string, trackerId: string) => Promise<ApiResult<void>>;
}

/** The levels a board can serve, in the order the screen lists them (D40). */
export const LEVELS: ReadonlyArray<{ level: TrackerLevel; label: string; help: string }> = [
  { level: 'epic', label: 'Epics', help: 'A portfolio board: its epics and milestones.' },
  { level: 'sprint', label: 'Sprints', help: 'A team board: its sprints and tickets, one lane on the Sprints screen.' },
  { level: 'task', label: 'Tasks', help: 'A board whose tickets are linked from branches, commits and prompts only.' },
];

/** Polling interval while a first sync is pending. */
export const POLL_MS = 2000;

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
  #workspaceId: string;
  #now: () => Date;
  #schedule: (fn: () => void, ms: number) => () => void;
  #cancelPoll: (() => void) | null = null;
  #disposed = false;

  view = $state<View>('loading');
  trackers = $state<TrackerConnection[]>([]);
  /** Why the read failed, in the daemon-error view. */
  loadError = $state<string | null>(null);

  /** The level the new board serves: the user's choice, never defaulted. */
  level = $state<TrackerLevel | null>(null);
  boardUrl = $state('');
  email = $state('');
  token = $state('');
  advanced = $state(false);
  siteUrl = $state('');
  boardId = $state('');

  busy = $state<null | 'connect' | 'sync' | 'disconnect'>(null);
  /** The board a sync or disconnect is running for. */
  busyTracker = $state<string | null>(null);
  /** The live `role="status"` line. */
  statusText = $state('');
  alert = $state<TrackerAlert | null>(null);
  /** Bumped on every new alert so the view can move focus to it. */
  alertSeq = $state(0);
  /** The form is open beside existing boards, for another one. */
  adding = $state(false);
  /** The board whose token is being replaced. */
  replacing = $state<string | null>(null);
  /** The board the disconnect dialog asks about. */
  confirmingDisconnect = $state<TrackerConnection | null>(null);
  /** The board connected last, until its first sync settles: drives the status line. */
  #awaitingFirstSync: string | null = null;

  constructor(
    api: TrackerApi,
    workspaceId: string,
    opts: { now?: () => Date; schedule?: (fn: () => void, ms: number) => () => void } = {},
  ) {
    this.#api = api;
    this.#workspaceId = workspaceId;
    this.#now = opts.now ?? (() => new Date());
    this.#schedule =
      opts.schedule ??
      ((fn, ms) => {
        const t = setTimeout(fn, ms);
        return () => clearTimeout(t);
      });
  }

  /** The workspace's boards by level, epics first; levels without a board left out. */
  get groups() {
    return LEVELS.map((l) => ({ ...l, trackers: this.trackers.filter((t) => t.level === l.level) })).filter(
      (g) => g.trackers.length > 0,
    );
  }

  get showForm(): boolean {
    return this.view === 'not_connected' || this.adding || this.replacing !== null;
  }

  /** Level, board URL (or Site + Board id), email and token are all given. */
  get canConnect(): boolean {
    const given = (v: string) => v.trim().length > 0;
    const board = this.advanced ? given(this.siteUrl) && given(this.boardId) : given(this.boardUrl);
    return this.level !== null && board && given(this.email) && given(this.token);
  }

  cardTitle(t: TrackerConnection): string {
    const site = t.base_url.replace(/^https:\/\//, '');
    const board = t.board_name ? `board "${t.board_name}" (#${t.board_ref})` : `board #${t.board_ref}`;
    return `Jira · ${site} · project ${t.external_scope} · ${board}`;
  }

  /** The status as text (R9); the dot beside it is decoration. */
  cardStatus(t: TrackerConnection): string {
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
    const res = await this.#api.tryListTrackers(this.#workspaceId);
    if (!res.ok) {
      this.#stopPolling();
      this.loadError = errorFor(res.error.status, 'body' in res.error ? res.error.body : undefined).text;
      this.view = 'daemon_error';
      return;
    }
    this.loadError = null;
    this.#show(res.data);
  }

  async connect() {
    if (!this.canConnect || this.busy || this.level === null) return;
    const body: Record<string, unknown> = this.advanced
      ? { base_url: this.siteUrl.trim(), board_id: this.boardId.trim() }
      : { board_url: this.boardUrl.trim() };
    body.email = this.email.trim();
    body.api_token = this.token;
    this.busy = 'connect';
    this.alert = null;
    this.statusText = 'Checking credentials and board…';
    const res = await this.#api.tryConnectTracker(this.#workspaceId, 'jira', this.level, body);
    // The token's only job is done, whatever the answer.
    this.token = '';
    this.busy = null;
    if (!res.ok) {
      this.statusText = '';
      this.#fail(res.error);
      return;
    }
    this.replacing = null;
    this.adding = false;
    this.#awaitingFirstSync = res.data.tracker.id;
    const others = this.trackers.filter((t) => t.id !== res.data.tracker.id);
    this.#show([...others, res.data.tracker]);
  }

  async sync(trackerId: string) {
    if (this.busy) return;
    this.busy = 'sync';
    this.busyTracker = trackerId;
    this.alert = null;
    const res = await this.#api.trySyncTracker(this.#workspaceId, trackerId);
    this.busy = null;
    this.busyTracker = null;
    if (!res.ok) {
      this.#fail(res.error);
      return;
    }
    await this.load();
  }

  /** Open an empty form for another board. */
  startAdd() {
    this.#clearForm();
    this.adding = true;
  }

  cancelAdd() {
    this.adding = false;
    this.#clearForm();
  }

  /** Re-open the form for a new token, the board's own values filled. */
  startReplace(t: TrackerConnection) {
    this.#clearForm();
    this.level = t.level;
    this.advanced = true;
    this.siteUrl = t.base_url;
    this.boardId = t.board_ref;
    this.email = t.account;
    this.adding = false;
    this.replacing = t.id;
  }

  cancelReplace() {
    this.replacing = null;
    this.#clearForm();
  }

  askDisconnect(t: TrackerConnection) {
    this.confirmingDisconnect = t;
  }

  cancelDisconnect() {
    this.confirmingDisconnect = null;
  }

  async confirmDisconnect() {
    const t = this.confirmingDisconnect;
    this.confirmingDisconnect = null;
    if (!t) return;
    this.busy = 'disconnect';
    this.busyTracker = t.id;
    this.alert = null;
    const res = await this.#api.tryDisconnectTracker(this.#workspaceId, t.id);
    this.busy = null;
    this.busyTracker = null;
    if (!res.ok) {
      this.#fail(res.error);
      return;
    }
    this.statusText = '';
    await this.load();
  }

  /** Stop polling; the screen is closing. */
  dispose() {
    this.#disposed = true;
    this.#stopPolling();
  }

  #clearForm() {
    this.level = null;
    this.boardUrl = '';
    this.advanced = false;
    this.siteUrl = '';
    this.boardId = '';
    this.token = '';
    this.alert = null;
  }

  #fail(error: { status: number; message: string; body?: Record<string, unknown> }) {
    this.alert = errorFor(error.status, error.body ?? { error: error.message });
    this.alertSeq += 1;
  }

  #show(trackers: TrackerConnection[]) {
    this.trackers = trackers;
    this.view = trackers.length > 0 ? 'connected' : 'not_connected';
    this.#stopPolling();
    const awaited = trackers.find((t) => t.id === this.#awaitingFirstSync);
    if (awaited?.status === 'pending') {
      this.statusText = 'Connected · first sync running…';
    } else if (awaited) {
      this.#awaitingFirstSync = null;
      this.statusText =
        awaited.status === 'error'
          ? `Connected, but the first sync failed: ${lastErrorText(awaited.last_error)}`
          : `Connected · ${awaited.counts.sprints} sprints · ${awaited.counts.tasks} tickets`;
    }
    if (trackers.some((t) => t.status === 'pending')) this.#poll();
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
