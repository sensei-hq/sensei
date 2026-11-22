import type { StreamFrame } from '../index.js';
import type { StateStream } from '../source.js';
import { failureReason, type ApiResult } from '../result.js';
import type {
  OpsAbandonedFeed,
  OpsBackoffView,
  OpsFailedTask,
  OpsIndexError,
  OpsKindDepth,
  OpsQueueListing,
  OpsTaskView,
} from './types.js';
import { WaveState, waveProgress, type WaveRow } from './wave-state.svelte.js';

export type { WaveRow };
export { waveProgress };

// JobsState observes the daemon's task queue: one hydrate at attach, then
// live deltas from job frames. The streams stay authoritative for membership
// (queued/started/terminal); a 15s poller refreshes waits and blocked-on
// edges the streams never re-emit, and is the recovery path when the daemon
// was down at attach time. The failures feed (failed tasks, abandonments,
// index errors) loads on attach and on explicit refresh only — failures are
// history, and a feed that fails closed must fault itself, never the daemon.

/** What JobsState needs of its source: the queue read, the failures reads, and the stream. */
export interface JobsSource {
  readQueue: () => Promise<ApiResult<OpsQueueListing>>;
  readFailed: () => Promise<ApiResult<{ tasks: OpsFailedTask[] }>>;
  readAbandoned: () => Promise<ApiResult<OpsAbandonedFeed>>;
  readIndexErrors: () => Promise<ApiResult<OpsIndexError[]>>;
  events: StateStream<StreamFrame>;
}

/** The poller cadence in ms — waits age in seconds, not milliseconds. */
export const JOBS_POLL_MS = 15000;

/** Classifies a job-channel frame for the membership machine. */
export function classifyJobFrame(frame: StreamFrame): 'queued' | 'started' | 'terminal' | 'stage' | 'other' {
  if (frame.entity !== 'job') return 'other';
  if (frame.key.startsWith('stage:')) return 'stage';
  if (frame.done) return 'terminal';
  return frame.action === 'add' ? 'queued' : 'started';
}

