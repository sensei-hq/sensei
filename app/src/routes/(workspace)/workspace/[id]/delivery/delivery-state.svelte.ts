// The workspace window's Delivery Metrics screen: each sprint board's sprints, with
// their figures and the AI work in your sessions behind them.
// Spec: docs/spec/daemon/tracker-delivery-metrics.md D2 (B1–B3, B5).
import { formatDayLabel } from '$lib/metrics/metric-view.js';
import {
  SDLC_REASONS,
  projectSdlcFigures,
  reasonText,
  sprintSdlcFigures,
  sprintWindowReason,
  type SdlcFigure,
  type Words,
} from '$lib/sdlc-figures.js';
import type { SprintFacts, SprintUsageRow, TrackerConnection } from '$lib/types.js';
import { boardName, failureText, usageCells, type SprintsApi } from '../tickets/tickets-state.svelte.js';

/** The four daemon reads this screen makes, and no other (D2, gate 5). Team figures
 *  live in the dōjō only (B5); nothing here reaches `/api/dojo/*`. */
export type DeliveryApi = Pick<
  SprintsApi,
  'tryListTrackers' | 'tryListSprints' | 'tryListTrackerTasks' | 'tryGetReasonCodes'
>;

/** Where a ticket opens on Tickets. */
export function ticketHref(workspaceId: string, trackerId: string, key: string): string {
  const enc = encodeURIComponent;
  return `/workspace/${enc(workspaceId)}/tickets?board=${enc(trackerId)}&ticket=${enc(key)}`;
}

/** A sprint's AI work in your sessions, or why it has none to show. */
export type Effort =
  | { kind: 'cells'; active: string; fresh: string; cost: string }
  | { kind: 'reason'; text: string };

export interface SprintRow {
  id: string;
  name: string;
  state: SprintUsageRow['state'];
  dates: string;
  figures: SdlcFigure[];
  effort: Effort;
}

export interface Fact {
  key: keyof SprintFacts & string;
  label: string;
  value: string;
}

export interface TicketLink {
  key: string;
  title: string;
  href: string;
}

/** The project figures read from Jira; the rest come from your sessions (spec D1). */
const BOARD_FACTS = new Set(['predictability']);

const FACTS: Array<[Fact['key'], string]> = [
  ['committed', 'committed'],
  ['added', 'added'],
  ['removed', 'removed'],
  ['completed_committed', 'completed committed'],
  ['completed_added', 'completed added'],
  ['carried_over', 'carried over'],
];

const day = (iso: string | null) => (iso ? formatDayLabel(iso) : null);

function dates(s: SprintUsageRow): string {
  const from = day(s.start_at);
  const to = day(s.complete_at ?? s.end_at);
  return from && to ? `${from} – ${to}` : (from ?? to ?? 'No dates');
}

function effort(s: SprintUsageRow): Effort {
  if (s.reason) return { kind: 'reason', text: sprintWindowReason(s.reason) };
  if (!s.linked_sprint) return { kind: 'reason', text: 'sensei returned no usage for this sprint.' };
  const c = usageCells(s.linked_sprint);
  return { kind: 'cells', active: c.active, fresh: c.fresh, cost: c.cost };
}

/** One sprint board: its project row, its sprints, and the one selected. */
export class DeliveryBoard {
  #api: DeliveryApi;
  #workspaceId: string;
  readonly #words: Words;
  /** Bumped per selection, so a slower earlier answer is dropped. */
  #seq = 0;

  readonly tracker: TrackerConnection;
  /** In the daemon's order (start date newest first, undated last); never re-sorted. */
  readonly sprints: SprintUsageRow[];
  selectedId = $state<string | null>(null);
  tickets = $state<TicketLink[] | null>(null);
  ticketsError = $state<string | null>(null);

  constructor(api: DeliveryApi, workspaceId: string, tracker: TrackerConnection, sprints: SprintUsageRow[], words: Words) {
    this.#api = api;
    this.#workspaceId = workspaceId;
    this.tracker = tracker;
    this.sprints = sprints;
    this.#words = words;
  }

  get id(): string {
    return this.tracker.id;
  }

