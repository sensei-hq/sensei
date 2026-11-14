// The workspace window's Integrations screen: the workspace's issue-tracker
// boards, each at a level (D40, docs/spec/tracker-levels.md).
// Spec: docs/spec/screen/workspace-integrations.md (daemon contract: the tracker
// spec's R3, R4, R8, R9).
//
// Two promises this state keeps. A failed read is the daemon-error state, never
// "not connected" — that would invite connecting a board the daemon already
// has. And the token lives only in `token`, only until its request returns: it
// is never stored, logged, or read back.
import { DEFAULT_REMEDY } from '@sensei/ui';
import { failureNotice } from '@sensei/ui';
import type { ApiResult } from '$lib/api.js';
import type {
  TrackerConnection,
  TrackerFieldMapping,
  TrackerKind,
  TrackerLevel,
  TrackerMetric,
  TrackerSyncCounts,
} from '$lib/types.js';
import { FIELD_MAPPING_ERRORS, errorFor, writebackStopped, type TrackerAlert } from '$lib/tracker-errors.js';

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
  trySetTrackerWriteback: (
    workspaceId: string,
    trackerId: string,
    enabled: boolean,
  ) => Promise<ApiResult<TrackerConnection>>;
  tryGetTrackerFields: (
    workspaceId: string,
    trackerId: string,
  ) => Promise<ApiResult<{ fields: TrackerFieldMapping[] }>>;
  tryPutTrackerFields: (
    workspaceId: string,
    trackerId: string,
    fields: TrackerFieldMapping[],
  ) => Promise<ApiResult<{ fields: TrackerFieldMapping[] }>>;
}

/** The metrics a field can carry, in the order the daemon writes them (usage W3, then
 *  SDLC S9), with labels. */
export const METRICS: ReadonlyArray<{ metric: TrackerMetric; label: string }> = [
  { metric: 'sessions', label: 'Sessions' },
  { metric: 'active_hours', label: 'Active hours' },
  { metric: 'claude_hours', label: 'Claude hours' },
  { metric: 'user_hours', label: 'Your hours' },
  { metric: 'wall_hours', label: 'Wall-clock hours' },
  { metric: 'cycle_hours', label: 'Cycle hours' },
  { metric: 'tokens_fresh', label: 'Fresh tokens' },
  { metric: 'cache_read', label: 'Cache-read tokens' },
  { metric: 'cache_write', label: 'Cache-write tokens' },
  { metric: 'tokens_out', label: 'Output tokens' },
  { metric: 'tokens_reasoning', label: 'Reasoning tokens' },
  { metric: 'metered_cost', label: 'Metered cost' },
  { metric: 'api_cost', label: 'API cost (USD)' },
  // SDLC figures (tracker-sdlc-metrics S9), appended in the daemon's order.
  { metric: 'compile_defects', label: 'Compile defects' },
  { metric: 'test_defects', label: 'Test defects' },
  { metric: 'review_fixes', label: 'Review fixes' },
  { metric: 'escaped_task_defects', label: 'Escaped task defects' },
  { metric: 'review_defects', label: 'Review defects' },
  { metric: 'total_defects', label: 'Total defects' },
  { metric: 'build_runs', label: 'Build runs' },
  { metric: 'rework_cycles', label: 'Rework cycles' },
  { metric: 'prompts', label: 'Prompts' },
  { metric: 'testing_efficiency', label: 'Testing efficiency (%)' },
  { metric: 'review_efficiency', label: 'Review efficiency (%)' },
  { metric: 'build_failure_rate', label: 'Build failure rate (%)' },
  { metric: 'on_hold_ratio', label: 'On-hold ratio (%)' },
  { metric: 'post_review_rework_ratio', label: 'Post-review rework (%)' },
  { metric: 'mean_build_recovery_minutes', label: 'Build recovery (minutes)' },
  { metric: 'on_hold_minutes', label: 'On hold (minutes)' },
  { metric: 'rework_minutes', label: 'Rework (minutes)' },
  { metric: 'worked_minutes', label: 'Worked (minutes)' },
  { metric: 'execution_hours', label: 'Execution hours' },
  { metric: 'lead_hours', label: 'Lead hours' },
  { metric: 'cost_per_prompt_cycle', label: 'Cost per prompt cycle (USD)' },
];

/** The row a refused save names (a duplicate names its second row), or `null`. */
/** The lowercased host of a board URL, or `''` when it is not one yet. */
export function boardHost(url: string): string {
  try {
    return new URL(url.trim()).hostname.toLowerCase();
  } catch {
    return '';
  }
}

