import { describe, it, expect, vi } from 'vitest';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const api = {
  getProjectMetrics: vi.fn(async () => ok({ metrics: [], narrative: null })),
  getMetricsRegistry: vi.fn(async () => ok({ metrics: [] })),
  tryGetProjectRecommendations: vi.fn(async () => ok([])),
  tryGetProjectTools: vi.fn(async () => ok({ tools: [{ tool: 'Read', count: 3 }], count: 1 })),
  getProjectMetricSeries: vi.fn(async () => ok({ series: [] })),
};
vi.mock('$lib/api.js', async (importOriginal) => ({
  failureReason: ((await importOriginal()) as typeof import('$lib/api.js')).failureReason,
  senseiApi: () => api,
}));
vi.mock('$lib/appstate.svelte.js', () => ({ appState: { port: 7744 } }));

const { load } = await import('./+page.js');
const run = () =>
  load({ params: { id: 'p1', key: 'unused_tools' }, url: new URL('http://x/?grain=daily') } as never) as Promise<Record<string, unknown>>;

// #288: the tool-usage read is the daemon's or an error — a failed read used to
// fold into no tools, which hid the "Tools used" view as if none were used.
describe('metric detail load — tool usage', () => {
  it('carries the tools the daemon reports', async () => {
    const d = await run();
    expect([d.tools, d.toolsError]).toEqual([[{ tool: 'Read', count: 3 }], null]);
  });
  it('a failed tools read is its own error; the rest of the page loads', async () => {
    api.tryGetProjectTools.mockResolvedValueOnce(down as never);
    const d = await run();
    expect([d.tools, d.toolsError, d.error]).toEqual([[], 'Service Unavailable', null]);
  });
});
