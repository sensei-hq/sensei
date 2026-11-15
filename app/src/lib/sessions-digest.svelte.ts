// Sessions digest — reactive view state.
//
// Owns the chart chip, the mini-cycler mode, the range window, and the loaded
// rows; exposes derived enriched/day-bucket/totals slices so the chart and the
// header always read from ONE filtered source (spec wrong-gate: "chart flips
// and the totals row goes stale"). Range changes refetch through an injected
// `fetcher` so the class is testable without the daemon.
//
// Shared by the Observatory Sessions screen and the workspace-scoped Sessions
// screen — the workspace page injects a workspace-scoped fetcher, the observatory
// page a multi-workspace one. Pure math lives in ./sessions-digest.ts — this
// file only adds runes + the refetch action on top of it.
import type { HourActivity, SessionRow, SessionsDigest } from '$lib/types.js';
import { failureReason, readOutcome, type ApiResult } from '$lib/api.js';
import {
  activityByDay,
  buildDays,
  computeTotals,
  enrich,
  nextMiniMode,
  recencyRange,
  sessionSpans,
  turnsByDay,
  type ActivityDay,
  type ChartVariant,
  type DigestTotals,
  type EnrichedSession,
  type MiniMode,
  type SessionSpan,
  type TurnDay,
  type SessionRange,
} from './sessions-digest.js';

/** One range's read: the session rows, and the work by the hour it happened. */
export interface DigestRead {
  sessions: SessionRow[];
  activity: HourActivity[];
}

/** Fetches a range, or why it could not be read. Injected so the page binds the
 *  real api client and specs bind a fake — keeping this class free of transport
 *  concerns. */
export type SessionsFetcher = (range: SessionRange) => Promise<ApiResult<DigestRead>>;

/** The digest read for a range — scoped to one workspace when given (the
 *  endpoint takes `range` and `workspace` additively) — as its rows. One fetcher
 *  for the Observatory screen and the workspace tab. */
export function sessionsFetcher(
  api: { tryGetSessionsDigest: (range?: string, workspace?: string) => Promise<ApiResult<SessionsDigest>> },
  workspaceId?: string,
): SessionsFetcher {
  return async (range) => {
    const read = await api.tryGetSessionsDigest(range, workspaceId);
    return read.ok ? { ok: true, data: { sessions: read.data.sessions, activity: read.data.activity } } : read;
  };
}

/** A Sessions page's first load: the rows, or why they could not be read (#288) —
 *  never the empty digest that read as a quiet week. Without a range it opens on
 *  the workspace's recency (`recencyRange`): the widest window finds the newest
 *  session, and a narrower range is then read from the server, which owns what a
 *  range holds. */
export async function loadSessionsDigest(fetcher: SessionsFetcher, range?: SessionRange) {
  const widest: SessionRange = '90d';
  const outcome = (r: ApiResult<DigestRead>, at: SessionRange) => {
    const { data, error } = readOutcome(r);
    return { sessions: data?.sessions ?? null, activity: data?.activity ?? null, range: at, error };
  };
  const first = await fetcher(range ?? widest);
  if (range || !first.ok) return outcome(first, range ?? widest);
  const chosen = recencyRange(first.data.sessions.map((s) => s.startedAt));
  if (chosen === widest) return outcome(first, chosen);
  return outcome(await fetcher(chosen), chosen);
}

export class SessionsDigestState {
  /** The big chart selected by the 4-chip group. Default `trend`. */
  chart = $state<ChartVariant>('trend');
  /** The header mini-cycler mode. `pulse` is reachable ONLY here. */
  miniMode = $state<MiniMode>('numbers');
  /** Active time window. Changing it refetches. */
  range = $state<SessionRange>('7d');
  /** Raw rows for the active window (newest-first from the daemon). */
  sessions = $state<SessionRow[]>([]);
  /** The window's work by the hour it happened (turns, corrections, tokens). */
  hours = $state<HourActivity[]>([]);
  /** In-flight refetch flag for the range chips. */
  loading = $state(false);
  /** Why the rows for the active range could not be read, or null (#288). */
  error = $state<string | null>(null);

  #fetcher: SessionsFetcher;

  /** `initialSessions` is null when the first load failed; `initialError` says why. */
  constructor(
    fetcher: SessionsFetcher,
    initialSessions: SessionRow[] | null = [],
    initialRange: SessionRange = '7d',
    initialError: string | null = null,
    initialHours: HourActivity[] | null = [],
  ) {
    this.#fetcher = fetcher;
    this.sessions = initialSessions ?? [];
    this.hours = initialHours ?? [];
    this.range = initialRange;
    this.error = initialError;
  }

  /** Every row with its derived display fields. */
  get enriched(): EnrichedSession[] {
    return enrich(this.sessions);
  }

  /** Ordered day-axis keys for the active range. */
  get days(): string[] {
    return buildDays(this.range);
  }

  /** Agent and your active time per local day (the bands chart), and how many
   *  sessions carry no hours yet. */
  get activity(): { days: ActivityDay[]; unmeasured: number; spans: SessionSpan[] } {
    return { ...activityByDay(this.sessions, this.days), spans: sessionSpans(this.sessions, this.days) };
  }

  /** Turns, corrections, first-try rate and tokens per local day — Trend, Stream and
   *  Tokens, so a session that ran for days counts on each of them. */
  get turnDays(): TurnDay[] {
    return turnsByDay(this.hours, this.days);
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
    await this.retry();
  }

  /** Re-read the active range — every range switch, and the page's Retry. A
   *  failed read drops the previous range's rows so they cannot pass as this
   *  range's. */
  async retry(): Promise<void> {
    this.loading = true;
    const read = await this.#fetcher(this.range);
    this.sessions = read.ok ? read.data.sessions : [];
    this.hours = read.ok ? read.data.activity : [];
    this.error = read.ok ? null : failureReason(read.error);
    this.loading = false;
  }
}
