// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Page from './+page.svelte';
import { METRICS_HOST, MetricsHost } from '../metrics-host.svelte.js';
import type { MetricsApi } from '../metrics-state.svelte.js';

// The user (2026-11-02): every metric and grain change re-read everything. The
// detail reads from the workspace's metrics state: the values are read once, a
// series once per metric and grain, and the screen draws before they land.

const ok = (data: unknown) => Promise.resolve({ ok: true, data });
const tick = async () => { await new Promise((r) => setTimeout(r, 0)); flushSync(); };
const ROW = { metric: 'ftr', metric_type: 'pct', value: 0.8, delta: null, direction: 'higher_better', date: '2026-11-01' };

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

function setup(slow = false) {
  const api = {
    getWorkspaceMetrics: vi.fn(() => (slow ? new Promise(() => {}) : ok({ metrics: [ROW], count: 1, narrative: null }))),
    getMetricsRegistry: vi.fn(() => ok({ metrics: [{ key: 'ftr', family: 'outcome', name: 'First-try rate', formula: 'first-try ÷ measurable' }], count: 1 })),
    getWorkspaceHealth: vi.fn(() => ok({ health_score: 70, rated_metrics: 1, components: {} })),
    getMetricCorrelations: vi.fn(() => ok({ cells: 0, min_pairs: 20, min_rho: 0.5, correlations: [] })),
    tryGetTranscriptConsent: vi.fn(() => ok([])),
    getWorkspaceMetricSeries: vi.fn((_id: string, key: string, grain: string) =>
      ok({ metric: key, grain, formula: null, series: [{ period: '2026-10-31', value: 0.7 }, { period: '2026-11-01', value: 0.8 }] })),
    tryGetWorkspaceRecommendations: vi.fn(() => ok([])),
  } as unknown as MetricsApi & Record<string, ReturnType<typeof vi.fn>>;
  return { api, host: new MetricsHost(() => 'w1', () => api) };
}

function render(host: MetricsHost, grain = 'daily') {
  const data = $state({ workspaceId: 'w1', selectedKey: 'ftr', grain });
  target = document.body.appendChild(document.createElement('div'));
  app = mount(Page, { target, props: { data } as never, context: new Map([[METRICS_HOST, host]]) });
  flushSync();
  return data;
}

describe('metric detail', () => {
  it('draws before the values land', () => {
    const { host } = setup(true);
    render(host);
    expect(target.querySelector('[data-component="metrics-loading"]')).not.toBeNull();
  });

  it('shows the metric from the shared state; a grain change reads that series only', async () => {
    const { api, host } = setup();
    const data = render(host);
    await tick();
    expect(target.querySelector('[data-component="signal-detail"][data-signal="ftr"]')).not.toBeNull();
    expect(target.textContent).toContain('first-try ÷ measurable');
    data.grain = 'weekly';
    await tick();
    expect(api.getWorkspaceMetrics).toHaveBeenCalledOnce();
    expect(api.getMetricsRegistry).toHaveBeenCalledOnce();
    const reads = (grain: string) =>
      (api.getWorkspaceMetricSeries as unknown as ReturnType<typeof vi.fn>).mock.calls
        .filter((c: unknown[]) => c[1] === 'ftr' && c[2] === grain);
    expect(reads('weekly')).toHaveLength(1);
    data.grain = 'daily';
    await tick();
    expect(reads('daily')).toHaveLength(1);
  });
});