function asNumber(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function asString(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback;
}

function rowFromFrame(id: number, data: Record<string, unknown>): OpsTaskView {
  return {
    id,
    kind: asString(data.kind, 'unknown'),
    path: asString(data.path, ''),
    folder_path: asString(data.folder_path, ''),
    parent_task_id: typeof data.parent_task_id === 'number' ? data.parent_task_id : null,
    retry_number: asNumber(data.retry_number, 0),
    wait_ms: asNumber(data.wait_ms, 0),
    running_ms: null,
    blocked_on: Array.isArray(data.blocked_on)
      ? data.blocked_on.filter((v): v is number => typeof v === 'number')
      : [],
    budget_secs: typeof data.budget_secs === 'number' ? data.budget_secs : null,
    nearing_budget: data.nearing_budget === true,
  };
}

export class JobsState {
  readonly source: JobsSource;
  /** Transcript/metric wave rows — the scan channel's folder frames never land here. */
  readonly wavesState = new WaveState(['transcript', 'metric']);

  pending = $state<OpsTaskView[]>([]);
  blocked = $state<OpsTaskView[]>([]);
  running = $state<OpsTaskView[]>([]);
  backoff = $state<OpsBackoffView[]>([]);
  failed = $state<OpsFailedTask[]>([]);
  abandoned = $state<OpsAbandonedFeed | null>(null);
  indexErrors = $state<OpsIndexError[]>([]);
  /** Exact queue totals — the lists cap, these never lie. */
  totals = $state<{ pending: number; blocked: number; running: number }>({
    pending: 0,
    blocked: 0,
    running: 0,
  });
  /** Per-kind depth for the queue section. */
  byKind = $state<Record<string, OpsKindDepth>>({});
  /** True when the daemon capped a list the totals still count. */
  truncated = $state(false);
  /** True once the failures feed has loaded — distinguishes "none" from "never read". */
  failuresLoaded = $state(false);
  /** Which failures feed refused, when one did — the feed's fault, never the daemon's. */
  failuresError = $state<string | null>(null);
  daemonReachable = $state(true);
  /** The live stream's own fault, held until the next open re-reads. */
  streamFault = $state<string | null>(null);

  #unsub: (() => void) | null = null;
  #pollTimeout: ReturnType<typeof setTimeout> | null = null;
  #attached = false;
  #openedOnce = false;
  // Joinable in-flight attach: every surface attaches from its mount effect,
  // so concurrent callers join one run. `#gen` retires a run a reset()
  // overtook.
  #attaching: Promise<void> | null = null;
  #gen = 0;

  constructor(source: JobsSource) {
    this.source = source;
  }

  get waves(): WaveRow[] {
    return this.wavesState.items;
  }

  /**
   * Observe the queue. Idempotent: calls while attached are no-ops.
   *
   * Sequence (ordering is load-bearing):
   *   1. Subscribe first so the broadcast channel has a listener before any
   *      work emits. The initial open is NOT a reconnect (attach hydrates
   *      below); later opens clear the stream fault and re-read.
   *   2. Hydrate from the DB so already-queued work is visible immediately.
   *   3. Start the poller — the wait/blocked refresh and the recovery path
   *      when the daemon was down at attach time.
   */
  attach(): Promise<void> {
    if (this.#attached) return Promise.resolve();
    if (this.#attaching) return this.#attaching;
    this.#attaching = this.#doAttach().finally(() => {
      this.#attaching = null;
    });
    return this.#attaching;
  }

  async #doAttach(): Promise<void> {
    const gen = this.#gen;
    this.#teardown();
    this.pending = [];
    this.blocked = [];
    this.running = [];
    this.backoff = [];
    this.failed = [];
    this.abandoned = null;
    this.indexErrors = [];
    this.totals = { pending: 0, blocked: 0, running: 0 };
    this.byKind = {};
    this.truncated = false;
    this.failuresLoaded = false;
    this.failuresError = null;
    this.streamFault = null;
    this.daemonReachable = true;

    this.#unsub = this.source.events(
      (frame) => this.apply(frame),
      () => {
        if (!this.#openedOnce) {
          this.#openedOnce = true;
          return;
        }
        this.streamFault = null;
        void this.#hydrate().catch(() => {
          // A reconnect read that fails must read as a fault, not stale
          // stores: unreachable until the poller says otherwise.
          this.daemonReachable = false;
        });
      },
      (reason) => {
        this.streamFault = reason;
      },
    );
    this.#attached = true;

    try {
      await this.#hydrate();
      if (gen !== this.#gen) return;
      await this.refreshFailures();
      if (gen !== this.#gen) return;
    } catch {
      // A daemon that is down at attach time reads as unreachable, never as
      // failed: the poller below is the recovery path.
      this.daemonReachable = false;
    }
    this.#ensurePoller();
  }

  /**
   * (Re)reads the failures feed: failed tasks, abandonments, index errors.
   * Each feed lands independently — one refusal keeps the other two and
   * faults itself by name, never the daemon.
   */
  async refreshFailures(): Promise<void> {
    const [failed, abandoned, indexErrors] = await Promise.all([
      this.source.readFailed(),
      this.source.readAbandoned(),
      this.source.readIndexErrors(),
    ]);
    const refused: string[] = [];
    if (failed.ok) this.failed = failed.data.tasks;
    else refused.push(`failed tasks (${failureReason(failed.error)})`);
    if (abandoned.ok) this.abandoned = abandoned.data;
    else refused.push(`abandonments (${failureReason(abandoned.error)})`);
    if (indexErrors.ok) this.indexErrors = indexErrors.data;
    else refused.push(`index errors (${failureReason(indexErrors.error)})`);
    this.failuresError = refused.length > 0 ? refused.join('; ') : null;
    this.failuresLoaded = true;
  }

  /** Full teardown — closes the stream, clears the lists. */
  reset(): void {
    this.#gen += 1;
    this.#teardown();
    this.pending = [];
    this.blocked = [];
    this.running = [];
    this.backoff = [];
    this.failed = [];
    this.abandoned = null;
    this.indexErrors = [];
    this.totals = { pending: 0, blocked: 0, running: 0 };
    this.byKind = {};
    this.truncated = false;
    this.failuresLoaded = false;
    this.failuresError = null;
    this.daemonReachable = true;
    this.streamFault = null;
    this.#attached = false;
    this.#openedOnce = false;
  }

  async #hydrate(): Promise<void> {
    const read = await this.source.readQueue();
    this.daemonReachable = read.ok;
    if (read.ok) {
      this.pending = read.data.pending;
      this.blocked = read.data.blocked;
      this.running = read.data.running;
      this.backoff = read.data.backoff;
      this.totals = read.data.totals;
      this.byKind = read.data.by_kind;
      this.truncated = read.data.truncated;
    }
    // A failed read keeps the last good lists — staleness reads as
    // unreachable, never as an empty queue.
  }

  /**
   * Applies one live frame: transcript/metric wave frames open/close wave
   * rows, job frames move queue rows through queued → running → terminal,
   * everything else — folders, stages, foreign entities — belongs to
   * another store.
   */
  apply(frame: StreamFrame): void {
    if (frame.entity === 'transcript' || frame.entity === 'metric') {
      this.wavesState.apply(frame);
      return;
    }
    const kind = classifyJobFrame(frame);
    if (kind === 'other' || kind === 'stage') return;
    const id = Number(frame.key);
    if (!Number.isInteger(id)) return;
    if (kind === 'terminal') {
      this.pending = this.pending.filter((r) => r.id !== id);
      this.blocked = this.blocked.filter((r) => r.id !== id);
      this.running = this.running.filter((r) => r.id !== id);
      this.backoff = this.backoff.filter((r) => r.task_id !== id);
      return;
    }
    if (kind === 'started') {
      const at = this.pending.findIndex((r) => r.id === id);
      const row = at >= 0 ? this.pending[at] : rowFromFrame(id, frame.data);
      this.pending = this.pending.filter((r) => r.id !== id);
      const runAt = this.running.findIndex((r) => r.id === id);
      if (runAt >= 0) this.running[runAt] = row;
      else this.running = [...this.running, row];
      return;
    }
    // Queued: replace on re-add so a duplicate frame never duplicates a row.
    const row = rowFromFrame(id, frame.data);
    const at = this.pending.findIndex((r) => r.id === id);
    if (at >= 0) this.pending[at] = row;
    else this.pending = [...this.pending, row];
  }

  #ensurePoller(): void {
    if (this.#pollTimeout !== null) return;
    const tick = () => {
      this.#pollTimeout = null;
      if (!this.#attached) return;
      void this.#hydrate()
        .catch(() => {
          this.daemonReachable = false;
        })
        .finally(() => {
          this.#ensurePoller();
        });
    };
    this.#pollTimeout = setTimeout(tick, JOBS_POLL_MS);
  }

  #teardown(): void {
    if (this.#unsub) {
      this.#unsub();
      this.#unsub = null;
    }
    if (this.#pollTimeout !== null) {
      clearTimeout(this.#pollTimeout);
      this.#pollTimeout = null;
    }
  }
}
