// Settings · Local models — the on-demand HF-via-embedded-llama provisioning
// surface. Co-locates the presentation logic (phase -> label/tone/progress) and
// the `LocalModels` polling controller so `LocalModelsPanel.svelte` stays a pure
// template: props in, markup out.
//
// The daemon owns the catalog + the pull lifecycle (see
// `crates/senseid/src/api/handlers/model_provisioning.rs`); this module renders
// the phase and drives the pull + status poll. Wire-API-wins: `phase` arrives as
// the `kernel::ProvisionPhase` serde shape, so an unrecognised phase degrades to
// a neutral label rather than crashing.

import type { SenseiApi } from '$lib/api.js';
import type { ProvisionModel, ProvisionPhase } from '$lib/types.js';

/** Semantic tone a phase label renders in. Mirrors the named-token vocabulary
 *  (`text-ink-mute` / `text-ink-soft` / `text-success` / `text-warning` /
 *  `text-accent`). */
export type PhaseTone = 'ink-mute' | 'ink-soft' | 'success' | 'warning' | 'accent';

/** The presentation of one phase: the label to show, its tone, an optional
 *  download percentage (0–100, only mid-download with a known total), whether
 *  the row offers a pull action (Pull / Retry), and whether a pull is in
 *  flight (queued/downloading/verifying/loading — the button is disabled and
 *  the status label shows instead). */
export interface PhaseDisplay {
  label: string;
  tone: PhaseTone;
  percent: number | null;
  actionable: boolean;
  inFlight: boolean;
}

/** Pure: map a `ProvisionPhase` to its presentation. Ready is success-toned;
 *  a pull-in-flight phase disables the action; `absent` / `failed` are the two
 *  actionable states (pull / retry). An unrecognised phase degrades to a
 *  neutral, non-actionable, not-in-flight label so a future kernel variant
 *  never crashes the panel. */
export function phaseDisplay(phase: ProvisionPhase): PhaseDisplay {
  switch (phase.phase) {
    case 'absent':
      return { label: 'not pulled', tone: 'ink-mute', percent: null, actionable: true, inFlight: false };
    case 'queued':
      return { label: 'queued…', tone: 'ink-soft', percent: null, actionable: false, inFlight: true };
    case 'downloading': {
      const percent = phase.total ? Math.round((phase.done / phase.total) * 100) : null;
      const label = percent !== null ? `downloading ${percent}%` : 'downloading…';
      return { label, tone: 'accent', percent, actionable: false, inFlight: true };
    }
    case 'verifying':
      return { label: 'verifying…', tone: 'ink-soft', percent: null, actionable: false, inFlight: true };
    case 'loading':
      return { label: 'loading…', tone: 'ink-soft', percent: null, actionable: false, inFlight: true };
    case 'ready':
      return { label: 'ready', tone: 'success', percent: 100, actionable: false, inFlight: false };
    case 'failed':
      return { label: 'failed', tone: 'warning', percent: null, actionable: true, inFlight: false };
    default:
      // Wire-API-wins: an unknown phase is surfaced neutrally, never dropped.
      return { label: 'unknown', tone: 'ink-mute', percent: null, actionable: false, inFlight: false };
  }
}

/** Pure: the error string a `failed` phase carries, for a title/tooltip. Empty
 *  for any other phase. */
export function phaseError(phase: ProvisionPhase): string {
  return phase.phase === 'failed' ? phase.error : '';
}

/** How often the controller re-fetches status while a pull is in flight. */
export const POLL_INTERVAL_MS = 1500;

/** Injectable timer seam so tests can drive the poll deterministically without
 *  patching globals. Defaults to the real `setTimeout`/`clearTimeout`. */
export interface Timer {
  set: (fn: () => void, ms: number) => ReturnType<typeof setTimeout>;
  clear: (h: ReturnType<typeof setTimeout>) => void;
}

const realTimer: Timer = {
  set: (fn, ms) => setTimeout(fn, ms),
  clear: (h) => clearTimeout(h),
};

