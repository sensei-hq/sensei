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

  it('recorded runs render as history with their outcome', () => {
    jobsState.waveHistory = [
      { id: 'a1a1a1a1-1111-4111-8111-111111111111', wave: 'transcript', key: '9', parent_task_id: 9, total: 4, settled: 4, outcome: 'done', opened_at: '', closed_at: '' },
      { id: 'b2b2b2b2-2222-4222-8222-222222222222', wave: 'metric', key: '3', parent_task_id: 3, total: 2, settled: 0, outcome: 'abandoned', opened_at: '', closed_at: '' },
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-pipeline-history]')).toHaveLength(2);
    expect(m.container.textContent).toContain('4 of 4');
    expect(m.container.textContent).toContain('abandoned');
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

  it('history rows with a repeated wave+key render once each (keys restart per session)', () => {
    jobsState.waveHistory = [
      { id: '11111111-1111-4111-8111-111111111111', wave: 'metric', key: '1', parent_task_id: 1, total: 2, settled: 2, outcome: 'done', opened_at: '2026-11-21T10:00:00.000Z', closed_at: '2026-11-21T10:01:00.000Z' },
      { id: '22222222-2222-4222-8222-222222222222', wave: 'metric', key: '1', parent_task_id: 1, total: 2, settled: 2, outcome: 'done', opened_at: '2026-11-21T11:00:00.000Z', closed_at: '2026-11-21T11:01:00.000Z' },
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-pipeline-history]')).toHaveLength(2);
  });

  it('an already-running backfill says so instead of naming no wave', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ok({ ok: true, queued: false, running: true })));
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-pipeline-backfill]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('already running'));
    expect(m.container.textContent).not.toContain('#undefined');
  });
});
