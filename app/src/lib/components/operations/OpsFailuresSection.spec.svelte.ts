// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { jobsState } from '$lib/ops.js';
import OpsFailuresSection from './OpsFailuresSection.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  jobsState.reset();
  vi.unstubAllGlobals();
});

function mount() {
  const m = mountComponent(OpsFailuresSection, {});
  cleanup.push(m.destroy);
  return m;
}

const failed = (taskId: number) => ({
  taskId, parentTaskId: null, kind: 'process_file', folderPath: '/r', path: `/r/f${taskId}.ts`,
  status: 'failed', itemsProcessed: 0, durationMs: null, retryNumber: 1, error: 'parse blew up',
  startedAt: '2026-11-21T10:00:00.000Z', completedAt: null,
});

const ok = (body: unknown) =>
  new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } });

describe('OpsFailuresSection', () => {
  it('waits for the feed before claiming anything', () => {
    const m = mount();
    expect(m.container.querySelector('[data-component="ops-failures"]')).not.toBeNull();
    expect(m.container.textContent).toContain('Loading failures');
  });

  it('says there are no failures when every feed is empty', () => {
    jobsState.failuresLoaded = true;
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('No failures');
  });

  it('failed rows name kind, path, and reason, with a retry', () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [failed(7)];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-failure-row]')).toHaveLength(1);
    expect(m.container.textContent).toContain('process_file');
    expect(m.container.textContent).toContain('parse blew up');
    expect(m.container.querySelector('[data-failure-retry="7"]')).not.toBeNull();
  });

  it('expanding a row loads attempts and children', async () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [failed(7)];
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string) =>
        ok(
          url.includes('/api/tasks/7')
            ? {
                taskId: 7, current: failed(7), attempts: [failed(7)],
                children: [{ ...failed(8), status: 'completed' }],
                childSummary: { total: 1, completed: 1, failed: 0, running: 0, settled: true },
              }
            : {},
        ),
      ),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-failure-row="7"]')!.click();
    await vi.waitFor(() => expect(m.container.querySelector('[data-failure-detail="7"]')).not.toBeNull());
    flushSync();
    expect(m.container.textContent).toContain('attempts (1)');
    expect(m.container.textContent).toContain('children 1');
  });

  it('retry re-queues and names the new id', async () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [failed(7)];
    flushSync();
    const fetch = vi.fn(async (url: string) =>
      ok(url.includes('/retry') ? { taskId: 9 } : {}),
    );
    vi.stubGlobal('fetch', fetch);
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-failure-retry="7"]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('re-queued as #9'));
    expect(fetch.mock.calls[0][0]).toContain('/api/tasks/7/retry');
  });

  it('a refused retry says why', async () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [failed(7)];
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ message: 'import_lib must not be re-run' }), { status: 409 })),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-failure-retry="7"]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('must not be re-run'));
  });

  it('a feed fault renders without hiding the rows that landed', () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [failed(7)];
    jobsState.failuresError = 'abandonments (feed failed closed)';
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-failure-row]')).toHaveLength(1);
    expect(m.container.textContent).toContain('abandonments');
  });

  it('abandonments and index errors render beside failed tasks', () => {
    jobsState.failuresLoaded = true;
    jobsState.abandoned = {
      days: 7, by_day: [], worst: [{ task_kind: 'process_file', path: '/r/old.ts', count: 3, last_at: 'x' }], tasks: [],
    };
    jobsState.indexErrors = [
      { folder_id: 'f', file_path: '/r/bad.ts', error: 'no adapter', adapter: null, phase: null, created_at: 'x' },
    ];
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('/r/old.ts');
    expect(m.container.textContent).toContain('no adapter');
  });
});
