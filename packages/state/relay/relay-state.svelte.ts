// The relay domain state (#421; design 2026-11-12 §2b, §2c; CONTRACT §1c, §1d, §3e, §3h).
//
// One composite over six entity states, built with a RelaySource — the daemon's SSE
// (`sources/sse.ts`) on the desktop, Supabase realtime on the dōjō. It hydrates from
// the source's reads, applies the stream's events, and keeps the screen's own state
// (view, stat filter, compose, the sprint selection). The page never fetches.
//
// An event that carries its row changes that row alone. Today's daemon names ids only
// (#418), so such an event re-reads once — events while a read is in flight share it,
// and one that lands mid-read earns one read after it. A write goes to the source; the
// event it causes confirms it, except where the daemon emits none yet (the backlog, the
// queue side of a new run), which re-read what they changed. A refusal says why and
// leaves the screen as it was (D83).

import { failureReason, type ApiError, type ApiResult } from '../result.js';
import type { StateStream } from '../source.js';
import type { RelayWaitRowView } from './now-selectors.js';
import type {
  RelayAddToBacklog,
  RelayAddTriage,
  RelayAnswer,
  RelayCancelledRun,
  RelayCreateRun,
  RelayCreatedRun,
  RelayCrew,
  RelayDecision,
  RelayDecisionChoice,
  RelayFacets,
  RelayNowRunTask,
  RelayFilter,
  RelayFloatingReview,
  RelayNow,
  RelayQueue,
  RelayResources,
  RelayRunList,
  RelayRunSummary,
  RelaySendMessage,
  RelayStreamEvent,
  RelayToday,
  RelayTriageItem,
  RelayTriageSource,
} from './types.js';
import {
  DecisionsState,
  ItemsState,
  LanesState,
  MessagesState,
  RunsState,
  TasksState,
  runSummary,
} from './entity-states.svelte.js';

/** Where the relay state reads, listens and writes: the daemon on the desktop, Supabase on the dōjō. */
export interface RelaySource {
  readNow(filter: RelayFilter): Promise<ApiResult<RelayNow>>;
  readQueue(filter: RelayFilter): Promise<ApiResult<RelayQueue>>;
  answer(body: RelayAnswer): Promise<ApiResult<{ action_id: string }>>;
  send(body: RelaySendMessage): Promise<ApiResult<{ message_id: string }>>;
  addToBacklog(body: RelayAddToBacklog): Promise<ApiResult<{ item_id: string }>>;
  addTriage(body: RelayAddTriage): Promise<ApiResult<{ item_id: string }>>;
  createRun(body: RelayCreateRun): Promise<ApiResult<RelayCreatedRun>>;
  /** Every run, open (a draft among them) and ended (#419; the Crew view, #424). */
  readRuns(): Promise<ApiResult<RelayRunList>>;
  startRun(runId: string, crewId: string | null): Promise<ApiResult<RelayCreatedRun>>;
  cancelRun(runId: string): Promise<ApiResult<RelayCancelledRun>>;
  /** Subscribe to the stream: every event, and every open — the first and each
   *  reconnect (a broadcast has no replay). Returns the teardown. */
  events: RelayEvents;
}

/** The relay stream: the package's StateStream (#414) carrying relay events. */
export type RelayEvents = StateStream<RelayStreamEvent>;

/** What a loader hands the page: the two reads, or why each could not be read. */
export interface RelayReads {
  now: RelayNow | null;
  nowError: ApiError | null;
  queue: RelayQueue | null;
  queueError: ApiError | null;
}

/** What the create-sprint form fills in; the state adds the selection and the crew. */
export type RelaySprintDraft = Omit<RelayCreateRun, 'item_ids' | 'crew_id' | 'start'>;

/** The parts of `relay.now` that are the read's summary, not entity rows. */
interface NowSummary {
  crews: RelayCrew[];
  facets: RelayFacets;
  run_tasks: RelayNowRunTask[];
  triage: RelayTriageSource[];
  today: RelayToday;
  resources: RelayResources;
}

export interface RelayStateOptions {
  /** Who answers a decision: the desktop, or the dōjō. */
  surface?: 'desktop' | 'dojo';
  /** Where the person's Lanes view is kept (#422): the surface's own per-viewer store. */
  laneView?: { read(): string | null; write(view: RelayLaneView): void };
}

/** Now's Lanes section: grouped by run, or by agent (#422). */
export type RelayLaneView = 'run' | 'agent';

