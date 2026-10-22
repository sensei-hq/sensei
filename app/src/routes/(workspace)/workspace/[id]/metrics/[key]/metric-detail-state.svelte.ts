// Workspace · Metric detail — the screen's state over what the loader read: the
// rail's signals, the selected metric's about-block, its chart (a movable
// window over the densified series), the selected datapoint, and the
// recommendations about this metric. The chart window and the selected point
// are the user's; each belongs to the series it was chosen on, so a different
// metric, grain or window starts from its default again.
import {
  buildSignals,
  familyLookup,
  groupSignals,
  seriesValues,
  seriesDistribution,
  densifySeries,
  metricYDomain,
  defaultWindowForGrain,
  historyNote,
  metricAbout,
  linkifyMetrics,
  metricTickFormatter,
} from '$lib/metrics/metric-view.js';
import { buildActionItems } from '../action-items.js';
import type { PageData } from './$types.js';

export type MetricDetailData = Pick<
  PageData,
  'rows' | 'registry' | 'narrative' | 'selectedKey' | 'grain' | 'series' | 'formula' | 'recommendations'
>;

export class MetricDetailState {
  /** A window start the user chose, and the series (key + grain) it was chosen on. */
  #window = $state<{ on: string; start: number } | null>(null);
  /** A point the user picked, and the window it was picked in. */
  #pick = $state<{ on: string; index: number } | null>(null);

  constructor(private readonly read: () => MetricDetailData) {}

  readonly signals = $derived.by(() => {
    const d = this.read();
    return buildSignals(d.rows, familyLookup(d.registry), d.narrative);
  });
  readonly groups = $derived(groupSignals(this.signals));
  readonly selected = $derived(this.signals.find((s) => s.key === this.read().selectedKey) ?? null);

  /** The static about-block (purpose / how to read / formula), with companion
   *  metrics named in "how to read" linked to their detail. */
  readonly about = $derived.by(() => {
    const d = this.read();
    return metricAbout(d.rows.find((r) => r.metric === d.selectedKey), d.formula);
  });
  readonly howToReadSegments = $derived.by(() => {
    const d = this.read();
    return this.about ? linkifyMetrics(this.about.howToRead, d.registry, d.selectedKey) : [];
  });

  readonly values = $derived.by(() => seriesValues(this.read().series));
  readonly distribution = $derived(seriesDistribution(this.values));
  /** Key-aware: maintainability / duplication are ratios inside 0–0.005 and read
   *  per 1,000 lines, so a type-only formatter would print every tick "0.00". */
  readonly format = $derived(metricTickFormatter(this.selected?.key ?? '', this.selected?.type ?? 'count'));
  readonly yDomain = $derived(metricYDomain(this.selected?.type ?? 'count', this.values));

  // ── The chart window ────────────────────────────────────────────────────

  /** Every period, absent ones as gaps (null). */
  readonly fullChartSeries = $derived.by(() => {
    const d = this.read();
    return densifySeries(d.series, d.grain);
  });
  readonly #seriesId = $derived.by(() => `${this.read().grain}|${this.read().selectedKey}`);
  /** The grain's default window: the last N periods. */
  readonly defaultStart = $derived.by(() => {
    const w = defaultWindowForGrain(this.read().grain);
    return w == null ? 0 : Math.max(0, this.fullChartSeries.length - w);
  });
  readonly effectiveStart = $derived(
    this.#window?.on === this.#seriesId ? this.#window.start : this.defaultStart,
  );
  readonly maxStart = $derived(Math.max(0, this.fullChartSeries.length - 2));
  readonly chartSeries = $derived(this.fullChartSeries.slice(this.effectiveStart));
  readonly caption = $derived.by(() => {
    const note = historyNote(this.fullChartSeries);
    return note ? `${note} · ${this.read().grain}` : '';
  });

  moveWindow(start: number): void {
    this.#window = { on: this.#seriesId, start };
  }

  // ── The selected point ──────────────────────────────────────────────────

  readonly #windowId = $derived(`${this.#seriesId}|${this.effectiveStart}`);
  /** The latest point with a value, unless the user picked one in this window. */
  readonly selectedIndex = $derived.by((): number | null => {
    if (this.#pick?.on === this.#windowId) return this.#pick.index;
    for (let i = this.chartSeries.length - 1; i >= 0; i--) {
      if (this.chartSeries[i].value != null) return i;
    }
    return null;
  });
  /** The selected point's day — at daily grain only, where a chart slot is a
   *  day the drill-down can load; coarser grains keep its own day selector. */
  readonly selectedDay = $derived.by(() =>
    this.read().grain === 'daily' && this.selectedIndex != null
      ? (this.chartSeries[this.selectedIndex]?.date ?? null)
      : null,
  );

  pick(index: number): void {
    this.#pick = { on: this.#windowId, index };
  }

  // ── Action items ────────────────────────────────────────────────────────

  /** The pending recommendations about this metric (matched on its key, name
   *  or family) — none when nothing is specific to it. */
  readonly relevantRecs = $derived.by(() => {
    const terms = [this.read().selectedKey, this.selected?.name, this.selected?.familyLabel]
      .filter((t): t is string => !!t)
      .map((t) => t.toLowerCase());
    return this.read().recommendations.filter((r) => {
      const hay = `${r.title ?? ''} ${r.why ?? ''} ${r.impact ?? ''}`.toLowerCase();
      return terms.some((t) => hay.includes(t));
    });
  });
  readonly actionItems = $derived(buildActionItems(this.relevantRecs));
}
