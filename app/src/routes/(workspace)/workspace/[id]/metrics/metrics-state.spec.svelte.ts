import { describe, it, expect, vi } from 'vitest';
import { flushSync } from 'svelte';
import { WorkspaceMetrics, type MetricsApi } from './metrics-state.svelte.js';

// The user (2026-11-02): load() waited on ~32 reads before the Metrics page drew
// anything, and refetched them on every drill-down and grain change. A page state
// manages the fetch instead: the page draws at once, each section fills as its read
// lands, the sparklines fill one by one, and what was read is reused.

type Deferred<T> = { promise: Promise<T>; resolve: (v: T) => void };
function deferred<T>(): Deferred<T> {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}
const ok = <T>(data: T) => ({ ok: true as const, data });
const fail = (message: string) => ({ ok: false as const, error: { message, status: 500 } });
const tick = async () => {
  for (let i = 0; i < 5; i++) await Promise.resolve();
  flushSync();
};

const row = (metric: string) =>
  ({ metric, metric_type: 'count', value: 1, delta: null, direction: 'higher_better', date: '2026-11-01' }) as never;
const point = (period: string, value: number) => ({ period, value }) as never;

/** A daemon whose every read is held until the test releases it. */
function daemon() {
  const held = {
    values: deferred<unknown>(),
    registry: deferred<unknown>(),
    health: deferred<unknown>(),
    correlations: deferred<unknown>(),
    consent: deferred<unknown>(),
    recommendations: deferred<unknown>(),
    series: new Map<string, Deferred<unknown>>(),
  };
  const seriesOf = (key: string, grain: string) => {
    const k = `${key}|${grain}`;
    if (!held.series.has(k)) held.series.set(k, deferred());
    return held.series.get(k)!;
  };
  const api = {
    getWorkspaceMetrics: vi.fn(() => held.values.promise),
    getMetricsRegistry: vi.fn(() => held.registry.promise),
    getWorkspaceHealth: vi.fn(() => held.health.promise),
    getMetricCorrelations: vi.fn(() => held.correlations.promise),
    tryGetTranscriptConsent: vi.fn(() => held.consent.promise),
    tryGetWorkspaceRecommendations: vi.fn(() => held.recommendations.promise),
    getWorkspaceMetricSeries: vi.fn((_id: string, key: string, grain: string) => seriesOf(key, grain).promise),
  } as unknown as MetricsApi & Record<string, ReturnType<typeof vi.fn>>;
  return { api, held, seriesOf };
}

