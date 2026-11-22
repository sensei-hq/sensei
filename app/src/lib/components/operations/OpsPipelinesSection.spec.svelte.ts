// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { jobsState } from '$lib/ops.js';
import OpsPipelinesSection from './OpsPipelinesSection.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  jobsState.reset();
  vi.unstubAllGlobals();
});

function mount() {
  const m = mountComponent(OpsPipelinesSection, {});
  cleanup.push(m.destroy);
  return m;
}

function openWave(wave: string, key: string, total: number) {
  jobsState.wavesState.apply({
    v: 1, id: 1, ts: '2026-11-21T10:00:00.000Z', entity: wave, action: 'add', key, data: { total },
  });
  flushSync();
}

const ok = (body: unknown) =>
  new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } });

describe('OpsPipelinesSection', () => {
  it('says there are no waves when no pipeline runs', () => {
    const m = mount();
    expect(m.container.querySelector('[data-component="ops-pipelines"]')).not.toBeNull();
    expect(m.container.textContent).toContain('No pipeline waves');
  });

  it('transcript and metric waves render with progress', () => {
    openWave('transcript', '9', 4);
    openWave('metric', '3', 2);
    const m = mount();
    expect(m.container.querySelectorAll('[data-pipeline-wave]')).toHaveLength(2);
    expect(m.container.textContent).toContain('transcript');
    expect(m.container.textContent).toContain('0 of 4');
    expect(m.container.textContent).toContain('metric');
  });

  it('starting a backfill posts and names the new wave', async () => {
    const fetch = vi.fn(async (_url: string, _init?: RequestInit) =>
      ok({ ok: true, queued: true, taskId: 11, from: null }),
    );
    vi.stubGlobal('fetch', fetch);
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-pipeline-backfill]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('Backfill started as #11'));
    expect(fetch.mock.calls[0][0]).toContain('/api/transcripts/backfill');
  });

  it('a refused backfill says why', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ message: 'no consented source' }), { status: 409 })),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-pipeline-backfill]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('no consented source'));
  });

  it('an already-running backfill says so instead of naming no wave', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ok({ ok: true, queued: false, running: true })));
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-pipeline-backfill]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('already running'));
    expect(m.container.textContent).not.toContain('#undefined');
  });
});