/** Controller for the Local-models panel. Owns the catalog + phases, the load /
 *  pull actions, and the status poll that runs (re-scheduling itself) while any
 *  model is in flight and stops when none are. `notice` carries the
 *  "not available in this build" message when the daemon lacks the embedded
 *  engine (a 501 on pull). Inject `api` (+ optional `timer`) so it's unit-tested
 *  without a live daemon. */
export class LocalModels {
  models = $state<ProvisionModel[]>([]);
  loading = $state(true);
  error = $state<string | null>(null);
  /** Set when a pull returns 501 — provisioning isn't available in this build. */
  notice = $state<string | null>(null);

  #api: SenseiApi;
  #timer: Timer;
  #pollHandle: ReturnType<typeof setTimeout> | null = null;
  /** Guard against overlapping polls (a slow fetch outlasting the interval). */
  #polling = false;
  #disposed = false;

  constructor(api: SenseiApi, timer: Timer = realTimer) {
    this.#api = api;
    this.#timer = timer;
  }

  /** True while any model is mid-pull — drives the poll lifecycle. */
  get anyInFlight(): boolean {
    return this.models.some((m) => phaseDisplay(m.phase).inFlight);
  }

  /** Fetch the catalog + current phases. Never throws — the api client folds a
   *  daemon hiccup into the empty-list fallback. */
  async load(): Promise<void> {
    this.loading = true;
    try {
      const { models } = await this.#api.provisionStatus();
      if (this.#disposed) return;
      this.models = models;
    } finally {
      if (!this.#disposed) this.loading = false;
    }
    this.#syncPolling();
  }

  /** Start (or join) a pull of `id`, then begin polling. On the daemon's 501
   *  (no embedded engine) sets `notice` and leaves the row unchanged. On any
   *  other failure sets `error`. On success optimistically flips the row so the
   *  UI shows activity within ~1s (immediate feedback), then the poll takes
   *  over from the daemon's truth. */
  async pull(id: string): Promise<void> {
    this.error = null;
    const res = await this.#api.provisionModel(id);
    if (this.#disposed) return;
    if (!res.ok) {
      if (res.error.status === 501) {
        this.notice = 'On-demand model pulls aren’t available in this build.';
      } else {
        this.error = res.error.message;
      }
      return;
    }
    // Optimistic: adopt the phase the daemon reports the pull started in so the
    // row flips to queued/downloading immediately, before the first poll.
    this.#applyPhase(id, res.data.phase);
    this.#syncPolling();
  }

  /** Overwrite the phase of one tracked model (optimistic + poll updates go
   *  through here so the array identity changes and runes re-render). */
  #applyPhase(id: string, phase: ProvisionPhase): void {
    this.models = this.models.map((m) => (m.id === id ? { ...m, phase } : m));
  }

  /** Start the poll if something is in flight and it isn't already scheduled or
   *  running; stop it if nothing is in flight. Idempotent — a poll already in
   *  flight (`#polling`) re-arms itself, so we must not schedule a second timer
   *  here (that would leak the first handle). */
  #syncPolling(): void {
    if (this.#disposed) return;
    if (this.anyInFlight) {
      if (this.#pollHandle === null && !this.#polling) this.#scheduleNextPoll();
    } else {
      this.#stopPolling();
    }
  }

  #scheduleNextPoll(): void {
    this.#pollHandle = this.#timer.set(() => {
      this.#pollHandle = null;
      void this.#pollOnce();
    }, POLL_INTERVAL_MS);
  }

  /** One poll tick: re-fetch status, adopt it, then re-arm iff still in flight.
   *  Guarded so a slow fetch can't stack overlapping polls. */
  async #pollOnce(): Promise<void> {
    if (this.#disposed || this.#polling) return;
    this.#polling = true;
    try {
      const { models } = await this.#api.provisionStatus();
      if (this.#disposed) return;
      this.models = models;
    } finally {
      this.#polling = false;
    }
    if (!this.#disposed && this.anyInFlight) this.#scheduleNextPoll();
  }

  #stopPolling(): void {
    if (this.#pollHandle !== null) {
      this.#timer.clear(this.#pollHandle);
      this.#pollHandle = null;
    }
  }

  /** Tear down the poll timer — call from the component's `onDestroy`. */
  dispose(): void {
    this.#disposed = true;
    this.#stopPolling();
  }
}