describe('WorkspaceMetrics', () => {
  it('reads nothing until opened, then every landing read at once', () => {
    const { api } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    expect(m.values.status).toBe('loading');
    expect(api.getWorkspaceMetrics).not.toHaveBeenCalled();
    m.open();
    for (const fn of ['getWorkspaceMetrics', 'getMetricsRegistry', 'getWorkspaceHealth', 'getMetricCorrelations', 'tryGetTranscriptConsent']) {
      expect(api[fn], fn).toHaveBeenCalledOnce();
    }
  });

  it('fills each section as its own read lands — a slow one holds back nothing else', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.health.resolve(ok({ health_score: 72, rated_metrics: 3, components: {} }));
    await tick();
    expect([m.health.status, m.values.status]).toEqual(['ready', 'loading']);
    expect(m.health.data?.health_score).toBe(72);
    held.values.resolve(ok({ metrics: [row('ftr')], count: 1, narrative: null }));
    held.registry.resolve(ok({ metrics: [{ key: 'ftr', family: 'flow' }], count: 1 }));
    await tick();
    expect(m.values.status).toBe('ready');
    expect(m.values.data?.rows.map((r) => r.metric)).toEqual(['ftr']);
  });

  it('the sparklines come with the values: no read per metric', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.values.resolve(ok({
      metrics: [row('a'), row('b')], count: 2, narrative: null,
      series: { a: [point('2026-10-31', 2), point('2026-11-01', 3)], b: [point('2026-11-01', 7)] },
    }));
    await tick();
    expect(m.sparks.a).toEqual([2, 3]);
    expect(m.sparks.b).toEqual([7]);
    expect(api.getWorkspaceMetricSeries).not.toHaveBeenCalled();
  });

  it('a failed read is its own section’s error; the others still fill', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.registry.resolve(fail('registry down'));
    held.health.resolve(ok({ health_score: 50, rated_metrics: 1, components: {} }));
    await tick();
    expect([m.registry.status, m.registry.error]).toEqual(['error', 'registry down']);
    expect(m.registry.data).toBeNull();
    expect(m.health.status).toBe('ready');
  });

  it('the signal grid needs the values and the registry: a registry that could not be read is its error, not an ungrouped grid (#288)', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.values.resolve(ok({ metrics: [row('ftr')], count: 1, narrative: null }));
    await tick();
    expect(m.grid.status).toBe('loading');
    held.registry.resolve(fail('Internal Server Error'));
    await tick();
    expect([m.grid.status, m.grid.error]).toEqual(['error', 'Internal Server Error']);
  });

  it('the grid is ready with the registry’s families and formulas once both land', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.values.resolve(ok({ metrics: [row('ftr')], count: 1, narrative: null }));
    held.registry.resolve(ok({ metrics: [{ key: 'ftr', family: 'flow', formula: 'first-try ÷ measurable' }], count: 1 }));
    await tick();
    expect(m.grid.status).toBe('ready');
    expect(m.grid.data?.registry).toEqual([{ key: 'ftr', family: 'flow', formula: 'first-try ÷ measurable' }]);
  });

  it('keeps only the correlations between metrics this workspace has', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.values.resolve(ok({ metrics: [row('ftr'), row('churn_rate')], count: 2, narrative: null }));
    held.correlations.resolve(ok({ cells: 3, min_pairs: 20, min_rho: 0.5, correlations: [
      { a: 'ftr', b: 'churn_rate', rho: -0.6, n: 30 },
      { a: 'ftr', b: 'elsewhere', rho: 0.7, n: 40 },
    ] }));
    await tick();
    expect(m.correlationsSection.data?.map((c) => c.b)).toEqual(['churn_rate']);
  });

  it('reopening reuses what was read: no read is made twice while it is fresh', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.values.resolve(ok({ metrics: [], count: 0, narrative: null }));
    await tick();
    m.open();
    expect(api.getWorkspaceMetrics).toHaveBeenCalledOnce();
  });

  it('a stale reopen refreshes in the background, keeping what is shown until the new read lands', async () => {
    const { api, held } = daemon();
    let now = 0;
    const m = new WorkspaceMetrics(api, 'w1', () => now);
    m.open();
    held.values.resolve(ok({ metrics: [row('ftr')], count: 1, narrative: null }));
    await tick();
    now = 120_000;
    held.values = deferred();
    m.open();
    expect(api.getWorkspaceMetrics).toHaveBeenCalledTimes(2);
    expect([m.values.status, m.values.data?.rows.length]).toEqual(['ready', 1]);
  });

  it('a stale reopen re-reads a series the detail already read, showing the old one until the new lands', async () => {
    const { api, held, seriesOf } = daemon();
    let now = 0;
    const m = new WorkspaceMetrics(api, 'w1', () => now);
    m.open();
    m.series('ftr', 'weekly');
    seriesOf('ftr', 'weekly').resolve(ok({ metric: 'ftr', grain: 'weekly', formula: null, series: [point('2026-10-26', 1)] }));
    await tick();
    now = 120_000;
    m.open();
    m.series('ftr', 'weekly');
    expect(api.getWorkspaceMetricSeries).toHaveBeenCalledTimes(2);
    expect(m.series('ftr', 'weekly').data?.length).toBe(1);
  });

  it('a metric’s series at a grain is read once and reused; a failure is that series’ error', async () => {
    const { api, seriesOf } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    expect(m.series('ftr', 'weekly').status).toBe('loading');
    m.series('ftr', 'weekly');
    expect(api.getWorkspaceMetricSeries).toHaveBeenCalledOnce();
    seriesOf('ftr', 'weekly').resolve(ok({ metric: 'ftr', grain: 'weekly', formula: 'x', series: [point('2026-10-26', 0.8)] }));
    await tick();
    expect(m.series('ftr', 'weekly').data?.length).toBe(1);
    m.series('churn_rate', 'monthly');
    seriesOf('churn_rate', 'monthly').resolve(fail('boom'));
    await tick();
    expect([m.series('churn_rate', 'monthly').status, m.series('churn_rate', 'monthly').error]).toEqual(['error', 'boom']);
  });

  it('the pending recommendations are read once, when the detail first asks', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    expect(api.tryGetWorkspaceRecommendations).not.toHaveBeenCalled();
    m.recommendations;
    m.recommendations;
    expect(api.tryGetWorkspaceRecommendations).toHaveBeenCalledOnce();
    expect(api.tryGetWorkspaceRecommendations).toHaveBeenCalledWith('w1', 'pending');
    held.recommendations.resolve(ok([]));
    await tick();
    expect(m.recommendations.status).toBe('ready');
  });

  // L3 · ActionItems act: a settled recommendation leaves the list it was read
  // into, so the landing and every detail stop offering it.
  it('settling a recommendation drops it from the read, and only it', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.recommendations;
    held.recommendations.resolve(ok([{ id: 'a', title: 'one', status: 'pending' }, { id: 'b', title: 'two', status: 'pending' }]));
    await tick();
    m.settleRecommendation('a');
    flushSync();
    expect(m.recommendations.data?.map((r) => r.id)).toEqual(['b']);
    expect(api.tryGetWorkspaceRecommendations).toHaveBeenCalledOnce();
  });

  it('settling while the read failed keeps the failure (never an empty list)', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.recommendations;
    held.recommendations.resolve(fail('boom'));
    await tick();
    m.settleRecommendation('a');
    expect([m.recommendations.status, m.recommendations.error]).toEqual(['error', 'boom']);
  });

  // D83 audit row 74: a failed correlations read is the section's error, never
  // an absent section that reads as "nothing moves together".
  it('the correlations section is loading, this workspace’s pairs, or why the read failed', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    expect(m.correlationsSection.status).toBe('loading');
    held.values.resolve(ok({ metrics: [row('ftr'), row('churn_rate')], count: 2, narrative: null }));
    held.correlations.resolve(ok({ cells: 3, min_pairs: 20, min_rho: 0.5, correlations: [
      { a: 'ftr', b: 'churn_rate', rho: -0.7, n: 30 },
      { a: 'ftr', b: 'not_here', rho: 0.6, n: 30 },
    ] }));
    await tick();
    expect(m.correlationsSection.status).toBe('ready');
    expect(m.correlationsSection.data?.map((c) => c.b)).toEqual(['churn_rate']);

    const failing = daemon();
    const f = new WorkspaceMetrics(failing.api, 'w1');
    f.open();
    failing.held.values.resolve(ok({ metrics: [row('ftr')], count: 1, narrative: null }));
    failing.held.correlations.resolve(fail('correlations down'));
    await tick();
    expect([f.correlationsSection.status, f.correlationsSection.data, f.correlationsSection.error])
      .toEqual(['error', null, 'correlations down']);
  });

  it('a correlations read with no pairs is ready and empty, not an error', async () => {
    const { api, held } = daemon();
    const m = new WorkspaceMetrics(api, 'w1');
    m.open();
    held.values.resolve(ok({ metrics: [row('ftr')], count: 1, narrative: null }));
    held.correlations.resolve(ok({ cells: 0, min_pairs: 20, min_rho: 0.5, correlations: [] }));
    await tick();
    expect([m.correlationsSection.status, m.correlationsSection.data, m.correlationsSection.error])
      .toEqual(['ready', [], null]);
  });
});
