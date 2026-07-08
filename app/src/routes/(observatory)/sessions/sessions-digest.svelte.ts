// Observatory · Sessions digest — reactive view state.
//
// Owns the chart chip, the mini-cycler mode, the range window, and the loaded
// rows; exposes derived enriched/day-bucket/totals slices so the chart and the
// header always read from ONE filtered source (spec wrong-gate: "chart flips
// and the totals row goes stale"). Range changes refetch through an injected
// `fetcher` so the class is testable without the daemon.
//
// Pure math lives in ./sessions-digest.ts — this file only adds runes + the
// refetch action on top of it.
import type { SessionRow } from '$lib/types.js';
import {
  aggregateByDay,
  buildDays,
  computeTotals,
  enrich,
  nextMiniMode,
  type ChartVariant,
  type DayBucket,
  type DigestTotals,
  type EnrichedSession,
  type MiniMode,
  type SessionRange,
} from './sessions-digest.js';

/** Fetches rows for a range. Injected so the page binds the real api client
 *  and specs bind a fake — keeping this class free of transport concerns. */
export type SessionsFetcher = (range: SessionRange) => Promise<SessionRow[]>;

export class SessionsDigestState {
  /** The big chart selected by the 4-chip group. Default `trend`. */
  chart = $state<ChartVariant>('trend');
  /** The header mini-cycler mode. `pulse` is reachable ONLY here. */
  miniMode = $state<MiniMode>('numbers');
  /** Active time window. Changing it refetches. */
  range = $state<SessionRange>('7d');
  /** Raw rows for the active window (newest-first from the daemon). */
  sessions = $state<SessionRow[]>([]);
  /** In-flight refetch flag for the range chips. */
  loading = $state(false);

  #fetcher: SessionsFetcher;

  constructor(fetcher: SessionsFetcher, initialSessions: SessionRow[] = [], initialRange: SessionRange = '7d') {
    this.#fetcher = fetcher;
    this.sessions = initialSessions;
    this.range = initialRange;
  }

  /** Every row with its derived display fields. */
  get enriched(): EnrichedSession[] {
    return enrich(this.sessions);
  }

  /** Ordered day-axis keys for the active range. */
  get days(): string[] {
    return buildDays(this.range);
  }

  /** Per-day quality/minute rollup — the charts' single source. */
  get dayBuckets(): DayBucket[] {
    return aggregateByDay(this.enriched, this.days);
  }

  /** Header stat strip + quality tally over the whole visible slice. */
  get totals(): DigestTotals {
    return computeTotals(this.enriched);
  }

  setChart(chart: ChartVariant): void {
    this.chart = chart;
  }

  cycleMiniMode(): void {
    this.miniMode = nextMiniMode(this.miniMode);
  }

  /** Switch the range and refetch. No-op when the range is unchanged so a
   *  double-click doesn't spawn a redundant round-trip. */
  async setRange(range: SessionRange): Promise<void> {
    if (range === this.range) return;
    this.range = range;
    this.loading = true;
    try {
      this.sessions = await this.#fetcher(range);
    } finally {
      this.loading = false;
    }
  }
}
