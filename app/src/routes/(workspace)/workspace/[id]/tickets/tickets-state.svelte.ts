// The workspace window's Tickets screen: one lane per sprint board, side by side
// (D40), each listing its tickets first with the sprint as a filter.
// Spec: docs/spec/screen/workspace-tickets.md, docs/spec/tracker-levels.md.
import { failureNotice } from '@sensei/ui';
import type { ApiResult } from '$lib/api.js';
import { activeMinutes, compactDuration, compactTokens, durationMinutes } from '$lib/sessions-digest.js';
import { errorFor } from '$lib/tracker-errors.js';
import type {
  MetricReason,
  ProjectSdlc,
  SdlcReasons,
  SprintSdlc,
  SprintUsageRow,
  TicketSdlc,
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
  tryGetReasonCodes: (domain: string) => Promise<ApiResult<{ domain: string; reasons: Record<string, MetricReason> }>>;
}

/** The reason domain the SDLC figures' codes live in (tracker-sdlc-metrics S6). */
export const SDLC_REASONS = 'sdlc_metrics';

/** Reason codes' words, by code. */
export type Words = Record<string, MetricReason>;

/** One SDLC figure as shown: its value, or (when null) its reason's summary. */
export interface SdlcFigure {
  key: string;
  label: string;
  text: string;
  /** The text is a reason's summary, not a value. */
  reason: boolean;
}

export interface SdlcRow {
  id: 'cost' | 'defects' | 'timing' | 'rework';
  label: string;
  figures: SdlcFigure[];
  note: string | null;
}

const pct = (x: number) => `${Math.round(x)}%`;
const money = (x: number) => `$${x.toFixed(2)}`;
const count = (x: number) => String(x);
const points = (x: number) => `${Number(x.toFixed(1))} pts`;
const hoursText = (h: number) => compactDuration(Math.round(h * 60));
const minutesText = (m: number) => compactDuration(Math.round(m));

/** A figure's value, or its reason's summary. A code the vocabulary has no words for
 *  is a fault and says so, never passing for "no data". */
function figure(
  key: string,
  label: string,
  value: number | null,
  format: (x: number) => string,
  reasons: SdlcReasons,
  words: Words,
): SdlcFigure {
  if (value != null) return { key, label, text: format(value), reason: false };
  const code = reasons[key];
  if (code == null) return { key, label, text: '—', reason: false };
  return { key, label, text: words[code]?.summary ?? 'Reason not recognised', reason: true };
}

/** A ticket's SDLC group (S8): cost, defects, timing, rework. */
export function ticketSdlcRows(s: TicketSdlc, words: Words): SdlcRow[] {
  const f = (key: keyof TicketSdlc & string, label: string, format: (x: number) => string) =>
    figure(key, label, s[key] as number | null, format, s.reasons, words);
  const unknown = s.runs_unknown ?? 0;
  // While runs are unknown every defect count is a lower bound (S6): an unknown run can
  // hide a build or test failure, an escape needs one, and review defects include escapes.
  const atLeast = (x: number) => (unknown > 0 ? `≥ ${x}` : String(x));
  const holds = (m: number) => minutesText(m) + (s.on_hold_count ? ` ×${s.on_hold_count}` : '');
  return [
    {
      id: 'cost',
      label: 'Cost',
      figures: [
        f('ai_token_cost', 'AI token cost', money),
        f('cost_per_prompt_cycle', 'per prompt cycle', money),
        f('effort_cost', 'effort cost', money),
      ],
      note: null,
    },
    {
      id: 'defects',
      label: 'Defects',
      figures: [
        f('compile_defects', 'compile', atLeast),
        f('test_defects', 'test', atLeast),
        f('review_defects', 'review', atLeast),
        f('escaped_task_defects', 'escaped task', atLeast),
      ],
      note: unknown > 0 ? `${unknown} run${unknown === 1 ? '' : 's'} with unknown outcome` : null,
    },
    {
      id: 'timing',
      label: 'Timing',
      figures: [
        f('execution_hours', 'execution time', hoursText),
        f('lead_hours', 'lead time', hoursText),
        f('on_hold_minutes', 'on hold', holds),
        f('on_hold_ratio', 'on-hold ratio', pct),
      ],
      note: null,
    },
    {
      id: 'rework',
      label: 'Rework',
      figures: [
        f('rework_minutes', 'rework time', minutesText),
        f('rework_cycles', 'rework cycles', count),
        f('post_review_rework_ratio', 'post-review rework', pct),
      ],
      note: null,
    },
  ];
}

/** A sprint's row in its lane header (S8). */
export function sprintSdlcFigures(s: SprintSdlc, words: Words): SdlcFigure[] {
  const f = (key: keyof SprintSdlc & string, label: string, format: (x: number) => string) =>
    figure(key, label, s[key] as number | null, format, s.reasons, words);
  return [
    f('commitment_reliability', 'commitment reliability', pct),
    f('scope_change', 'scope change', pct),
    f('carry_over_rate', 'carry-over', pct),
    f('unplanned_work_ratio', 'unplanned work', pct),
    f('velocity', 'velocity', points),
  ];
}

