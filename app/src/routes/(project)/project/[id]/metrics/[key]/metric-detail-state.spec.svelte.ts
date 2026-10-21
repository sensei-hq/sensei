import { describe, it, expect } from 'vitest';
import { MetricDetailState, type MetricDetailData } from './metric-detail-state.svelte';
import type { MetricSeriesPoint } from '$lib/metrics/metric-view.js';
import type { Recommendation } from '$lib/types.js';

/** `n` consecutive days from 2026-10-01; `gaps` are days with no value. */
function days(n: number, gaps: number[] = []): MetricSeriesPoint[] {
  return Array.from({ length: n }, (_, i) => ({
    period: `2026-10-${String(i + 1).padStart(2, '0')}`,
    value: gaps.includes(i) ? null : i,
    direction: 'higher_better',
  })) as MetricSeriesPoint[];
}

function data(over: Partial<MetricDetailData> = {}): MetricDetailData {
  return {
    rows: [], registry: [], narrative: null, selectedKey: 'ftr', grain: 'daily',
    series: days(20), formula: null, recommendations: [],
    ...over,
  } as MetricDetailData;
}

describe('MetricDetailState — the chart window', () => {
  it('shows the grain’s default window, the last 14 days', () => {
    const d = $state(data());
    const s = new MetricDetailState(() => d);
    expect(s.fullChartSeries).toHaveLength(20);
    expect(s.effectiveStart).toBe(6);
    expect(s.chartSeries).toHaveLength(14);
    expect(s.maxStart).toBe(18);
  });

  it('moves the window, and a new metric or grain starts from its default again', () => {
    const d = $state(data());
    const s = new MetricDetailState(() => d);
    s.moveWindow(2);
    expect(s.effectiveStart).toBe(2);
    d.selectedKey = 'rework';
    expect(s.effectiveStart).toBe(6);
    s.moveWindow(3);
    d.grain = 'weekly';
    expect(s.effectiveStart).not.toBe(3);
  });
});

describe('MetricDetailState — the selected point', () => {
  it('defaults to the latest day with a value, and a click moves it', () => {
    const d = $state(data({ series: days(20, [19]) }));
    const s = new MetricDetailState(() => d);
    // Window is days 6..19; day 19 has no value, so the latest is index 12 (day 18).
    expect(s.selectedIndex).toBe(12);
    expect(s.selectedDay).toBe('2026-10-19');
    s.pick(0);
    expect(s.selectedDay).toBe('2026-10-07');
  });

  it('re-defaults when the window moves', () => {
    const d = $state(data());
    const s = new MetricDetailState(() => d);
    s.pick(0);
    s.moveWindow(0);
    expect(s.selectedIndex).toBe(19);
  });

  it('names no day at a coarser grain', () => {
    const d = $state(data({ grain: 'weekly' }));
    const s = new MetricDetailState(() => d);
    expect(s.selectedDay).toBeNull();
  });
});

describe('MetricDetailState — action items', () => {
  it('keeps only the recommendations about this metric', () => {
    const recs = [
      { id: 'a', title: 'Raise FTR by splitting tasks', why: '', impact: '' },
      { id: 'b', title: 'Prune unused deps', why: '', impact: '' },
    ] as unknown as Recommendation[];
    const d = $state(data({ recommendations: recs }));
    const s = new MetricDetailState(() => d);
    expect(s.relevantRecs.map((r) => r.id)).toEqual(['a']);
  });
});
