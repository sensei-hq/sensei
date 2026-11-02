// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Page from './+page.svelte';
import { METRICS_HOST, MetricsHost } from './metrics-host.svelte.js';
import type { MetricsApi } from './metrics-state.svelte.js';

// The user (2026-11-02): the Metrics page drew nothing until ~32 reads had
// returned. It draws its layout at once and fills as the reads land.

type Deferred = { promise: Promise<unknown>; resolve: (v: unknown) => void };
const deferred = (): Deferred => {
  let resolve!: (v: unknown) => void;
  return { promise: new Promise((r) => (resolve = r)), resolve };
};
const ok = (data: unknown) => ({ ok: true, data });
const tick = async () => { await new Promise((r) => setTimeout(r, 0)); flushSync(); };

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

function render() {
  const held = { values: deferred(), registry: deferred(), health: deferred(), series: deferred() };
  const api = {
    getWorkspaceMetrics: vi.fn(() => held.values.promise),
    getMetricsRegistry: vi.fn(() => held.registry.promise),
    getWorkspaceHealth: vi.fn(() => held.health.promise),
    getMetricCorrelations: vi.fn(() => Promise.resolve(ok({ cells: 0, min_pairs: 20, min_rho: 0.5, correlations: [] }))),
    tryGetTranscriptConsent: vi.fn(() => Promise.resolve(ok([]))),
    getWorkspaceMetricSeries: vi.fn(() => held.series.promise),
    tryGetWorkspaceRecommendations: vi.fn(),
  } as unknown as MetricsApi;
  const host = new MetricsHost(() => 'w1', () => api);
  target = document.body.appendChild(document.createElement('div'));
  const data = { workspaceId: 'w1', workspace: { id: 'w1', name: 'sensei', repos_count: 2 } };
  app = mount(Page, { target, props: { data } as never, context: new Map([[METRICS_HOST, host]]) });
  flushSync();
  return { held, api };
}

const q = (sel: string) => target.querySelector(sel);

describe('metrics landing', () => {
  it('draws its layout before any read has returned', () => {
    const { api } = render();
    expect(api.getWorkspaceMetrics).toHaveBeenCalledOnce();
    expect(q('[data-component="metrics-header"]')).not.toBeNull();
    expect(q('[data-component="metrics-loading"]')).not.toBeNull();
    expect(q('[data-component="metrics-error"]')).toBeNull();
  });

  it('fills the grid when the values and registry land, the sparkline after its own read', async () => {
    const { held } = render();
    held.values.resolve(ok({ metrics: [{ metric: 'ftr', metric_type: 'pct', value: 0.8, delta: null, direction: 'higher_better', date: '2026-11-01' }], count: 1, narrative: null }));
    held.registry.resolve(ok({ metrics: [{ key: 'ftr', family: 'outcome', name: 'First-try rate', formula: 'x' }], count: 1 }));
    await tick();
    expect(q('[data-component="metrics-loading"]')).toBeNull();
    expect(q('[data-component="signal-cell"][data-signal="ftr"]')).not.toBeNull();
    expect(q('[data-component="signal-cell"][data-signal="ftr"] svg')).toBeNull();
    held.series.resolve(ok({ metric: 'ftr', grain: 'daily', formula: null, series: [
      { period: '2026-10-30', value: 0.7 }, { period: '2026-10-31', value: 0.75 }, { period: '2026-11-01', value: 0.8 },
    ] }));
    await tick();
    expect(q('[data-component="signal-cell"][data-signal="ftr"] svg')).not.toBeNull();
  });

  it('a registry that cannot be read is the page’s error, not an ungrouped grid (#288)', async () => {
    const { held } = render();
    held.values.resolve(ok({ metrics: [], count: 0, narrative: null }));
    held.registry.resolve({ ok: false, error: { message: 'registry down', status: 500 } });
    await tick();
    expect(q('[data-component="metrics-error"]')?.textContent).toContain('registry down');
  });
});
