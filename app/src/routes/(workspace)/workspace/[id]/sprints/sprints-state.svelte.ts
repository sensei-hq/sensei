// The workspace window's Sprints screen. Spec: docs/spec/screen/workspace-sprints.md.
import type { ApiResult } from '$lib/api.js';
import { activeMinutes, compactDuration, compactTokens } from '$lib/sessions-digest.js';
import { errorFor } from '$lib/tracker-errors.js';
import type {
  SprintUsageRow,
  TrackerConnection,
  TrackerTaskDetail,
  TrackerTaskRow,
  TrackerUsage,
  UnlinkedSession,
  BacklogUsage,
} from '$lib/types.js';

export interface SprintsApi {
  tryListTrackers: (workspaceId: string) => Promise<ApiResult<TrackerConnection[]>>;
  tryListSprints: (workspaceId: string) => Promise<ApiResult<SprintUsageRow[]>>;
  tryListTrackerTasks: (workspaceId: string, sprintId?: string) => Promise<ApiResult<TrackerTaskRow[]>>;
  tryGetTrackerTask: (workspaceId: string, key: string) => Promise<ApiResult<TrackerTaskDetail>>;
  tryListUnlinkedSessions: (workspaceId: string, sprintId: string) => Promise<ApiResult<UnlinkedSession[]>>;
  tryGetBacklog: (workspaceId: string) => Promise<ApiResult<BacklogUsage>>;
  tryListBacklogTasks: (workspaceId: string) => Promise<ApiResult<TrackerTaskRow[]>>;
}

/** The id the sprint list uses for its Backlog row (tickets in no sprint). */
export const BACKLOG = 'backlog';

export interface UsageCells {
  sessions: string;
  active: string;
  fresh: string;
  out: string;
  cost: string;
}

/** A failed request in the screens' shared words. */
function failureText(error: { status: number; message: string; body?: Record<string, unknown> }): string {
  return errorFor(error.status, error.body ?? { error: error.message }).text;
}

/** One usage object as table cells. Unknown is "—", never 0. */
export function usageCells(u: TrackerUsage): UsageCells {
  return {
    sessions: `${u.sessions} (${u.by_source.branch}/${u.by_source.prompt})`,
    active: compactDuration(activeMinutes(u.active_secs)),
    fresh: compactTokens(u.tokens_fresh),
    out: compactTokens(u.tokens_out),
    cost: u.metered_cost == null ? '—' : `$${u.metered_cost.toFixed(2)}`,
  };
}

const day = (iso: string | null) =>
  iso ? new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) : null;

function dateRange(s: SprintUsageRow): string {
  const from = day(s.start_at);
  const to = day(s.complete_at ?? s.end_at);
  if (from && to) return `${from} – ${to}`;
  if (from) return `from ${from}`;
  return 'no dates';
}

const tickets = (n: number) => `${n} ticket${n === 1 ? '' : 's'}`;

const REASONS: Record<string, string> = {
  not_started: 'Not started. Usage counts from the sprint’s start.',
  no_dates: 'Jira has no dates for this sprint, so its sessions can’t be placed in it.',
};

export interface SprintRowView {
  id: string;
  name: string;
  state: string;
  dates: string;
  tickets: string;
  current: boolean;
}

export type Strip =
  | { kind: 'window'; total: string; parts: Array<{ label: string; sessions: string; active: string; fresh: string }> }
  | { kind: 'reason'; text: string };

type View = 'loading' | 'not_connected' | 'daemon_error' | 'ready';

export class SprintsState {
  #api: SprintsApi;
  #workspaceId: string;
  /** Bumped per selection, so a slower earlier answer is dropped. */
  #seq = 0;

  view = $state<View>('loading');
  loadError = $state<string | null>(null);
  boardTitle = $state('');
  sprints = $state<SprintUsageRow[]>([]);
  selectedId = $state<string | null>(null);
  tasks = $state<TrackerTaskRow[] | null>(null);
  tasksError = $state<string | null>(null);
  ticket = $state<TrackerTaskDetail | null>(null);
  ticketError = $state<string | null>(null);
  unlinked = $state<UnlinkedSession[] | null>(null);
  unlinkedError = $state<string | null>(null);
  /** The tickets in no sprint and their total; read with the sprints. */
  backlog = $state<BacklogUsage | null>(null);

  constructor(api: SprintsApi, workspaceId: string) {
    this.#api = api;
    this.#workspaceId = workspaceId;
  }

  get selected(): SprintUsageRow | null {
    return this.sprints.find((s) => s.id === this.selectedId) ?? null;
  }

  /** The selected row's name: a sprint's, or "Backlog". */
  get selectedName(): string {
    return this.selectedId === BACKLOG ? 'Backlog' : (this.selected?.name ?? '');
  }

  get rows(): SprintRowView[] {
    const rows = this.sprints.map((s) => ({
      id: s.id,
      name: s.name,
      state: s.state,
      dates: dateRange(s),
      tickets: tickets(s.tasks),
      current: s.id === this.selectedId,
    }));
    if (!this.backlog) return rows;
    return [
      ...rows,
      {
        id: BACKLOG,
        name: 'Backlog',
        state: 'no sprint',
        dates: '',
        tickets: tickets(this.backlog.tickets),
        current: this.selectedId === BACKLOG,
      },
    ];
  }

