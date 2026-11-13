// Relay · the crew screen's state (D90; CONTRACT §1c, §1d, §3e, §3h, §3h-bis).
//
// The loader reads; this class keeps what it read and is the only thing that reads
// again or writes. A write is one write per press (single-flight): the person's
// second press while the first is in flight is not a second write. A write that is
// refused says why and leaves the screen as it was — the daemon decides what
// happened, the screen does not guess (D83).
import type { ApiResult, ApiError } from '$lib/api.js';
import { failureReason } from '$lib/api.js';
import type {
  RelayApi,
  RelayCreateRun,
  RelayDecision,
  RelayFilter,
  RelayFloatingReview,
  RelayNow,
  RelayQueue,
  RelayRunSummary,
  RelaySendMessage,
  RelayTriageItem,
} from '$lib/relay-api.js';

/** What the create-sprint form fills in; the state adds the selection and the crew. */
export type RelaySprintDraft = Omit<RelayCreateRun, 'item_ids' | 'crew_id'>;

/** What the loader hands the page: the two reads, or why each could not be read. */
export interface RelayReads {
  now: RelayNow | null;
  nowError: ApiError | null;
  queue: RelayQueue | null;
  queueError: ApiError | null;
}

export class RelayState {
  readonly api: RelayApi;
  readonly filter: RelayFilter;

  now = $state<RelayNow | null>(null);
  nowError = $state<ApiError | null>(null);
  queue = $state<RelayQueue | null>(null);
  queueError = $state<ApiError | null>(null);

  /** A read in flight: the body says so instead of showing a stale crew. */
  reading = $state(false);
  readingQueue = $state(false);

  /** The view the toolbar shows: Now, Queue or Crew (CONTRACT §1a). */
  view = $state<'now' | 'queue' | 'crew'>('now');
  /** The pressed stat on Now, narrowing its lists (CONTRACT §1c-0); null = everything. */
  statFilter = $state<'working' | 'waiting' | 'blocked' | 'done' | null>(null);
  composing = $state(false);
  /** Why the last add to the backlog was refused (the row stays where it was). */
  addError = $state<string | null>(null);

  // ── Sprints (queue cutover) ──
  /** Backlog item ids picked for a sprint (the queue tab's checkboxes). Reassigned
   *  on every change: a Set is not proxied, a fresh one is what the screen sees. */
  selected = $state<Set<string>>(new Set());
  /** The create-sprint form is open. */
  creatingSprint = $state(false);
  /** Why the last sprint was refused (the selection and the form stay as they were). */
  sprintError = $state<string | null>(null);

  #nowInFlight: Promise<void> | null = null;
  #queueInFlight: Promise<void> | null = null;
  #createInFlight: Promise<string | null> | null = null;

  constructor(reads: RelayReads, api: RelayApi, filter: RelayFilter) {
    this.api = api;
    this.filter = filter;
    this.now = reads.now;
    this.nowError = reads.nowError;
    this.queue = reads.queue;
    this.queueError = reads.queueError;
  }

  /** The crew this screen is on, or null while there is none (or the read failed). */
  get crew() {
    return this.now?.crews[0] ?? null;
  }

  /** True when the screen has a crew to draw: decisions, lanes, compose, tabs. */
  get hasCrew(): boolean {
    return Boolean(this.crew);
  }

  openQueue(): void {
    this.view = 'queue';
  }

  /** Open sprint runs, in the order the daemon created them (the band); null while
   *  there is no read (D83: a failed read is not an empty band). */
  get runs(): RelayRunSummary[] | null {
    return this.now ? this.now.runs : null;
  }

  /** Reports waiting for a reviewer, oldest first; null while there is no read. */
  get floatingReviews(): RelayFloatingReview[] | null {
    return this.now ? this.now.floating_reviews : null;
  }

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

  /** Mint a sprint over the selection: the run's id, or null when refused (the reason
   *  in `sprintError`, the selection kept). One write per press; on success the daemon's
   *  rows are read back (the run, Task 0, the planned items) and the selection clears. */
  async createSprint(draft: RelaySprintDraft): Promise<string | null> {
    if (this.#createInFlight) return this.#createInFlight;
    this.#createInFlight = this.#createSprint(draft);
    return this.#createInFlight;
  }

  async #createSprint(draft: RelaySprintDraft): Promise<string | null> {
    this.sprintError = null;
    try {
      const item_ids = [...this.selected];
      if (item_ids.length === 0) {
        this.sprintError = 'Pick at least one backlog item for the sprint.';
        return null;
      }
      // The screen's crew, named: the daemon never has to pick among several.
      const answer = await this.api.tryCreateRun({
        ...draft,
        item_ids,
        crew_id: this.crew?.id ?? null,
      });
      if (!answer.ok) {
        this.sprintError = failureReason(answer.error);
        return null;
      }
      this.clearSelection();
      this.creatingSprint = false;
      await Promise.all([this.reReadNow(), this.reReadQueue()]);
      return answer.data.run_id;
    } finally {
      this.#createInFlight = null;
    }
  }

  /** The queue's count for its tab: both halves, so it never claims an empty queue. */
  get queueCount(): number | null {
    if (!this.queue) return null;
    return this.queue.triage.length + this.queue.backlog.length;
  }

  async retryNow(): Promise<void> {
    if (this.#nowInFlight) return this.#nowInFlight;
    this.#nowInFlight = this.#readNow(true);
    return this.#nowInFlight;
  }

  async retryQueue(): Promise<void> {
    if (this.#queueInFlight) return this.#queueInFlight;
    this.#queueInFlight = this.#readQueue(true);
    return this.#queueInFlight;
  }

  async #readNow(reading: boolean): Promise<void> {
    if (reading) this.reading = true;
    try {
      const answer = await this.api.tryGetNow(this.filter);
      if (answer.ok) {
        this.now = answer.data;
        this.nowError = null;
      } else {
        this.nowError = answer.error;
      }
    } finally {
      this.reading = false;
      this.#nowInFlight = null;
    }
  }

  async #readQueue(reading: boolean): Promise<void> {
    if (reading) this.readingQueue = true;
    try {
      const answer = await this.api.tryGetQueue(this.filter);
      if (answer.ok) {
        this.queue = answer.data;
        this.queueError = null;
      } else {
        this.queueError = answer.error;
      }
    } finally {
      this.readingQueue = false;
      this.#queueInFlight = null;
    }
  }

  /** Answer a decision: the reason it was refused, or null when it was taken. */
  async answer(decision: RelayDecision, option: string): Promise<string | null> {
    const answer = await this.api.tryAnswerDecision({
      decision_id: decision.id,
      option,
      note: null,
      surface: 'desktop',
    });
    if (!answer.ok) return failureReason(answer.error);
    // The daemon records it; the screen reads again rather than removing the row itself.
    await this.reReadNow();
    return null;
  }

  /** Add a triage row to the backlog by its own identity, then read the queue again. */
  async addToBacklog(item: RelayTriageItem): Promise<string | null> {
    this.addError = null;
    const answer: ApiResult<{ item_id: string }> = await this.api.tryAddToBacklog(
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

  /** Queue a message for the crew: its reason when refused, else null. */
  async send(body: RelaySendMessage): Promise<string | null> {
    const answer = await this.api.trySendMessage(body);
    return answer.ok ? null : failureReason(answer.error);
  }

  async reReadNow(): Promise<void> {
    await this.#readNow(false);
  }

  async reReadQueue(): Promise<void> {
    await this.#readQueue(false);
  }
}
