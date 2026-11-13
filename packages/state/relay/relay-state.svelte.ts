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
import type {
  RelayAddToBacklog,
  RelayAnswer,
  RelayCreateRun,
  RelayCreatedRun,
  RelayCrew,
  RelayDecision,
  RelayFacets,
  RelayFilter,
  RelayFloatingReview,
  RelayNow,
  RelayQueue,
  RelayResources,
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
  createRun(body: RelayCreateRun): Promise<ApiResult<RelayCreatedRun>>;
  /** Subscribe to the stream: every event, and each reconnect after the first open
   *  (a broadcast has no replay). Returns the teardown. */
  events(on: (event: RelayStreamEvent) => void, onReconnect: () => void): () => void;
}

/** What a loader hands the page: the two reads, or why each could not be read. */
export interface RelayReads {
  now: RelayNow | null;
  nowError: ApiError | null;
  queue: RelayQueue | null;
  queueError: ApiError | null;
}

/** What the create-sprint form fills in; the state adds the selection and the crew. */
export type RelaySprintDraft = Omit<RelayCreateRun, 'item_ids' | 'crew_id'>;

/** The parts of `relay.now` that are the read's summary, not entity rows. */
interface NowSummary {
  crews: RelayCrew[];
  facets: RelayFacets;
  triage: RelayTriageSource[];
  today: RelayToday;
  resources: RelayResources;
}

export interface RelayStateOptions {
  /** Who answers a decision: the desktop, or the dōjō. */
  surface?: 'desktop' | 'dojo';
}

/** An event that carries its row (#418 shape), by the entity state that holds it. */
const ROW_ENTITIES = ['lane', 'run', 'decision', 'message', 'item'] as const;
type RowEntity = (typeof ROW_ENTITIES)[number];

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

  // ── The screen's own state ──
  /** The view the toolbar shows: Now, Queue or Crew (CONTRACT §1a). */
  view = $state<'now' | 'queue' | 'crew'>('now');
  /** The pressed stat on Now, narrowing its lists (CONTRACT §1c-0); null = everything. */
  statFilter = $state<'working' | 'waiting' | 'blocked' | 'done' | null>(null);
  composing = $state(false);
  /** Why the last add to the backlog was refused (the row stays where it was). */
  addError = $state<string | null>(null);
  /** Backlog item ids picked for a sprint. Reassigned on every change: a Set is not proxied. */
  selected = $state<Set<string>>(new Set());
  creatingSprint = $state(false);
  /** Why the last sprint was refused (the selection and the form stay as they were). */
  sprintError = $state<string | null>(null);

  #nowInFlight: Promise<void> | null = null;
  #nowAgain = false;
  #queueInFlight: Promise<void> | null = null;
  #queueAgain = false;
  #createInFlight: Promise<string | null> | null = null;

  constructor(source: RelaySource, filter: RelayFilter, reads: RelayReads, options: RelayStateOptions = {}) {
    this.source = source;
    this.filter = filter;
    this.surface = options.surface ?? 'desktop';
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

  openQueue(): void {
    this.view = 'queue';
  }

  // ── The stream ──

  /** One stream event: a row changes that row alone; ids alone re-read what they name. */
  apply(event: RelayStreamEvent): void {
    const data = event.data as Record<string, unknown>;
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
        void this.reReadNow();
        return;
      default:
        // A step is the task screen's; anything else names nothing this state holds.
        return;
    }
  }

  #applyRow(entity: RowEntity, action: RelayStreamEvent['action'], row: { id: string }): void {
    const target = {
      lane: this.lanes,
      run: this.runs,
      decision: this.decisions,
      message: this.messages,
      item: this.items,
    }[entity] as { apply(e: { action: RelayStreamEvent['action']; entity: string; data: { id: string } }): void };
    target.apply({ action, entity, data: row });
  }

  /** Subscribe to the source's stream; returns the teardown (call on destroy). */
  connect(): () => void {
    const stop = this.source.events(
      (event) => this.apply(event),
      () => {
        void this.reReadNow();
        void this.reReadQueue();
      },
    );
    // The read the loader made predates the socket: one read covers the gap.
    void this.reReadNow();
    return stop;
  }

  // ── Reads ──

  async retryNow(): Promise<void> {
    this.reading = true;
    await this.reReadNow();
  }

  async retryQueue(): Promise<void> {
    this.readingQueue = true;
    await this.reReadQueue();
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
    try {
      const answer = await this.source.readNow(this.filter);
      if (answer.ok) {
        this.#hydrateNow(answer.data);
        this.nowError = null;
      } else {
        this.nowError = answer.error;
      }
    } finally {
      this.reading = false;
      this.#nowInFlight = null;
    }
    if (this.#nowAgain) {
      this.#nowAgain = false;
      await this.reReadNow();
    }
  }

  async #readQueue(): Promise<void> {
    try {
      const answer = await this.source.readQueue(this.filter);
      if (answer.ok) {
        this.#hydrateQueue(answer.data);
        this.queueError = null;
      } else {
        this.queueError = answer.error;
      }
    } finally {
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
  async answer(decision: RelayDecision, option: string): Promise<string | null> {
    const answer = await this.source.answer({ decision_id: decision.id, option, note: null, surface: this.surface });
    return answer.ok ? null : failureReason(answer.error);
  }

  /** Queue a message for the crew: its reason when refused, else null. Its event confirms it. */
  async send(body: RelaySendMessage): Promise<string | null> {
    const answer = await this.source.send(body);
    return answer.ok ? null : failureReason(answer.error);
  }

  /** Add a triage row to the backlog by its own identity. The daemon emits nothing for
   *  it yet (#418), so the queue is read again. */
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
    await this.reReadQueue();
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
   *  `sprintError`, the selection kept). One write per press. The run's event re-reads
   *  Now; nothing tells the queue its items were planned yet (#418), so both are read. */
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
    // The screen's crew, named: the daemon never has to pick among several.
    const answer = await this.source.createRun({ ...draft, item_ids, crew_id: this.crew?.id ?? null });
    if (!answer.ok) {
      this.sprintError = failureReason(answer.error);
      return null;
    }
    this.clearSelection();
    this.creatingSprint = false;
    await Promise.all([this.reReadNow(), this.reReadQueue()]);
    return answer.data.run_id;
  }
}
