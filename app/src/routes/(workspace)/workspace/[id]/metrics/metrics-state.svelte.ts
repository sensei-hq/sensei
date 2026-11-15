// A workspace's metrics, read and kept (user, 2026-11-02: load() is the default,
// but where it is not performant a page state manages the fetch). The Metrics
// landing and every metric's detail read from one of these, so:
// - the page draws at once; each section fills as its own read lands, and a slow
//   read holds back nothing else;
// - the sparklines come with the values (the daemon sends each metric's daily
//   series with them), so there is no read per metric;
// - what was read is reused across the landing and the details, and a stale
//   reopen refreshes in the background, keeping what is shown meanwhile;
// - a failed read is that section's error, never an empty or zero stand-in.
import { SvelteMap } from 'svelte/reactivity';
import type { senseiApi, ApiResult } from '$lib/api.js';
import { densifySeries, type MetricSeriesPoint, type MetricsNarrative, type RegistryMetric, type WorkspaceMetricRow } from '$lib/metrics/metric-view.js';
import type { WorkspaceHealth } from '$lib/metrics/health-radar.js';
import type { MetricCorrelation } from '$lib/metrics/correlation-view.js';
import { historyOffNotice, type HistoryOffNotice } from '$lib/metrics/history-notice.js';
import type { Recommendation } from '$lib/types.js';

export type MetricsApi = Pick<
  ReturnType<typeof senseiApi>,
  | 'getWorkspaceMetrics'
  | 'getMetricsRegistry'
  | 'getWorkspaceHealth'
  | 'getMetricCorrelations'
  | 'tryGetTranscriptConsent'
  | 'getWorkspaceMetricSeries'
  | 'tryGetWorkspaceRecommendations'
  | 'tryPreviewRecommendation'
  | 'tryMaterializeRecommendationWithReason'
  | 'trySnoozeWorkspaceRecommendation'
>;

export type Grain = 'daily' | 'weekly' | 'monthly';

/** One read's place on the screen: loading, its data, or why it failed. */
export interface Section<T> {
  status: 'loading' | 'ready' | 'error';
  data: T | null;
  error: string | null;
}

const LOADING: Section<never> = Object.freeze({ status: 'loading', data: null, error: null });
const settled = <T, U = T>(r: ApiResult<T>, pick: (d: T) => U = (d) => d as unknown as U): Section<U> =>
  r.ok ? { status: 'ready', data: pick(r.data), error: null } : { status: 'error', data: null, error: r.error.message };

/** A reopen within this long reuses what was read; past it, a background refresh. */
const FRESH_MS = 60_000;

export class WorkspaceMetrics {
  values = $state<Section<{ rows: WorkspaceMetricRow[]; narrative: MetricsNarrative | null }>>(LOADING);
  registry = $state<Section<RegistryMetric[]>>(LOADING);
  health = $state<Section<WorkspaceHealth>>(LOADING);
  /** Portfolio-wide correlations, unfiltered; `correlations` keeps this workspace's. */
  allCorrelations = $state<Section<MetricCorrelation[]>>(LOADING);
  /** Which assistants' history is not read (#280); null when all are, or unread. */
  history = $state<HistoryOffNotice | null>(null);
  /** Each metric's daily values for its sparkline, as each lands. */
  sparks = $state<Record<string, (number | null)[]>>({});

  #series = new SvelteMap<string, Section<MetricSeriesPoint[]>>();
  #reads = new Map<string, Promise<Section<MetricSeriesPoint[]>>>();
  #recommendations = $state<Section<Recommendation[]>>(LOADING);
  #recommendationsAsked = false;
  #openedAt: number | null = null;

  constructor(
    private readonly api: MetricsApi,
    readonly workspaceId: string,
    private readonly now: () => number = () => Date.now(),
  ) {}

