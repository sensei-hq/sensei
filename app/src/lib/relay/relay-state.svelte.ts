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
  RelayDecision,
  RelayFilter,
  RelayNow,
  RelayQueue,
  RelaySendMessage,
  RelayTriageItem,
} from '$lib/relay-api.js';

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

  tab = $state<'now' | 'queue'>('now');
  composing = $state(false);
  /** Why the last add to the backlog was refused (the row stays where it was). */
  addError = $state<string | null>(null);

  #nowInFlight: Promise<void> | null = null;
  #queueInFlight: Promise<void> | null = null;

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
    this.tab = 'queue';
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