/** A board's project row in its lane header (S8). */
export function projectSdlcFigures(p: ProjectSdlc, words: Words): SdlcFigure[] {
  const f = (key: keyof ProjectSdlc & string, label: string, format: (x: number) => string) =>
    figure(key, label, p[key] as number | null, format, p.reasons, words);
  return [
    f('predictability', 'predictability', points),
    f('tokens_per_ticket', 'tokens per ticket', compactTokens),
    f('prompts_per_ticket', 'prompts per ticket', (x) => x.toFixed(1)),
  ];
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

/** The API cost at list prices: "$1.24", "$1.24+" when some work could not be
 *  priced (a lower bound), "<$0.01" for a fraction of a cent, "—" when nothing was. */
function apiCost(u: TrackerUsage): string {
  if (u.api_cost == null) return '—';
  const plus = u.api_cost_partial > 0 ? '+' : '';
  const shown = u.api_cost > 0 && u.api_cost < 0.005 ? '<$0.01' : `$${u.api_cost.toFixed(2)}`;
  return shown + plus;
}

/** One usage object as table cells. Unknown is "—", never 0. */
export function usageCells(u: TrackerUsage): UsageCells {
  return {
    sessions: `${u.sessions} (${u.by_source.branch}/${u.by_source.prompt}/${u.by_source.fetch})`,
    active: compactDuration(activeMinutes(u.active_secs)),
    fresh: compactTokens(u.tokens_fresh),
    out: compactTokens(u.tokens_out),
    cost: apiCost(u),
    claude: compactDuration(activeMinutes(u.claude_secs)),
    you: compactDuration(activeMinutes(u.user_secs)),
    wall: compactDuration(activeMinutes(u.wall_secs)),
    cycle: u.first_at && u.last_at ? compactDuration(durationMinutes(u.first_at, u.last_at)) : '—',
  };
}

const tickets = (n: number) => `${n} ticket${n === 1 ? '' : 's'}`;

const REASONS: Record<string, string> = {
  not_started: 'Not started. Usage counts from the sprint’s start.',
  no_dates: 'The tracker has no dates for this sprint, so its sessions can’t be placed in it.',
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
  readonly #words: Words;
  /** All tickets, one sprint's (its id), or the Backlog. */
  filter = $state<string>(ALL);
  /** Narrow the list to tickets with AI sessions. */
  aiOnly = $state(false);
  tasks = $state<TrackerTaskRow[] | null>(null);
  tasksError = $state<string | null>(null);
  unlinked = $state<UnlinkedSession[] | null>(null);
  unlinkedError = $state<string | null>(null);

  /** Notice props for a failed ticket read (packages/ui/notice.js). */
  get tasksNotice() {
    return failureNotice("Couldn't load this board's tickets", this.tasksError);
  }

  /** Notice props for a failed read of the sessions linked to no ticket. */
  get unlinkedNotice() {
    return failureNotice("Couldn't load the sessions not linked to a ticket", this.unlinkedError);
  }

  constructor(
    api: SprintsApi,
    workspaceId: string,
    tracker: TrackerConnection,
    sprints: SprintUsageRow[],
    backlog: BacklogUsage,
    words: Words,
  ) {
    this.#api = api;
    this.#workspaceId = workspaceId;
    this.tracker = tracker;
    this.sprints = sprints;
    this.backlog = backlog;
    this.#words = words;
  }

  /** The lane header's SDLC row: the chosen sprint's own figures, else the board's
   *  project figures (All tickets, Backlog). Never built from the tickets. */
  get sdlcRow(): { kind: 'sprint' | 'project'; figures: SdlcFigure[] } {
    const s = this.selected;
    return s
      ? { kind: 'sprint', figures: sprintSdlcFigures(s.sdlc, this.#words) }
      : { kind: 'project', figures: projectSdlcFigures(this.tracker.sdlc_project, this.#words) };
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

  /** Said when any listed ticket's API cost leaves work unpriced. */
  get costNote(): string | null {
    return this.tasks?.some((t) => t.usage.api_cost_partial > 0)
      ? 'Some AI work could not be priced (subagents or a model with no price), so its API cost is a lower bound.'
      : null;
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
  /** The key of the ticket last opened — what a failed read names. */
  ticketKey = $state<string | null>(null);

  /** Notice props for a failed load: what failed, the reason as the detail. */
  get loadNotice() {
    return failureNotice("Couldn't load this workspace's tickets", this.loadError);
  }

  /** Notice props for a ticket that would not open. */
  get ticketNotice() {
    return failureNotice(`Couldn't open ${this.ticketKey ?? 'that ticket'}`, this.ticketError);
  }
  /** The lane a ticket was opened from: where "Back" returns. */
  ticketLane = $state<TicketLane | null>(null);
  /** The SDLC reason vocabulary, read with the boards. */
  words = $state<Words>({});

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

  /** The open ticket's SDLC group. */
  get ticketSdlc(): SdlcRow[] {
    return this.ticket ? ticketSdlcRows(this.ticket.sdlc, this.words) : [];
  }

  async load() {
    this.view = 'loading';
    const [trackers, reasons] = await Promise.all([
      this.#api.tryListTrackers(this.#workspaceId),
      this.#api.tryGetReasonCodes(SDLC_REASONS),
    ]);
    if (!trackers.ok) return this.#failed(trackers.error);
    // Without the words a null figure could only show its bare code (A10).
    if (!reasons.ok) return this.#failed(reasons.error);
    this.words = reasons.data.reasons;
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
      lanes.push(new TicketLane(this.#api, this.#workspaceId, boards[i], sprints.data, backlog.data, this.words));
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
    this.ticketKey = key;
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