  /** What the signal grid needs: the values and the registry (which alone carries
   *  each metric's family and formula). Either failing is the grid's error — an
   *  unread registry is not an ungrouped grid (#288). */
  get grid(): Section<{ rows: WorkspaceMetricRow[]; narrative: MetricsNarrative | null; registry: RegistryMetric[] }> {
    const [v, r] = [this.values, this.registry];
    if (v.status === 'error' || r.status === 'error') return { status: 'error', data: null, error: v.error ?? r.error };
    if (v.status === 'loading' || r.status === 'loading') return LOADING;
    return { status: 'ready', data: { ...v.data!, registry: r.data! }, error: null };
  }

  /** The correlations read as a section: loading, this workspace's pairs (only
   *  pairs whose both metrics it has), or why it failed — never an absent
   *  section that reads as "nothing moves together" (D83 audit row 74). The
   *  filter needs the values, so it waits on them and carries their failure. */
  get correlationsSection(): Section<MetricCorrelation[]> {
    const [c, v] = [this.allCorrelations, this.values];
    if (c.status === 'error') return { status: 'error', data: null, error: c.error };
    if (v.status === 'error') return { status: 'error', data: null, error: v.error };
    if (c.status === 'loading' || v.status === 'loading' || !c.data || !v.data) return LOADING;
    const present = new Set(v.data.rows.map((r) => r.metric));
    return { status: 'ready', data: c.data.filter((p) => present.has(p.a) && present.has(p.b)), error: null };
  }

  /** Start the landing's reads; a fresh reopen reuses them, a stale one refreshes. */
  open(): void {
    if (this.#openedAt !== null && this.now() - this.#openedAt < FRESH_MS) return;
    // A refresh reads the series again too; what was read stays shown meanwhile.
    if (this.#openedAt !== null) this.#reads.clear();
    this.#openedAt = this.now();
    const id = this.workspaceId;
    void this.api.getWorkspaceMetrics(id).then((r) => {
      this.values = settled(r, (d) => ({ rows: d.metrics ?? [], narrative: d.narrative ?? null }));
      if (r.ok) {
        this.sparks = Object.fromEntries(
          Object.entries(r.data.series ?? {}).map(([key, points]) => [key, densifySeries(points, 'daily').map((p) => p.value)]),
        );
      }
    });
    void this.api.getMetricsRegistry().then((r) => (this.registry = settled(r, (d) => d.metrics ?? [])));
    void this.api.getWorkspaceHealth(id).then((r) => (this.health = settled(r)));
    void this.api.getMetricCorrelations().then((r) => (this.allCorrelations = settled(r, (d) => d.correlations)));
    void this.api.tryGetTranscriptConsent().then((r) => (this.history = historyOffNotice(r.ok ? r.data : null)));
  }

  /** A metric's series at a grain: read the first time it is asked for, then kept.
   *  Safe to call while rendering — state is written only when the read lands. */
  series(key: string, grain: Grain): Section<MetricSeriesPoint[]> {
    void this.#read(key, grain);
    return this.#series.get(`${key}|${grain}`) ?? LOADING;
  }

  /** One read per metric and grain, however many ask. */
  #read(key: string, grain: Grain): Promise<Section<MetricSeriesPoint[]>> {
    const k = `${key}|${grain}`;
    let pending = this.#reads.get(k);
    if (!pending) {
      pending = this.api.getWorkspaceMetricSeries(this.workspaceId, key, grain).then((r) => {
        const section = settled(r, (d) => d.series);
        this.#series.set(k, section);
        return section;
      });
      this.#reads.set(k, pending);
    }
    return pending;
  }

  /** The pending recommendations: read the first time the detail asks, then kept. */
  get recommendations(): Section<Recommendation[]> {
    if (!this.#recommendationsAsked) {
      this.#recommendationsAsked = true;
      void this.api
        .tryGetWorkspaceRecommendations(this.workspaceId, 'pending')
        .then((r) => (this.#recommendations = settled(r)));
    }
    return this.#recommendations;
  }

  /** Drop a decided recommendation from what was read, so neither the landing
   *  nor any detail offers it again. A failed or unfinished read is left as is. */
  settleRecommendation(id: string): void {
    const r = this.#recommendations;
    if (r.status !== 'ready' || !r.data) return;
    this.#recommendations = { ...r, data: r.data.filter((rec) => rec.id !== id) };
  }
}