  /** The selected sprint's own usage — never a sum of its tickets. */
  get strip(): Strip | null {
    if (this.selectedId === BACKLOG && this.backlog) {
      // No window: the backlog has no dates, so its total is all-time.
      const { tickets: n, usage } = this.backlog;
      const c = usageCells(usage);
      return {
        kind: 'window',
        total: `${tickets(n)} in no sprint`,
        parts: [{ label: 'Linked to backlog tickets', sessions: String(usage.sessions), active: c.active, fresh: c.fresh }],
      };
    }
    const s = this.selected;
    if (!s) return null;
    if (s.reason) return { kind: 'reason', text: REASONS[s.reason] ?? s.reason };
    const parts: Array<[string, TrackerUsage | null]> = [
      ['Linked to this sprint', s.linked_sprint],
      ['Linked to other tickets', s.linked_other],
      ['Not linked', s.unlinked],
    ];
    if (s.candidates_in_window == null || parts.some(([, u]) => u == null)) {
      return { kind: 'reason', text: 'sensei returned no usage for this sprint.' };
    }
    const n = s.candidates_in_window;
    return {
      kind: 'window',
      total: `${n} session${n === 1 ? '' : 's'} in this sprint’s window`,
      parts: parts.map(([label, u]) => {
        const c = usageCells(u!);
        return { label, sessions: String(u!.sessions), active: c.active, fresh: c.fresh };
      }),
    };
  }

  get sharedNote(): string | null {
    return this.tasks?.some((t) => t.usage.shared > 0)
      ? 'A session linked to several tickets counts in full for each; the sprint total counts it once.'
      : null;
  }

  /** "now in Sprint 14" for a ticket carried on to another sprint. */
  movedTo(t: TrackerTaskRow): string | null {
    return t.current_sprint && t.current_sprint.id !== this.selectedId ? `now in ${t.current_sprint.name}` : null;
  }

  /** "branch · prompt ×2 · as AUTH-420": how a session names the ticket. */
  linksText(s: TrackerTaskDetail['sessions'][number]): string {
    const parts = s.links.map((l) => (l.mentions > 1 ? `${l.source} ×${l.mentions}` : l.source));
    const current = this.ticket?.task.key;
    const old = [...new Set(s.links.map((l) => l.matched_key))].filter((k) => k !== current);
    return [...parts, ...old.map((k) => `as ${k}`)].join(' · ');
  }

  async load() {
    this.view = 'loading';
    const [trackers, sprints, backlog] = await Promise.all([
      this.#api.tryListTrackers(this.#workspaceId),
      this.#api.tryListSprints(this.#workspaceId),
      this.#api.tryGetBacklog(this.#workspaceId),
    ]);
    if (!sprints.ok) {
      const body = 'body' in sprints.error ? sprints.error.body : undefined;
      if (sprints.error.status === 404 && body?.error === 'not_connected') {
        this.view = 'not_connected';
        return;
      }
      this.loadError = failureText(sprints.error);
      this.view = 'daemon_error';
      return;
    }
    if (!trackers.ok) {
      this.loadError = failureText(trackers.error);
      this.view = 'daemon_error';
      return;
    }
    // A failed backlog read is the daemon's error, never "0 tickets".
    if (!backlog.ok) {
      this.loadError = failureText(backlog.error);
      this.view = 'daemon_error';
      return;
    }
    const t = trackers.data[0];
    if (!t) {
      this.view = 'not_connected';
      return;
    }
    this.loadError = null;
    this.boardTitle = t.board_name ?? `board #${t.board_ref}`;
    this.sprints = sprints.data;
    this.backlog = backlog.data;
    this.view = 'ready';
    const first = sprints.data.find((s) => s.state === 'active') ?? sprints.data[0];
    await this.select(first?.id ?? BACKLOG);
  }

  async select(id: string) {
    const seq = ++this.#seq;
    this.selectedId = id;
    this.tasks = null;
    this.tasksError = null;
    this.ticket = null;
    this.ticketError = null;
    this.unlinked = null;
    this.unlinkedError = null;
    const res =
      id === BACKLOG
        ? await this.#api.tryListBacklogTasks(this.#workspaceId)
        : await this.#api.tryListTrackerTasks(this.#workspaceId, id);
    if (seq !== this.#seq) return;
    if (res.ok) this.tasks = res.data;
    else this.tasksError = failureText(res.error);
  }

  async openTicket(key: string) {
    this.ticketError = null;
    const res = await this.#api.tryGetTrackerTask(this.#workspaceId, key);
    if (res.ok) this.ticket = res.data;
    else {
      this.ticket = null;
      this.ticketError = failureText(res.error);
    }
  }

  closeTicket() {
    this.ticket = null;
    this.ticketError = null;
  }

  /** Show or hide the selected sprint's unlinked sessions. */
  async toggleUnlinked() {
    if (this.unlinked) {
      this.unlinked = null;
      return;
    }
    const id = this.selectedId;
    if (!id || id === BACKLOG) return;
    this.unlinkedError = null;
    const res = await this.#api.tryListUnlinkedSessions(this.#workspaceId, id);
    if (id !== this.selectedId) return;
    if (res.ok) this.unlinked = res.data;
    else this.unlinkedError = failureText(res.error);
  }
}