export function refusedRow(rows: TrackerFieldMapping[], body: Record<string, unknown> | undefined): number | null {
  const ref = typeof body?.field_ref === 'string' ? body.field_ref : null;
  const metric = typeof body?.metric === 'string' ? body.metric : null;
  const hits = rows
    .map((r, i) => ({ r, i }))
    .filter(({ r }) => (ref !== null && r.field_ref === ref) || (ref === null && metric !== null && r.metric === metric));
  if (hits.length === 0) return null;
  // The daemon refuses a duplicate at its second occurrence.
  return body?.error === 'duplicate_mapping' && hits.length > 1 ? hits[1].i : hits[0].i;
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

const lastErrorText = (code: string | null, kind: TrackerKind = 'jira') =>
  code ? errorFor(502, { error: code }, kind).text : '';

const tickets = (n: number) => `${n} ${n === 1 ? 'ticket' : 'tickets'}`;

/** A board's write-back state as the card says it (W-UI), naming its tracker. */
export function writebackText(w: TrackerConnection['writeback'], kind: TrackerKind): string {
  switch (w.state) {
    case 'active': {
      const failing = w.failing > 0 ? ` ${w.failing} couldn’t be written.` : '';
      // Skipped tickets are not written to (tracker-forge-comment-writeback F7).
      return `Writing to ${tickets(w.assigned - w.not_private)} assigned to you.${failing}`;
    }
    case 'disabled':
      return 'Write-back is off.';
    case 'pass_failed':
      return (w.error && writebackStopped(w.error, kind)) || `Write-back stopped (${w.error ?? 'unknown'}).`;
    case 'owner_not_a_person':
      return 'This token belongs to an app account, so sensei can’t tell which tickets are yours. Nothing is written.';
    case 'owner_lookup_failed':
      return w.error
        ? `Couldn’t check which tickets are yours: ${lastErrorText(w.error, kind).replace(/\.$/, '')}. Retrying on each sync.`
        : 'Couldn’t check which tickets are yours. Retrying on each sync.';
    case 'owner_unknown':
      return 'Checking which tickets are yours on the next sync.';
    case 'nothing_assigned':
      return 'No tickets on this board are assigned to you.';
  }
}

/** The tickets write-back skips because their repository or project is not private
 *  (tracker-forge-comment-writeback F7), or `null` when there are none. */
export function notPrivateText(w: TrackerConnection['writeback']): string | null {
  if (w.not_private <= 0) return null;
  const are = w.not_private === 1 ? 'is' : 'are';
  return `${tickets(w.not_private)} ${are} in public or internal repositories; sensei doesn’t comment there.`;
}

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

  /** Notice props for the daemon-error view (packages/ui/notice.js). */
  get loadNotice() {
    return failureNotice("Couldn't load this workspace's trackers", this.loadError);
  }

  /** The level the new board serves: the user's choice, never defaulted. */
  level = $state<TrackerLevel | null>(null);
  /** The tracker the form connects: Jira, GitLab or GitHub. */
  kind = $state<TrackerKind>('jira');
  /** GitHub: the project's Iteration field, when it has more than one (tracker-github §H1). */
  iterationField = $state('');
  /** GitHub: the field names the daemon offered after `iteration_field_ambiguous`. */
  iterationFields = $state<string[]>([]);
  boardUrl = $state('');
  /** GitLab, self-managed: the host typed again (tracker-gitlab §G5.2). */
  confirmHost = $state('');
  email = $state('');
  token = $state('');
  advanced = $state(false);
  siteUrl = $state('');
  boardId = $state('');

  busy = $state<null | 'connect' | 'sync' | 'disconnect' | 'writeback' | 'fields'>(null);
  /** The board whose Fields editor is open. */
  editingFields = $state<string | null>(null);
  /** The editor's rows: the whole mapping, saved as one set. */
  fieldRows = $state<TrackerFieldMapping[]>([]);
  /** A refused save's reason, on the row it names. */
  fieldError = $state<{ row: number; text: string } | null>(null);
  /** The refused row as a Notice, with its next step (the shared family). */
  get fieldNotice() {
    return this.fieldError
      ? { tone: 'fault' as const, title: this.fieldError.text, remedy: 'Fix this row, then save the fields again.' }
      : null;
  }
  /** The board a sync or disconnect is running for. */
  busyTracker = $state<string | null>(null);
  /** The live `role="status"` line. */
  statusText = $state('');
  alert = $state<TrackerAlert | null>(null);
  /** Notice props for the alert (packages/ui/notice.js). The alert's text is
   *  already a sentence written for the reader (lib/tracker-errors.ts), so it is
   *  the title; its code is for support. */
  get alertNotice() {
    // Every alert names its next step (H2).
    return this.alert ? { tone: 'fault' as const, title: this.alert.text, code: this.alert.code, remedy: DEFAULT_REMEDY } : null;
  }
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

  /** Whether the GitLab board URL is on a self-managed host, which must be confirmed. */
  get needsConfirmHost(): boolean {
    return this.kind === 'gitlab' && boardHost(this.boardUrl) !== 'gitlab.com';
  }

  /** Jira: level, board URL (or Site + Board id), email and token. GitLab: level,
   *  board URL, token, and the host typed again unless it is gitlab.com. */
  get canConnect(): boolean {
    const given = (v: string) => v.trim().length > 0;
    if (this.kind === 'github') {
      return this.level !== null && given(this.boardUrl) && given(this.token);
    }
    if (this.kind === 'gitlab') {
      return (
        this.level !== null && given(this.boardUrl) && given(this.token) && (!this.needsConfirmHost || given(this.confirmHost))
      );
    }
    const board = this.advanced ? given(this.siteUrl) && given(this.boardId) : given(this.boardUrl);
    return this.level !== null && board && given(this.email) && given(this.token);
  }

  /** GitLab: where the board's sprints come from; GitHub: its Iteration field. `null` for Jira. */
  sprintsLine(t: TrackerConnection): string | null {
    if (t.kind === 'github') return t.iteration_field ? `Sprints: the ${t.iteration_field} iteration field` : null;
    if (t.kind !== 'gitlab' || !t.sprint_source) return null;
    return `Sprints: ${t.sprint_source === 'iteration' ? 'iterations' : 'milestones'}`;
  }

  /** GitLab: the one-line notice for 14 days after the sprint source changed (§G3). */
  sourceChangeNotice(t: TrackerConnection): string | null {
    if (t.kind !== 'gitlab' || !t.sprint_source_changed_at || !t.sprint_source) return null;
    const age = this.#now().getTime() - new Date(t.sprint_source_changed_at).getTime();
    if (!(age >= 0 && age <= 14 * 24 * 3600 * 1000)) return null;
    const [now, before] = t.sprint_source === 'iteration' ? ['iterations', 'milestone'] : ['milestones', 'iteration'];
    return `Sprints now come from ${now}. Earlier ${before} sprints are marked removed.`;
  }

  cardTitle(t: TrackerConnection): string {
    const site = t.base_url.replace(/^https:\/\//, '');
    if (t.kind === 'github') {
      return `GitHub · ${t.external_scope}${t.board_name ? ` · ${t.board_name}` : ''}`;
    }
    if (t.kind === 'gitlab') {
      const path = t.full_path ?? t.external_scope;
      return `GitLab · ${site} · ${path}${t.board_name && t.board_name !== path ? ` (${t.board_name})` : ''}`;
    }
    const board = t.board_name ? `board "${t.board_name}" (#${t.board_ref})` : `board #${t.board_ref}`;
    return `Jira · ${site} · project ${t.external_scope} · ${board}`;
  }

  /** Open (or close) the board's Fields editor with its saved mapping. */
  async openFields(t: TrackerConnection) {
    if (this.editingFields === t.id) {
      this.editingFields = null;
      return;
    }
    this.fieldError = null;
    this.alert = null;
    const res = await this.#api.tryGetTrackerFields(this.#workspaceId, t.id);
    if (!res.ok) {
      this.#fail(res.error);
      return;
    }
    this.fieldRows = res.data.fields.map((f) => ({ ...f }));
    this.editingFields = t.id;
  }

  addFieldRow() {
    this.fieldRows = [...this.fieldRows, { metric: 'sessions', field_ref: '' }];
  }

  removeFieldRow(i: number) {
    this.fieldRows = this.fieldRows.filter((_, j) => j !== i);
    this.fieldError = null;
  }

  /** Save the editor's rows as the board's whole mapping. */
  async saveFields() {
    const id = this.editingFields;
    if (!id || this.busy) return;
    const rows = this.fieldRows.map((r) => ({ metric: r.metric, field_ref: r.field_ref.trim() }));
    this.busy = 'fields';
    this.busyTracker = id;
    this.fieldError = null;
    this.alert = null;
    const res = await this.#api.tryPutTrackerFields(this.#workspaceId, id, rows);
    this.busy = null;
    this.busyTracker = null;
    if (!res.ok) {
      const body = 'body' in res.error ? res.error.body : undefined;
      const code = typeof body?.error === 'string' ? body.error : res.error.message;
      const row = refusedRow(rows, body);
      if (row !== null && FIELD_MAPPING_ERRORS[code]) {
        this.fieldError = { row, text: FIELD_MAPPING_ERRORS[code] };
      } else if ((res.error.status === 422 || code === 'busy') && FIELD_MAPPING_ERRORS[code]) {
        this.alert = { code, text: FIELD_MAPPING_ERRORS[code], field: null, link: null };
        this.alertSeq += 1;
      } else {
        this.#fail(res.error);
      }
      return;
    }
    this.fieldRows = res.data.fields.map((f) => ({ ...f }));
    this.statusText = 'Fields saved.';
    await this.load();
  }

  /** The board's write-back state as text (W-UI). */
  writebackLine(t: TrackerConnection): string {
    return writebackText(t.writeback, t.kind);
  }

  /** Tickets skipped because their repository is not private (F7), or `null`. */
  notPrivateLine(t: TrackerConnection): string | null {
    return notPrivateText(t.writeback);
  }

  /** What write-back does on a tracker without field mapping, and the token it needs (F10). */
  commentOnlyNote(t: TrackerConnection): string {
    const where = t.kind === 'gitlab' ? 'private projects' : 'private repositories';
    const scope: Partial<Record<TrackerKind, string>> = {
      gitlab: 'The token needs the api scope.',
      github: 'The token needs Issues: read and write on each repository (or a classic repo token).',
    };
    return [
      `Sensei posts one comment on each ticket assigned to you, in ${where} only, and keeps it up to date.`,
      scope[t.kind],
      'The first comment notifies the ticket’s participants; updates don’t.',
    ]
      .filter(Boolean)
      .join(' ');
  }


  /** Switch write-back on or off; the card shows what the daemon returns, or stays as it was. */
  async toggleWriteback(t: TrackerConnection) {
    if (this.busy) return;
    this.busy = 'writeback';
    this.busyTracker = t.id;
    this.alert = null;
    const res = await this.#api.trySetTrackerWriteback(this.#workspaceId, t.id, !t.writeback.enabled);
    this.busy = null;
    this.busyTracker = null;
    if (!res.ok) {
      this.#fail(res.error);
      return;
    }
    this.trackers = this.trackers.map((x) => (x.id === res.data.id ? res.data : x));
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
        return `error · ${lastErrorText(t.last_error, t.kind)}`;
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
    const kind = this.kind;
    let body: Record<string, unknown>;
    if (kind === 'github') {
      body = { board_url: this.boardUrl.trim(), api_token: this.token };
      if (this.iterationField.trim()) body.iteration_field = this.iterationField.trim();
    } else if (kind === 'gitlab') {
      body = { board_url: this.boardUrl.trim(), api_token: this.token };
      if (this.confirmHost.trim()) body.confirm_host = this.confirmHost.trim();
    } else {
      body = this.advanced
        ? { base_url: this.siteUrl.trim(), board_id: this.boardId.trim() }
        : { board_url: this.boardUrl.trim() };
      body.email = this.email.trim();
      body.api_token = this.token;
    }
    this.busy = 'connect';
    this.alert = null;
    this.statusText = 'Checking credentials and board…';
    const res = await this.#api.tryConnectTracker(this.#workspaceId, kind, this.level, body);
    // The token's only job is done, whatever the answer.
    this.token = '';
    this.busy = null;
    if (!res.ok) {
      this.statusText = '';
      this.#fail(res.error, kind);
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
      this.#fail(res.error, this.trackers.find((t) => t.id === trackerId)?.kind);
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
    if (t.kind === 'github') {
      // The project's address and its field, so only the token is new.
      this.kind = 'github';
      this.boardUrl = t.url ?? '';
      this.iterationField = t.iteration_field ?? '';
      this.adding = false;
      this.replacing = t.id;
      return;
    }
    if (t.kind === 'gitlab') {
      // The board's current path and its confirmed host, so only the token is new.
      this.kind = 'gitlab';
      this.boardUrl = `${t.base_url}/${t.full_path ?? t.external_scope}`;
      this.confirmHost = t.base_url.replace(/^https:\/\//, '');
      this.adding = false;
      this.replacing = t.id;
      return;
    }
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
      this.#fail(res.error, t.kind);
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
    this.kind = 'jira';
    this.confirmHost = '';
    this.iterationField = '';
    this.iterationFields = [];
    this.boardUrl = '';
    this.advanced = false;
    this.siteUrl = '';
    this.boardId = '';
    this.token = '';
    this.alert = null;
  }

  #fail(error: { status: number; message: string; body?: Record<string, unknown> }, kind?: TrackerKind) {
    this.alert = errorFor(error.status, error.body ?? { error: error.message }, kind);
    const fields = error.body?.fields;
    this.iterationFields =
      this.alert.code === 'iteration_field_ambiguous' && Array.isArray(fields) ? fields.map(String) : [];
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
          ? `Connected, but the first sync failed: ${lastErrorText(awaited.last_error, awaited.kind)}`
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
