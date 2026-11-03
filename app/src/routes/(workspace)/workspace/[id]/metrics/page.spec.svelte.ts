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
  const held = { values: deferred(), registry: deferred(), health: deferred(), series: deferred(), correlations: deferred() };
  const api = {
    getWorkspaceMetrics: vi.fn(() => held.values.promise),
    getMetricsRegistry: vi.fn(() => held.registry.promise),
    getWorkspaceHealth: vi.fn(() => held.health.promise),
    getMetricCorrelations: vi.fn(() => held.correlations.promise),
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

  it('fills the grid, sparklines included, when the values and registry land', async () => {
    const { held, api } = render();
    held.values.resolve(ok({
      metrics: [{ metric: 'ftr', metric_type: 'pct', value: 0.8, delta: null, direction: 'higher_better', date: '2026-11-01' }],
      count: 1, narrative: null,
      series: { ftr: [{ period: '2026-10-30', value: 0.7 }, { period: '2026-10-31', value: 0.75 }, { period: '2026-11-01', value: 0.8 }] },
    }));
    held.registry.resolve(ok({ metrics: [{ key: 'ftr', family: 'outcome', name: 'First-try rate', formula: 'x' }], count: 1 }));
    await tick();
    expect(q('[data-component="metrics-loading"]')).toBeNull();
    expect(q('[data-component="signal-cell"][data-signal="ftr"] svg')).not.toBeNull();
    expect(api.getWorkspaceMetricSeries).not.toHaveBeenCalled();
  });

  it('a registry that cannot be read is the page’s error, not an ungrouped grid (#288)', async () => {
    const { held } = render();
    held.values.resolve(ok({ metrics: [], count: 0, narrative: null }));
    held.registry.resolve({ ok: false, error: { message: 'registry down', status: 500 } });
    await tick();
    expect(q('[data-component="metrics-error"]')?.textContent).toContain('registry down');
  });

  // D83 audit rows 5–7 (one root): a failed health read drew the radar as
  // "No rated metrics yet" with a null score and 0 rated metrics. A failed read
  // is said as such; a read still in flight claims nothing.
  /** The grid has landed with one signal (values + registry), so the radar
   *  section is drawn (with none, the page shows its empty state instead). */
  const gridLanded = (held: ReturnType<typeof render>['held']) => {
    held.values.resolve(ok({
      metrics: [
        { metric: 'ftr', metric_type: 'pct', value: 0.8, delta: null, direction: 'higher_better', date: '2026-11-01' },
        { metric: 'churn_rate', metric_type: 'pct', value: 0.2, delta: null, direction: 'lower_better', date: '2026-11-01' },
      ],
      count: 2, narrative: null,
    }));
    held.registry.resolve(ok({ metrics: [
      { key: 'ftr', family: 'outcome', name: 'First-try rate', formula: 'x' },
      { key: 'churn_rate', family: 'quality', name: 'Churn rate', formula: 'y' },
    ], count: 2 }));
  };
  const noPairs = ok({ cells: 0, min_pairs: 20, min_rho: 0.5, correlations: [] });

  it.skip('pending: d83-metrics — a health read in flight claims no "no rated metrics"', async () => {
    const { held } = render();
    gridLanded(held);
    await tick();
    expect(q('[data-component="metrics-radar"]')).not.toBeNull();
    // What the person sees while it reads: the radar's own loading state.
    expect(q('[data-component="metrics-radar"] [aria-busy="true"]')).not.toBeNull();
    expect(q('[data-component="metrics-radar"]')?.textContent).not.toContain('No rated metrics yet');
    expect(q('[data-component="metrics-radar-error"]')).toBeNull();
  });

  it.skip('pending: d83-metrics — a failed health read is the radar’s error, never "no rated metrics"', async () => {
    const { held } = render();
    gridLanded(held);
    held.health.resolve({ ok: false, error: { message: 'health down', status: 500 } });
    await tick();
    expect(q('[data-component="metrics-radar-error"]')?.textContent).toContain('health down');
    expect(q('[data-component="metrics-radar"]')?.textContent).not.toContain('No rated metrics yet');
  });

  // Guards, green now and kept green through the fix: a read that succeeded is
  // drawn as read — honestly empty, or with its score and rated count.
  it('a read health score with nothing rated draws the honest empty radar', async () => {
    const { held } = render();
    gridLanded(held);
    held.health.resolve(ok({ health_score: null, rated_metrics: 0, components: {} }));
    await tick();
    // Honest-empty: the daemon read fine and has rated nothing yet.
    expect(q('[data-component="metrics-radar"]')?.textContent).toContain('No rated metrics yet');
    expect(q('[data-component="metrics-radar-error"]')).toBeNull();
  });

  it('a read health score draws its score and rated count', async () => {
    const { held } = render();
    gridLanded(held);
    held.health.resolve(ok({
      health_score: 72, rated_metrics: 1,
      components: { ftr: { name: 'First-try rate', rating: 3.4, weight: 1 } },
    }));
    await tick();
    expect(q('[data-testid="health-score"]')?.textContent?.trim()).toBe('72');
    expect(q('[data-component="metrics-radar"]')?.textContent).toContain('1 rated');
    expect(q('[data-testid="health-radar-empty"]')).toBeNull();
    expect(q('[data-component="metrics-radar-error"]')).toBeNull();
  });

  it('a correlations read with no pairs shows no error', async () => {
    const { held } = render();
    gridLanded(held);
    held.correlations.resolve(noPairs);
    await tick();
    expect(q('[data-component="metrics-grid"]')).not.toBeNull();
    expect(q('[data-component="metrics-correlations-error"]')).toBeNull();
  });

  it('a correlations read with a pair draws the section and no error', async () => {
    const { held } = render();
    gridLanded(held);
    held.correlations.resolve(ok({ cells: 1, min_pairs: 20, min_rho: 0.5, correlations: [
      { a: 'ftr', b: 'churn_rate', rho: -0.7, n: 30 },
    ] }));
    await tick();
    expect(q('[data-component="metrics-correlations"]')).not.toBeNull();
    expect(q('[data-component="metrics-correlations-error"]')).toBeNull();
  });

  // D83 audit row 74: a failed correlations read hid the section.
  it.skip('pending: d83-metrics — a failed correlations read says so instead of hiding the section', async () => {
    const { held } = render();
    gridLanded(held);
    held.correlations.resolve({ ok: false, error: { message: 'correlations down', status: 500 } });
    await tick();
    expect(q('[data-component="metrics-correlations-error"]')?.textContent).toContain('correlations down');
  });
});
