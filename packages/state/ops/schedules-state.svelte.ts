import type { StreamFrame } from '../index.js';
import type { StateStream } from '../source.js';
import type { ApiResult } from '../result.js';
import type { OpsScheduleLog, OpsSchedulePatch, OpsScheduledTask } from './types.js';

// SchedulesState observes the daemon's scheduled workers: one hydrate at
// attach, then a 60s poll — runs change on minute cadence, never
// sub-second. A schedule frame re-reads the list (targeted re-read per D4);
// no daemon emitter sends one yet — the tick only writes rows — so today the
// poller carries every delta, and the stream carries the fault signal.

/** What SchedulesState needs of its source: the schedules read, the controls, and the stream. */
export interface SchedulesSource {
  readSchedules: () => Promise<ApiResult<OpsScheduledTask[]>>;
  patchSchedule: (name: string, patch: OpsSchedulePatch) => Promise<ApiResult<OpsScheduledTask>>;
  readScheduleLog: (name: string) => Promise<ApiResult<OpsScheduleLog>>;
  events: StateStream<StreamFrame>;
}

/** The poller cadence in ms — runs change on minute cadence. */
export const SCHEDULES_POLL_MS = 60000;

/** Rows disabled by the user — silenced, never failing. */
export function pausedSchedules(items: OpsScheduledTask[]): OpsScheduledTask[] {
  return items.filter((s) => !s.enabled);
}

/** Rows whose last run failed — the schedules the center flags. */
export function failingSchedules(items: OpsScheduledTask[]): OpsScheduledTask[] {
  return items.filter((s) => s.last_ok === false);
}

export class SchedulesState {
  readonly source: SchedulesSource;

  items = $state<OpsScheduledTask[]>([]);
  /** Log tails by worker name — loaded on expand, never polled. */
  logs = $state<Record<string, OpsScheduleLog>>({});
  daemonReachable = $state(true);
  /** The live stream's own fault, held until the next open re-reads. */
  streamFault = $state<string | null>(null);

  #unsub: (() => void) | null = null;
  #pollTimeout: ReturnType<typeof setTimeout> | null = null;
  #attached = false;
  #openedOnce = false;
  #attaching: Promise<void> | null = null;
  #gen = 0;

  constructor(source: SchedulesSource) {
    this.source = source;
  }

  /**
   * Observe the schedules. Idempotent: calls while attached are no-ops.
   * Subscribe first, hydrate from the DB, then start the poller — the
   * recovery path when the daemon was down at attach time.
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
    this.items = [];
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
    this.items = [];
    this.logs = {};
    this.daemonReachable = true;
    this.streamFault = null;
    this.#attached = false;
    this.#openedOnce = false;
  }

  /**
   * Edits one worker's cadence. The daemon's returned row replaces the
   * local one in place; a refusal leaves the row and returns the reason
   * for the control to say.
   */
  async patch(name: string, patch: OpsSchedulePatch): Promise<ApiResult<OpsScheduledTask>> {
    const res = await this.source.patchSchedule(name, patch);
    if (res.ok) {
      const at = this.items.findIndex((s) => s.name === name);
      if (at >= 0) this.items[at] = res.data;
      else this.items = [...this.items, res.data];
    }
    return res;
  }

  /**
   * Loads one worker's log tail into the cache. Every expand re-reads —
   * the tail is history the stream never carries, and the read is the
   * update. A refusal caches nothing and returns the reason.
   */
  async loadLog(name: string): Promise<ApiResult<OpsScheduleLog>> {
    const res = await this.source.readScheduleLog(name);
    if (res.ok) this.logs = { ...this.logs, [name]: res.data };
    return res;
  }

  /**
   * Applies one live frame. A schedule frame re-reads the list — the tick
   * writes rows the frames never carry, so the read is the update.
   */
  apply(frame: StreamFrame): void {
    if (frame.entity !== 'schedule') return;
    void this.#hydrate().catch(() => {
      this.daemonReachable = false;
    });
  }

  async #hydrate(): Promise<void> {
    const read = await this.source.readSchedules();
    this.daemonReachable = read.ok;
    if (read.ok) this.items = read.data;
    // A failed read keeps the last good rows — staleness reads as
    // unreachable, never as no schedules.
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
    this.#pollTimeout = setTimeout(tick, SCHEDULES_POLL_MS);
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
