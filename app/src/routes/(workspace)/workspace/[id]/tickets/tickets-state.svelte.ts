// The workspace window's Tickets screen: one lane per sprint board, side by side
// (D40), each listing its tickets first with the sprint as a filter.
// Spec: docs/spec/screen/workspace-tickets.md, docs/spec/tracker-levels.md.
import type { ApiResult } from '$lib/api.js';
import { activeMinutes, compactDuration, compactTokens, durationMinutes } from '$lib/sessions-digest.js';
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
  tryListSprints: (workspaceId: string, trackerId: string) => Promise<ApiResult<SprintUsageRow[]>>;
  tryListTrackerTasks: (
    workspaceId: string,
    trackerId: string,
    sprintId?: string,
  ) => Promise<ApiResult<TrackerTaskRow[]>>;
  tryGetTrackerTask: (workspaceId: string, key: string) => Promise<ApiResult<TrackerTaskDetail>>;
  tryListUnlinkedSessions: (
    workspaceId: string,
    trackerId: string,
    sprintId: string,
  ) => Promise<ApiResult<UnlinkedSession[]>>;
  tryGetBacklog: (workspaceId: string, trackerId: string) => Promise<ApiResult<BacklogUsage>>;
  tryListBacklogTasks: (workspaceId: string, trackerId: string) => Promise<ApiResult<TrackerTaskRow[]>>;
}

const boardName = (t: TrackerConnection) => t.board_name ?? `board #${t.board_ref}`;

/** Sprint-filter values besides a sprint's id: every ticket, or those in no sprint. */
export const ALL = 'all';
export const BACKLOG = 'backlog';

export interface UsageCells {
  sessions: string;
  active: string;
  fresh: string;
  out: string;
  cost: string;
  /** Active AI time's split: Claude working, and you. */
  claude: string;
  you: string;
  /** The sessions' clock time, idle inside them included. */
  wall: string;
  /** First session's start to the last one's end. */
  cycle: string;
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
    claude: compactDuration(activeMinutes(u.claude_secs)),
    you: compactDuration(activeMinutes(u.user_secs)),
    wall: compactDuration(activeMinutes(u.wall_secs)),
    cycle: u.first_at && u.last_at ? compactDuration(durationMinutes(u.first_at, u.last_at)) : '—',
  };
}

const tickets = (n: number) => `${n} ticket${n === 1 ? '' : 's'}`;

const REASONS: Record<string, string> = {
  not_started: 'Not started. Usage counts from the sprint’s start.',
  no_dates: 'Jira has no dates for this sprint, so its sessions can’t be placed in it.',
};

/** One option of the sprint filter. */
export interface FilterItem {
  value: string;
  label: string;
}

export type Strip =
  | { kind: 'window'; total: string; parts: Array<{ label: string; sessions: string; active: string; fresh: string }> }
  | { kind: 'reason'; text: string };

type View = 'loading' | 'not_connected' | 'daemon_error' | 'ready';

/** One sprint board: its tickets, filtered by one of its sprints or its Backlog. */
export class TicketLane {
  #api: SprintsApi;
  #workspaceId: string;
  /** Bumped per filter change, so a slower earlier answer is dropped. */
  #seq = 0;

  readonly tracker: TrackerConnection;
  readonly sprints: SprintUsageRow[];
  /** The board's tickets in no sprint and their total. */
  readonly backlog: BacklogUsage;
  /** All tickets, one sprint's (its id), or the Backlog. */
  filter = $state<string>(ALL);
  /** Narrow the list to tickets with AI sessions. */
  aiOnly = $state(false);
  tasks = $state<TrackerTaskRow[] | null>(null);
  tasksError = $state<string | null>(null);
  unlinked = $state<UnlinkedSession[] | null>(null);
  unlinkedError = $state<string | null>(null);

  constructor(
    api: SprintsApi,
    workspaceId: string,
    tracker: TrackerConnection,
    sprints: SprintUsageRow[],
    backlog: BacklogUsage,
  ) {
    this.#api = api;
    this.#workspaceId = workspaceId;
    this.tracker = tracker;
    this.sprints = sprints;
    this.backlog = backlog;
  }

  get id(): string {
    return this.tracker.id;
  }

  get title(): string {
    return boardName(this.tracker);
  }

  /** The sprint the filter names, if it names one. */
  get selected(): SprintUsageRow | null {
    return this.sprints.find((s) => s.id === this.filter) ?? null;
  }

  /** The sprint filter's options: All tickets, each sprint newest first, Backlog. */
  get filterItems(): FilterItem[] {
    return [
      { value: ALL, label: 'All tickets' },
      ...this.sprints.map((s) => ({ value: s.id, label: `${s.name} · ${s.state}` })),
      { value: BACKLOG, label: `Backlog · ${tickets(this.backlog.tickets)}` },
    ];
  }

  /** What the filter shows, in a word: "All tickets", a sprint's name, "Backlog". */
  get filterLabel(): string {
    if (this.filter === ALL) return 'All tickets';
    if (this.filter === BACKLOG) return 'Backlog';
    return this.selected?.name ?? '';
  }