/** An event that carries its row (#418 shape), by the entity state that holds it. */
const ROW_ENTITIES = ['lane', 'run', 'decision', 'message', 'item'] as const;
type RowEntity = (typeof ROW_ENTITIES)[number];
/** One row the stream delivered, its event keys stripped. */
type RowChange = { entity: RowEntity; action: RelayStreamEvent['action']; row: { id: string } };
/** Keys a row event carries about the event, not the row (#418): what happened, why,
 *  and the id it is named by today beside `id`. */
const EVENT_KEYS: Record<RowEntity, readonly string[]> = {
  lane: ['op', 'reason', 'lane_id'],
  run: ['op', 'reason'],
  decision: ['op', 'reason'],
  message: ['op', 'reason'],
  item: ['op', 'reason'],
};

export class RelayState {
  readonly source: RelaySource;
  readonly filter: RelayFilter;
  readonly surface: 'desktop' | 'dojo';

  // ── The six entity states ──
  readonly lanes = new LanesState();
  readonly tasks = new TasksState();
  readonly runs = new RunsState();
  readonly items = new ItemsState();
  readonly decisions = new DecisionsState();
  readonly messages = new MessagesState();

  /** The read's summary; null while nothing was read (D83: not an empty crew). */
  #summary = $state<NowSummary | null>(null);
  /** The queue was read (an empty queue is a read, not a failure). */
  #queueRead = $state(false);

  nowError = $state<ApiError | null>(null);
  queueError = $state<ApiError | null>(null);
  /** A read the person asked for is in flight: the body says so instead of showing a stale crew. */
  reading = $state(false);
  readingQueue = $state(false);
  /** Why the live stream is down, so what the screen shows may be stale (D83); null while
   *  it is up. The stream's next open clears it. */
  streamFault = $state<string | null>(null);

  // ── The screen's own state ──
  /** Now's Lanes grouped by run or by agent (#422); the person's choice, kept. */
  laneView = $state<RelayLaneView>('run');
  readonly #laneViewStore: RelayStateOptions['laneView'];

  /** The view the toolbar shows: Now, Queue or Crew (CONTRACT §1a). */
  view = $state<'now' | 'queue' | 'crew'>('now');
  /** The pressed stat on Now, narrowing its lists (CONTRACT §1c-0); null = everything. */
  statFilter = $state<'working' | 'waiting' | 'blocked' | 'done' | null>(null);
  composing = $state(false);
  /** Why the last add to the backlog was refused (the row stays where it was). */
  addError = $state<string | null>(null);
  /** Why the last open item the person added was refused (what they typed stays). */
  triageError = $state<string | null>(null);
  /** Wait rows whose nudge is on its way (#419): the button is busy until the source answers. */
  nudging = $state<string[]>([]);
  /** Why a row's last nudge was refused, by row id; a later nudge that is taken clears it. */
  nudgeErrors = $state<Record<string, string>>({});
  /** Backlog item ids picked for a sprint. Reassigned on every change: a Set is not proxied. */
  selected = $state<Set<string>>(new Set());
  creatingSprint = $state(false);
  /** The role AddMember shows the register line for: the coordinator first (#424). */
  addRole = $state<'coordinator' | 'contributor'>('coordinator');
  /** Why the last sprint was refused (the selection and the form stay as they were). */
  sprintError = $state<string | null>(null);
  /** Every run, open and ended (`GET /relay/runs`); null until read. */
  runList = $state<RelayRunList | null>(null);
  /** Why the last read of the runs failed; null when it did not. */
  runsError = $state<string | null>(null);
  /** The run being started or cancelled; null when none is. */
  startingRun = $state<string | null>(null);
  /** Why the last start or cancel was refused. */
  runActionError = $state<string | null>(null);

  #nowInFlight: Promise<void> | null = null;
  #nowAgain = false;
  #queueInFlight: Promise<void> | null = null;
  #queueAgain = false;
  /** Stream rows applied while a read is out, in arrival order: the read's snapshot is
   *  older than them, so they are applied again on top of it when it lands (#450). */
  #nowSince: RowChange[] | null = null;
  #queueSince: RowChange[] | null = null;
  #createInFlight: Promise<string | null> | null = null;
  #runActionInFlight: Promise<string | null> | null = null;

  constructor(source: RelaySource, filter: RelayFilter, reads: RelayReads, options: RelayStateOptions = {}) {
    this.source = source;
    this.filter = filter;
    this.surface = options.surface ?? 'desktop';
    this.#laneViewStore = options.laneView;
    const kept = options.laneView?.read();
    if (kept === 'run' || kept === 'agent') this.laneView = kept;
    if (reads.now) this.#hydrateNow(reads.now);
    this.nowError = reads.nowError;
    if (reads.queue) this.#hydrateQueue(reads.queue);
    this.queueError = reads.queueError;
  }

