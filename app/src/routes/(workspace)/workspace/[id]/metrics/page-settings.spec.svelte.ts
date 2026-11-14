// @vitest-environment jsdom
//
// The Metrics screen carries the SDLC settings panel and reads its values on
// mount (docs/spec/screen/workspace-metrics-settings.md, done gate 1).
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import Page from './+page.svelte';
import { METRICS_HOST, MetricsHost } from './metrics-host.svelte.js';
import type { MetricsApi } from './metrics-state.svelte.js';

const reads: string[] = [];
vi.mock('$lib/appstate.svelte.js', () => ({ appState: { port: 7744 } }));
vi.mock('$lib/api.js', () => ({
  senseiApi: () => ({
    tryGetMetricSettings: async (w: string) => {
      reads.push(w);
      return { ok: true, data: { hourly_rate: 40, minutes_per_point: null, idle_threshold_minutes: 20 } };
    },
    tryPutMetricSettings: async () => ({ ok: false, error: { status: 0, message: 'Network error' } }),
  }),
}));

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  target?.remove();
});

// The metric reads never land: this spec is about the settings panel alone.
const pending = () => new Promise(() => {});
const metricsApi = {
  getWorkspaceMetrics: pending,
  getMetricsRegistry: pending,
  getWorkspaceHealth: pending,
  getMetricCorrelations: pending,
  tryGetTranscriptConsent: pending,
  getWorkspaceMetricSeries: pending,
  tryGetWorkspaceRecommendations: pending,
} as unknown as MetricsApi;

describe('Workspace Metrics /+page — SDLC settings', () => {
  it("mounts the panel and shows this workspace's stored values", async () => {
    const host = new MetricsHost(() => 'w-1', () => metricsApi);
    target = document.body.appendChild(document.createElement('div'));
    const data = { workspaceId: 'w-1', workspace: { id: 'w-1', name: 'w', repos_count: 1 } };
    app = mount(Page, { target, props: { data } as never, context: new Map([[METRICS_HOST, host]]) });
    flushSync();
    const m = { container: target };
    await vi.waitFor(() =>
      expect(m.container.querySelector<HTMLInputElement>('[data-setting="hourly_rate"]')?.value).toBe('40'),
    );
    expect(reads).toEqual(['w-1']);
    expect(m.container.querySelector('section[data-section="metric-settings"]')).not.toBe(null);
    expect(m.container.querySelector<HTMLInputElement>('[data-setting="minutes_per_point"]')!.value).toBe('');
  });
});
