import type { StreamFrame } from '../index.js';
import type { StateStream } from '../source.js';
import type { ApiResult } from '../result.js';
import type { OpsSyncStateRow } from './types.js';
import { WaveState, type WaveRow } from './wave-state.svelte.js';

// SyncState observes dōjō sync: sync-state rows hydrated from the DB, wave
// rows from sync frames, and transient per-persona plan/push activity from
// the sync stage frames (which ride as job updates — stages are not queue
// tasks, so they carry no id to key on).

/** What SyncState needs of its source: the sync-state read and the stream. */
export interface SyncSource {
  readSyncState: () => Promise<ApiResult<OpsSyncStateRow[]>>;
  events: StateStream<StreamFrame>;
}

/** The poller cadence in ms — passes run on minute cadence. */
export const SYNC_POLL_MS = 60000;

/** Transient per-persona stage activity inside a pass — cleared by the next hydrate, never persisted. */
export interface SyncStageActivity {
  stage: 'plan' | 'push';
  state: 'started' | 'completed' | 'failed';
  items?: number;
  error?: string;
}

/** Parses `stage:sync_plan|<persona>` / `stage:sync_push|<persona>` keys; null for scan stages and ids. */
export function classifySyncStage(key: string): { stage: 'plan' | 'push'; persona: string } | null {
  const match = /^stage:sync_(plan|push):(.+)$/.exec(key);
  if (!match) return null;
  return { stage: match[1] as 'plan' | 'push', persona: match[2] };
}

export class SyncState {
  readonly source: SyncSource;
  /** Sync pass wave rows — one row per pass, opened by totals, closed by the terminal. */
  readonly wavesState = new WaveState(['sync']);

  rows = $state<OpsSyncStateRow[]>([]);
  activity = $state<Record<string, SyncStageActivity>>({});
  daemonReachable = $state(true);
  /** The live stream's own fault, held until the next open re-reads. */
  streamFault = $state<string | null>(null);

  #unsub: (() => void) | null = null;
  #pollTimeout: ReturnType<typeof setTimeout> | null = null;
  #attached = false;
  #openedOnce = false;
  #attaching: Promise<void> | null = null;
  #gen = 0;

  constructor(source: SyncSource) {
    this.source = source;
  }

  get waves(): WaveRow[] {
    return this.wavesState.items;
  }

  /**
   * Observe sync. Idempotent: calls while attached are no-ops. Subscribe
   * first, hydrate from the DB, then start the poller — the recovery path
   * when the daemon was down at attach time.
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
    this.rows = [];
    this.activity = {};
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
    } catch {
      this.daemonReachable = false;
    }
    this.#ensurePoller();
  }

  /** Full teardown — closes the stream, clears the rows. */
  reset(): void {
    this.#gen += 1;
    this.#teardown();
    this.rows = [];
    this.activity = {};
    this.daemonReachable = true;
    this.streamFault = null;
    this.#attached = false;
    this.#openedOnce = false;
  }

  /**
   * Applies one live frame: sync wave frames open/close pass rows, sync
   * stage frames track per-persona plan/push activity, everything else —
   * queue ids, scan stages — belongs to another store.
   */
  apply(frame: StreamFrame): void {
    if (frame.entity === 'sync') {
      this.wavesState.apply(frame);
      return;
    }
    if (frame.entity !== 'job') return;
    const stage = classifySyncStage(frame.key);
    if (!stage) return;
    const items = frame.data.items;
    const entry: SyncStageActivity =
      frame.error != null
        ? { stage: stage.stage, state: 'failed', error: frame.error.message }
        : frame.done
          ? {
              stage: stage.stage,
              state: 'completed',
              ...(typeof items === 'number' ? { items } : {}),
            }
          : { stage: stage.stage, state: 'started' };
    this.activity = { ...this.activity, [stage.persona]: entry };
  }

  async #hydrate(): Promise<void> {
    const read = await this.source.readSyncState();
    this.daemonReachable = read.ok;
    if (read.ok) this.rows = read.data;
    // A failed read keeps the last good rows — staleness reads as
    // unreachable, never as nothing synced.
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
    this.#pollTimeout = setTimeout(tick, SYNC_POLL_MS);
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