  #hydrateNow(now: RelayNow): void {
    this.lanes.set(now.lanes);
    this.runs.set(now.runs.map(RunsState.row));
    this.tasks.set(TasksState.rows(now.floating_reviews, now.completed_today));
    this.decisions.set(now.decisions);
    this.messages.set(now.person_messages);
    this.#summary = {
      crews: now.crews,
      facets: now.facets,
      // Every task of an open run (#422): a task event re-reads Now, which brings them.
      run_tasks: now.run_tasks,
      triage: now.triage,
      today: now.today,
      resources: now.resources,
    };
  }

  #hydrateQueue(queue: RelayQueue): void {
    this.items.set(ItemsState.rows(queue.triage, queue.backlog));
    this.#queueRead = true;
  }

  /** `relay.now` as the screen reads it, assembled from the entity states; null without a read. */
  readonly #now = $derived.by((): RelayNow | null => {
    const s = this.#summary;
    if (!s) return null;
    return {
      crews: s.crews,
      facets: s.facets,
      lanes: this.lanes.items,
      runs: this.runs.items.map(runSummary),
      run_tasks: s.run_tasks,
      floating_reviews: this.tasks.floating,
      decisions: this.decisions.items,
      person_messages: this.messages.items,
      triage: s.triage,
      completed_today: this.tasks.completed,
      today: s.today,
      resources: s.resources,
    };
  });

  get now(): RelayNow | null {
    return this.#now;
  }

  /** The queue as the screen reads it; null without a read. */
  readonly #queue = $derived.by((): RelayQueue | null =>
    this.#queueRead ? { triage: this.items.triage, backlog: this.items.backlog } : null,
  );

  get queue(): RelayQueue | null {
    return this.#queue;
  }

  /** The crew this screen is on, or null while there is none (or no read). */
  get crew(): RelayCrew | null {
    return this.#summary?.crews[0] ?? null;
  }

  get hasCrew(): boolean {
    return Boolean(this.crew);
  }

  /** Open sprint runs in the daemon's order (the band); null while there is no read. */
  get runsBand(): RelayRunSummary[] | null {
    return this.#summary ? this.runs.items.map(runSummary) : null;
  }

  /** Reports waiting for a reviewer, oldest first; null while there is no read. */
  get floatingReviews(): RelayFloatingReview[] | null {
    return this.#summary ? this.tasks.floating : null;
  }

  /** The Queue's count: both halves, so it never claims an empty queue; null while unread. */
  get queueCount(): number | null {
    return this.#queueRead ? this.items.items.length : null;
  }

  setLaneView(view: RelayLaneView): void {
    this.laneView = view;
    this.#laneViewStore?.write(view);
  }

  openQueue(): void {
    this.view = 'queue';
  }

  // ── The stream ──

  /** One stream event: a row changes that row alone; ids alone re-read what they name. */
  apply(event: RelayStreamEvent): void {
    const data = event.data as Record<string, unknown>;
    // A run changed: the run list (drafts, past runs) is read whole, once it was read.
    if (event.entity === 'run' && this.runList) void this.readRuns();
    if ((ROW_ENTITIES as readonly string[]).includes(event.entity) && typeof data.id === 'string') {
      this.#applyRow(event.entity as RowEntity, event.action, data as { id: string });
      return;
    }
    switch (event.entity) {
      case 'run':
        void this.reReadNow();
        void this.reReadQueue();
        return;
      case 'lane':
      case 'task':
      case 'relay':
      case 'decision':
      case 'message':
        void this.reReadNow();
        return;
      case 'item':
        // A queue row named by its id alone (a relay.items change, #414): read the queue.
        void this.reReadQueue();
        return;
      default:
        // A step is the task screen's; anything else names nothing this state holds.
        return;
    }
  }

  #applyRow(entity: RowEntity, action: RelayStreamEvent['action'], data: { id: string }): void {
    // The event's own keys say what happened; they are not the row's fields.
    const row: Record<string, unknown> & { id: string } = { ...data };
    for (const key of EVENT_KEYS[entity]) delete row[key];
    const change: RowChange = { entity, action, row };
    this.#applyChange(change);
    // A read out while this row arrived holds an older snapshot: the row is applied
    // again when it lands (#426 found a lane vanish from the crew mid-read; #450).
    (entity === 'item' ? this.#queueSince : this.#nowSince)?.push(change);
    // A lane in a crew this screen has not read: a coordinator minted it — read the crew.
    const crew = entity === 'lane' && action !== 'remove' ? (row as { crew_id?: unknown }).crew_id : undefined;
    if (typeof crew === 'string' && this.#summary && !this.#summary.crews.some((c) => c.id === crew)) {
      void this.reReadNow();
    }
  }

  #applyChange({ entity, action, row }: RowChange): void {
    const target = {
      lane: this.lanes,
      run: this.runs,
      decision: this.decisions,
      message: this.messages,
      item: this.items,
    }[entity] as {
      get(id: string): unknown;
      apply(e: { action: RelayStreamEvent['action']; entity: string; data: { id: string } }): void;
    };
    // The daemon adds a row it may already hold (a lane re-registers): restate it.
    const restate = action === 'add' && target.get(row.id) !== undefined;
    target.apply({ action: restate ? 'update' : action, entity, data: row });
  }

  /** Subscribe to the source's stream; returns the teardown (call on destroy). Each open
   *  of the stream reads Now and the queue: the loader's read predates the first open,
   *  and a broadcast replays nothing missed while it was down. */
  connect(): () => void {
    return this.source.events(
      (event) => this.apply(event),
      () => {
        this.streamFault = null;
        void this.reReadNow();
        void this.reReadQueue();
      },
      (reason) => {
        this.streamFault = reason;
      },
    );
  }

  // ── Reads ──

  /** The person's Retry: one read per press, and a press while one is in flight shares
   *  it — nothing changed between the presses, so no catch-up read follows. */
  async retryNow(): Promise<void> {
    this.reading = true;
    await (this.#nowInFlight ?? this.reReadNow());
  }

  async retryQueue(): Promise<void> {
    this.readingQueue = true;
    await (this.#queueInFlight ?? this.reReadQueue());
  }

  /** Read Now again. One read at a time: a call while one is in flight shares it and
   *  earns one more read after it, so a change that landed mid-read is not missed. */
  reReadNow(): Promise<void> {
    if (this.#nowInFlight) {
      this.#nowAgain = true;
      return this.#nowInFlight;
    }
    this.#nowInFlight = this.#readNow();
    return this.#nowInFlight;
  }

  reReadQueue(): Promise<void> {
    if (this.#queueInFlight) {
      this.#queueAgain = true;
      return this.#queueInFlight;
    }
    this.#queueInFlight = this.#readQueue();
    return this.#queueInFlight;
  }

  async #readNow(): Promise<void> {
    const since: RowChange[] = [];
    this.#nowSince = since;
    try {
      const answer = await this.source.readNow(this.filter);
      if (answer.ok) {
        this.#hydrateNow(answer.data);
        for (const change of since) this.#applyChange(change);
        this.nowError = null;
      } else {
        this.nowError = answer.error;
      }
    } finally {
      this.#nowSince = null;
      this.reading = false;
      this.#nowInFlight = null;
    }
    if (this.#nowAgain) {
      this.#nowAgain = false;
      await this.reReadNow();
    }
  }

  async #readQueue(): Promise<void> {
    const since: RowChange[] = [];
    this.#queueSince = since;
    try {
      const answer = await this.source.readQueue(this.filter);
      if (answer.ok) {
        this.#hydrateQueue(answer.data);
        for (const change of since) this.#applyChange(change);
        this.queueError = null;
      } else {
        this.queueError = answer.error;
      }
    } finally {
      this.#queueSince = null;
      this.readingQueue = false;
      this.#queueInFlight = null;
    }
    if (this.#queueAgain) {
      this.#queueAgain = false;
      await this.reReadQueue();
    }
  }

  // ── The person's writes ──

  /** Answer a decision: the reason it was refused, or null when it was taken. Its event confirms it. */
  async answer(decision: RelayDecision, choice: RelayDecisionChoice): Promise<string | null> {
    const answer = await this.source.answer({ decision_id: decision.id, ...choice, note: null, surface: this.surface });
    return answer.ok ? null : failureReason(answer.error);
  }

  /** Queue a message for the crew: its reason when refused, else null. Its event confirms it. */
  async send(body: RelaySendMessage): Promise<string | null> {
    const answer = await this.source.send(body);
    return answer.ok ? null : failureReason(answer.error);
  }

  /** Nudge the party a wait is on (#419): the row's message, from this surface. Its reason
   *  when refused, else null; the message's event re-reads Now, which then shows the nudge. */
  async nudge(row: Pick<RelayWaitRowView, 'id' | 'nudge'>): Promise<string | null> {
    if (this.nudging.includes(row.id)) return null;
    this.nudging = [...this.nudging, row.id];
    try {
      const reason = await this.send({ ...row.nudge.message, surface: this.surface });
      const { [row.id]: _, ...rest } = this.nudgeErrors;
      this.nudgeErrors = reason === null ? rest : { ...rest, [row.id]: reason };
      return reason;
    } finally {
      this.nudging = this.nudging.filter((id) => id !== row.id);
    }
  }

  /** Add a triage row to the backlog by its own identity; its item events confirm it
   *  (#418). */
  async addToBacklog(item: RelayTriageItem): Promise<string | null> {
    this.addError = null;
    const answer = await this.source.addToBacklog(
      item.item_id !== null
        ? { item_id: item.item_id }
        : { ticket_provider: item.ticket.provider, ticket_key: item.ticket.key },
    );
    if (!answer.ok) {
      this.addError = failureReason(answer.error);
      return this.addError;
    }
    return null;
  }

  /** Add an open item to triage; its item event confirms it (#418). A refusal is the
   *  reason, in `triageError`. */
  async addTriage(body: RelayAddTriage): Promise<string | null> {
    this.triageError = null;
    const answer = await this.source.addTriage(body);
    if (!answer.ok) {
      this.triageError = failureReason(answer.error);
      return this.triageError;
    }
    return null;
  }

  // ── The sprint selection (queue cutover) ──

  get selectedCount(): number {
    return this.selected.size;
  }

  isSelected(itemId: string): boolean {
    return this.selected.has(itemId);
  }

  toggleSelected(itemId: string): void {
    const next = new Set(this.selected);
    if (next.has(itemId)) next.delete(itemId);
    else next.add(itemId);
    this.selected = next;
  }

  clearSelection(): void {
    this.selected = new Set();
  }

  /** Mint a sprint over the selection: the run's id, or null when refused (the reason in
   *  `sprintError`, the selection kept). One write per press. The run's row and its
   *  items' removals confirm it (#418): nothing is read back. */
  async createSprint(draft: RelaySprintDraft): Promise<string | null> {
    if (this.#createInFlight) return this.#createInFlight;
    // Cleared when this write settles — not inside it: a refusal that never awaits would
    // clear the guard before it was set, and every later press would get the old answer.
    const write = this.#createSprint(draft).finally(() => {
      if (this.#createInFlight === write) this.#createInFlight = null;
    });
    this.#createInFlight = write;
    return write;
  }

  async #createSprint(draft: RelaySprintDraft): Promise<string | null> {
    this.sprintError = null;
    const item_ids = [...this.selected];
    if (item_ids.length === 0) {
      this.sprintError = 'Pick at least one backlog item for the sprint.';
      return null;
    }
    // The screen's crew, named: the daemon never has to pick among several. With a crew
    // the run starts in the same step; without one it waits as a draft (#419).
    const crew_id = this.crew?.id ?? null;
    const answer = await this.source.createRun({ ...draft, item_ids, crew_id, start: crew_id !== null });
    if (!answer.ok) {
      this.sprintError = failureReason(answer.error);
      return null;
    }
    this.clearSelection();
    this.creatingSprint = false;
    return answer.data.run_id;
  }

  // ── The run list (#424): a draft to start, past runs ──

  /** Read every run, open and ended. A failure says so and keeps what was read before. */
  async readRuns(): Promise<void> {
    const answer = await this.source.readRuns();
    if (!answer.ok) {
      this.runsError = failureReason(answer.error);
      return;
    }
    this.runsError = null;
    this.runList = answer.data;
  }

  /** Start a draft on the screen's crew. Null when taken; else why it was refused. One
   *  write per press; the runs are read again after it. */
  startRun(runId: string): Promise<string | null> {
    return this.#runAction(runId, () => this.source.startRun(runId, this.crew?.id ?? null));
  }

  /** Cancel a run (its open tasks end; planned items go back to the backlog). */
  cancelRun(runId: string): Promise<string | null> {
    return this.#runAction(runId, () => this.source.cancelRun(runId));
  }

  #runAction(runId: string, write: () => Promise<ApiResult<unknown>>): Promise<string | null> {
    if (this.#runActionInFlight) return this.#runActionInFlight;
    const action = (async () => {
      this.startingRun = runId;
      this.runActionError = null;
      const answer = await write();
      if (!answer.ok) {
        this.runActionError = failureReason(answer.error);
        return this.runActionError;
      }
      await this.readRuns();
      return null;
    })().finally(() => {
      this.startingRun = null;
      if (this.#runActionInFlight === action) this.#runActionInFlight = null;
    });
    this.#runActionInFlight = action;
    return action;
  }
}