  get title(): string {
    return boardName(this.tracker);
  }

  get project(): SdlcFigure[] {
    return projectSdlcFigures(this.tracker.sdlc_project, this.#words);
  }

  /** The project row's board fact: predictability, read from Jira (D1). */
  get projectBoard(): SdlcFigure[] {
    return this.project.filter((f) => BOARD_FACTS.has(f.key));
  }

  /** The project row's figures from your sessions: tokens and prompts per ticket (D1). */
  get projectYours(): SdlcFigure[] {
    return this.project.filter((f) => !BOARD_FACTS.has(f.key));
  }

  get rows(): SprintRow[] {
    return this.sprints.map((s) => ({
      id: s.id,
      name: s.name,
      state: s.state,
      dates: dates(s),
      figures: sprintSdlcFigures(s.sdlc, this.#words),
      effort: effort(s),
    }));
  }

  get selected(): SprintUsageRow | null {
    return this.sprints.find((s) => s.id === this.selectedId) ?? null;
  }

  /** The selected sprint's counts; null when it has no replay (see `factsNote`). */
  get facts(): Fact[] | null {
    const f = this.selected?.sdlc.facts;
    return f ? FACTS.map(([key, label]) => ({ key, label, value: String(f[key]) })) : null;
  }

  /** Why the counts are missing, or the tickets whose status is unknown. */
  get factsNote(): string | null {
    const s = this.selected;
    if (!s) return null;
    const f = s.sdlc.facts;
    if (!f) {
      const code = s.sdlc.reasons.commitment_reliability;
      return code ? reasonText(code, this.#words) : null;
    }
    const n = f.unknown_status;
    return n > 0 ? `${n} ticket${n === 1 ? '' : 's'} with an unknown status` : null;
  }

  /** Select a sprint and read its tickets. */
  async select(id: string) {
    const seq = ++this.#seq;
    this.selectedId = id;
    this.tickets = null;
    this.ticketsError = null;
    const res = await this.#api.tryListTrackerTasks(this.#workspaceId, this.tracker.id, id);
    if (seq !== this.#seq) return;
    if (!res.ok) {
      this.ticketsError = failureText(res.error);
      return;
    }
    this.tickets = res.data.map((t) => ({
      key: t.key,
      title: t.summary ?? '',
      href: ticketHref(this.#workspaceId, this.tracker.id, t.key),
    }));
  }
}

type View = 'loading' | 'not_connected' | 'daemon_error' | 'ready';

export class DeliveryState {
  #api: DeliveryApi;
  #workspaceId: string;

  view = $state<View>('loading');
  loadError = $state<string | null>(null);
  boards = $state<DeliveryBoard[]>([]);

  constructor(api: DeliveryApi, workspaceId: string) {
    this.#api = api;
    this.#workspaceId = workspaceId;
  }

  async load() {
    this.view = 'loading';
    this.boards = [];
    const [trackers, reasons] = await Promise.all([
      this.#api.tryListTrackers(this.#workspaceId),
      this.#api.tryGetReasonCodes(SDLC_REASONS),
    ]);
    if (!trackers.ok) return this.#failed(trackers.error);
    // Without the words a null figure could only show its bare code (A10).
    if (!reasons.ok) return this.#failed(reasons.error);
    const words = reasons.data.reasons;
    const sprintBoards = trackers.data.filter((t) => t.level === 'sprint');
    if (sprintBoards.length === 0) {
      this.view = 'not_connected';
      return;
    }
    const reads = await Promise.all(sprintBoards.map((t) => this.#api.tryListSprints(this.#workspaceId, t.id)));
    const boards: DeliveryBoard[] = [];
    for (const [i, sprints] of reads.entries()) {
      // A board's read failing is the daemon's error: a board is never silently missing.
      if (!sprints.ok) return this.#failed(sprints.error);
      boards.push(new DeliveryBoard(this.#api, this.#workspaceId, sprintBoards[i], sprints.data, words));
    }
    this.loadError = null;
    this.boards = boards;
    this.view = 'ready';
  }

  #failed(error: Parameters<typeof failureText>[0]) {
    this.loadError = failureText(error);
    this.view = 'daemon_error';
  }
}