  /** The chosen sprint's or the Backlog's own usage — never a sum of its tickets.
   *  All tickets has no strip. */
  get strip(): Strip | null {
    if (this.filter === BACKLOG) {
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

  /** The rows shown: every loaded ticket, or only those with AI sessions. */
  get visibleTasks(): TrackerTaskRow[] {
    const all = this.tasks ?? [];
    return this.aiOnly ? all.filter((t) => t.usage.sessions > 0) : all;
  }

  /** "730 tickets", or with the switch on "12 of 730 tickets has AI sessions". */
  get countLine(): string {
    const total = this.tasks?.length ?? 0;
    return this.aiOnly ? `${this.visibleTasks.length} of ${tickets(total)} has AI sessions` : tickets(total);
  }

  /** A ticket's current sprint, or "Backlog" when no sprint lists it. */
  sprintOf(t: TrackerTaskRow): string {
    return t.current_sprint?.name ?? 'Backlog';
  }

  get sharedNote(): string | null {
    return this.tasks?.some((t) => t.usage.shared > 0)
      ? 'A session linked to several tickets counts in full for each; the sprint total counts it once.'
      : null;
  }

  /** Show All tickets, one sprint's (its id), or the Backlog's. */
  async setFilter(value: string) {
    const seq = ++this.#seq;
    this.filter = value;
    this.tasks = null;
    this.tasksError = null;
    this.unlinked = null;
    this.unlinkedError = null;
    const t = this.tracker.id;
    const res =
      value === BACKLOG
        ? await this.#api.tryListBacklogTasks(this.#workspaceId, t)
        : await this.#api.tryListTrackerTasks(this.#workspaceId, t, value === ALL ? undefined : value);
    if (seq !== this.#seq) return;
    if (res.ok) this.tasks = res.data;
    else this.tasksError = failureText(res.error);
  }

  /** Show or hide the chosen sprint's unlinked sessions (a sprint only: All
   *  tickets and the Backlog have no window). */
  async toggleUnlinked() {
    if (this.unlinked) {
      this.unlinked = null;
      return;
    }
    const id = this.filter;
    if (id === ALL || id === BACKLOG) return;
    this.unlinkedError = null;
    const res = await this.#api.tryListUnlinkedSessions(this.#workspaceId, this.tracker.id, id);
    if (id !== this.filter) return;
    if (res.ok) this.unlinked = res.data;
    else this.unlinkedError = failureText(res.error);
  }
}

export class TicketsState {
  #api: SprintsApi;
  #workspaceId: string;

  view = $state<View>('loading');
  loadError = $state<string | null>(null);
  lanes = $state<TicketLane[]>([]);
  ticket = $state<TrackerTaskDetail | null>(null);
  ticketError = $state<string | null>(null);
  /** The lane a ticket was opened from: where "Back" returns. */
  ticketLane = $state<TicketLane | null>(null);

  constructor(api: SprintsApi, workspaceId: string) {
    this.#api = api;
    this.#workspaceId = workspaceId;
  }

  /** The board's name when there is one lane; the lanes carry their own names otherwise. */
  get boardTitle(): string {
    return this.lanes.length === 1 ? this.lanes[0].title : '';
  }

  /** "Back to Sprint 13", or with several lanes "Back to Team A · Sprint 13". */
  get backLabel(): string {
    const l = this.ticketLane;
    if (!l) return 'Back';
    return this.lanes.length > 1 ? `Back to ${l.title} · ${l.filterLabel}` : `Back to ${l.filterLabel}`;
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
    const trackers = await this.#api.tryListTrackers(this.#workspaceId);
    if (!trackers.ok) return this.#failed(trackers.error);
    const boards = trackers.data.filter((t) => t.level === 'sprint');
    if (boards.length === 0) {
      this.view = 'not_connected';
      return;
    }
    const reads = await Promise.all(
      boards.map((t) =>
        Promise.all([
          this.#api.tryListSprints(this.#workspaceId, t.id),
          this.#api.tryGetBacklog(this.#workspaceId, t.id),
        ]),
      ),
    );
    const lanes: TicketLane[] = [];
    for (const [i, [sprints, backlog]] of reads.entries()) {
      // Any lane's read failing is the daemon's error: a lane is never silently missing,
      // and a failed backlog read is never "0 tickets".
      if (!sprints.ok) return this.#failed(sprints.error);
      if (!backlog.ok) return this.#failed(backlog.error);
      lanes.push(new TicketLane(this.#api, this.#workspaceId, boards[i], sprints.data, backlog.data));
    }
    this.loadError = null;
    this.lanes = lanes;
    this.view = 'ready';
    await Promise.all(lanes.map((l) => l.setFilter(ALL)));
  }

  #failed(error: Parameters<typeof failureText>[0]) {
    this.loadError = failureText(error);
    this.view = 'daemon_error';
  }

  async openTicket(lane: TicketLane, key: string) {
    this.ticketError = null;
    this.ticketLane = lane;
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
}
