// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { schedulesState } from '$lib/ops.js';
import OpsSchedulesSection from './OpsSchedulesSection.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  schedulesState.reset();
  vi.unstubAllGlobals();
});

function mount() {
  const m = mountComponent(OpsSchedulesSection, {});
  cleanup.push(m.destroy);
  return m;
}

const row = (name: string, over: Record<string, unknown> = {}) => ({
  name, description: `${name} worker`, enabled: true, interval_secs: 300, window: null, days: [],
  last_run_at: null, last_ok: null, last_error: null, avg_ms: null, running_since: null,
  last_skip_reason: null, last_skip_at: null, overrun_limit_secs: 600, next_run_at: null, ...over,
});

const ok = (body: unknown) =>
  new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } });

describe('OpsSchedulesSection', () => {
  it('says there are no schedules when the list is empty', () => {
    const m = mount();
    expect(m.container.querySelector('[data-component="ops-schedules"]')).not.toBeNull();
    expect(m.container.textContent).toContain('No schedules');
  });

  it('rows name the worker, its cadence, and its standing', () => {
    schedulesState.items = [
      row('metrics', { last_ok: true }),
      row('dojo_sync', { last_ok: false, last_error: 'all personas failed' }),
      row('pruner', { enabled: false }),
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-schedule-row]')).toHaveLength(3);
    expect(m.container.textContent).toContain('metrics');
    expect(m.container.textContent).toContain('every 300s');
    expect(m.container.textContent).toContain('failing: all personas failed');
    expect(m.container.textContent).toContain('paused');
  });

  it('a skipped run names its reason', () => {
    schedulesState.items = [row('metrics', { last_skip_reason: 'overrun guard', last_ok: true })];
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('skipped: overrun guard');
  });

  it('expanding a row loads its log tail', async () => {
    schedulesState.items = [row('metrics')];
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ok({ worker: 'metrics', file: 'metrics.log', lines: ['pass ok', 'pass ok'] })),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-schedule-row="metrics"]')!.click();
    await vi.waitFor(() => expect(m.container.querySelector('[data-schedule-log]')).not.toBeNull());
    flushSync();
    expect(m.container.querySelectorAll('[data-schedule-log] li')).toHaveLength(2);
  });

  it('disabling a worker patches and updates the row', async () => {
    schedulesState.items = [row('metrics')];
    flushSync();
    const fetch = vi.fn(async (url: string, _init?: RequestInit) =>
      ok(url.includes('/log') ? { worker: 'metrics', file: 'm.log', lines: [] } : row('metrics', { enabled: false })),
    );
    vi.stubGlobal('fetch', fetch);
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-schedule-row="metrics"]')!.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-schedule-detail="metrics"]')).not.toBeNull(),
    );
    m.container.querySelector<HTMLButtonElement>('[data-schedule-toggle]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('paused'));
    const patch = fetch.mock.calls.find(([, init]) => (init as RequestInit).method === 'PATCH')!;
    expect(patch[0]).toContain('/api/tasks/scheduled/metrics');
    expect(JSON.parse((patch[1] as RequestInit).body as string)).toEqual({ enabled: false });
  });

  it('editing the cadence patches the interval', async () => {
    schedulesState.items = [row('metrics', { interval_secs: 300 })];
    flushSync();
    const fetch = vi.fn(async (url: string, init?: RequestInit) =>
      ok(
        url.includes('/log')
          ? { worker: 'metrics', file: 'm.log', lines: [] }
          : row('metrics', { interval_secs: 60 }),
      ),
    );
    vi.stubGlobal('fetch', fetch);
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-schedule-row="metrics"]')!.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-schedule-interval]')).not.toBeNull(),
    );
    const input = m.container.querySelector<HTMLInputElement>('[data-schedule-interval]')!;
    input.value = '60';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    m.container.querySelector<HTMLButtonElement>('[data-schedule-apply]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('every 60s'));
    const patch = fetch.mock.calls.find(([, i]) => (i as RequestInit).method === 'PATCH')!;
    expect(JSON.parse((patch[1] as RequestInit).body as string)).toEqual({ interval_secs: 60 });
  });

  it('a refused patch says why and keeps the row', async () => {
    schedulesState.items = [row('metrics')];
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ message: 'unknown worker' }), { status: 404 })),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-schedule-row="metrics"]')!.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-schedule-detail="metrics"]')).not.toBeNull(),
    );
    m.container.querySelector<HTMLButtonElement>('[data-schedule-toggle]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('unknown worker'));
    expect(m.container.textContent).not.toContain('paused');
  });
});
