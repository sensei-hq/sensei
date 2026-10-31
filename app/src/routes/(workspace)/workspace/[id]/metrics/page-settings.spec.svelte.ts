// @vitest-environment jsdom
//
// The Metrics screen carries the SDLC settings panel and reads its values on
// mount (docs/spec/screen/workspace-metrics-settings.md, done gate 1).
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

const reads: string[] = [];
vi.mock('$app/state', () => ({ page: { params: { id: 'w-1' } } }));
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

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

describe('Workspace Metrics /+page — SDLC settings', () => {
  it("mounts the panel and shows this workspace's stored values", async () => {
    const data = {
      workspace: { id: 'w-1', name: 'w' }, rows: [], registry: [], series: {}, narrative: null,
      health: null, correlations: [], history: [], error: null,
    } as unknown as PageData;
    const m = mountComponent(Page, { data });
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector<HTMLInputElement>('[data-setting="hourly_rate"]')?.value).toBe('40'),
    );
    expect(reads).toEqual(['w-1']);
    expect(m.container.querySelector('section[data-section="metric-settings"]')).not.toBe(null);
    expect(m.container.querySelector<HTMLInputElement>('[data-setting="minutes_per_point"]')!.value).toBe('');
  });
});
