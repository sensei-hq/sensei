// The workspace window's Sprints screen: one lane per sprint board, side by side
// (D40). Spec: docs/spec/screen/workspace-sprints.md, docs/spec/tracker-levels.md.
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
}

/** One sprint board's sprints. */
interface Lane {
  tracker: TrackerConnection;
  sprints: SprintUsageRow[];
}

export interface LaneView {
  id: string;
  title: string;
  rows: SprintRowView[];
}

const boardName = (t: TrackerConnection) => t.board_name ?? `board #${t.board_ref}`;

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
  #lanes = $state<Lane[]>([]);
  selectedId = $state<string | null>(null);
  tasks = $state<TrackerTaskRow[] | null>(null);
  tasksError = $state<string | null>(null);
  ticket = $state<TrackerTaskDetail | null>(null);
  ticketError = $state<string | null>(null);
  unlinked = $state<UnlinkedSession[] | null>(null);
  unlinkedError = $state<string | null>(null);

  constructor(api: SprintsApi, workspaceId: string) {
    this.#api = api;
    this.#workspaceId = workspaceId;
  }

  /** The board's name when there is one lane; the lanes carry their own names otherwise. */
  get boardTitle(): string {
    return this.#lanes.length === 1 ? boardName(this.#lanes[0].tracker) : '';
  }

  /** The lane holding `sprintId`. */
  #laneOf(sprintId: string | null): Lane | null {
    return this.#lanes.find((l) => l.sprints.some((s) => s.id === sprintId)) ?? null;
  }

  get selected(): SprintUsageRow | null {
    return this.#laneOf(this.selectedId)?.sprints.find((s) => s.id === this.selectedId) ?? null;
  }

  get lanes(): LaneView[] {
    return this.#lanes.map((l) => ({
      id: l.tracker.id,
      title: boardName(l.tracker),
      rows: l.sprints.map((s) => ({
        id: s.id,
        name: s.name,
        state: s.state,
        dates: dateRange(s),
        tickets: `${s.tasks} ticket${s.tasks === 1 ? '' : 's'}`,
        current: s.id === this.selectedId,
      })),
    }));
  }

  /** The selected sprint's own usage — never a sum of its tickets. */
  get strip(): Strip | null {
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
    const trackers = await this.#api.tryListTrackers(this.#workspaceId);
    if (!trackers.ok) {
      this.loadError = failureText(trackers.error);
      this.view = 'daemon_error';
      return;
    }
    const boards = trackers.data.filter((t) => t.level === 'sprint');
    if (boards.length === 0) {
      this.view = 'not_connected';
      return;
    }
    const reads = await Promise.all(boards.map((t) => this.#api.tryListSprints(this.#workspaceId, t.id)));
    const lanes: Lane[] = [];
    for (const [i, res] of reads.entries()) {
      if (!res.ok) {
        this.loadError = failureText(res.error);
        this.view = 'daemon_error';
        return;
      }
      lanes.push({ tracker: boards[i], sprints: res.data });
    }
    this.loadError = null;
    this.#lanes = lanes;
    this.view = 'ready';
    const first = lanes[0].sprints.find((s) => s.state === 'active') ?? lanes[0].sprints[0];
    if (first) await this.select(first.id);
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
    const lane = this.#laneOf(id);
    if (!lane) return;
    const res = await this.#api.tryListTrackerTasks(this.#workspaceId, lane.tracker.id, id);
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
    const lane = this.#laneOf(id);
    if (!id || !lane) return;
    this.unlinkedError = null;
    const res = await this.#api.tryListUnlinkedSessions(this.#workspaceId, lane.tracker.id, id);
    if (id !== this.selectedId) return;
    if (res.ok) this.unlinked = res.data;
    else this.unlinkedError = failureText(res.error);
  }
}
