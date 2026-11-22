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
  id: 'row-1', retriedFrom: null, retriedFromTaskId: null,
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

  it('an attempt that retries names the task it retries', async () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [failed(7)];
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ok({ taskId: 9, current: null, attempts: [{ ...failed(9), retriedFromTaskId: 7 }], children: [], childSummary: {} })),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-failure-row="7"]')!.click();
    await vi.waitFor(() => expect(m.container.querySelector('[data-failure-detail="7"]')).not.toBeNull());
    flushSync();
    expect(m.container.textContent).toContain('retried from #7');
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

  it('a feed that fails closed never reads as healthy', () => {
    // Every feed refused on first load: the error renders instead of an
    // empty "No failures" — fail-closed per the alerting spec.
    jobsState.failuresLoaded = true;
    jobsState.failuresError = 'failed tasks (feed failed closed)';
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('failed closed');
    expect(m.container.textContent).not.toContain('No failures');
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
      { id: 10, folder_id: 'f', file_path: '/r/bad.ts', error: 'no adapter', adapter: null, phase: null, created_at: 'x' },
    ];
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('/r/old.ts');
    expect(m.container.textContent).toContain('no adapter');
  });

  it('failed rows with a repeated task id render once each (ids restart per session)', () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [
      { ...failed(7), id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', error: 'first session' },
      { ...failed(7), id: 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb', error: 'second session' },
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-failure-row]')).toHaveLength(2);
  });

  it('attempts with a repeated retry number render once each', async () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = [failed(7)];
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        ok({
          taskId: 7, current: failed(7),
          attempts: [
            { ...failed(7), id: 'cccccccc-cccc-4ccc-8ccc-cccccccccccc', retryNumber: 0 },
            { ...failed(7), id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd', retryNumber: 0 },
          ],
          children: [],
          childSummary: { total: 0, completed: 0, failed: 0, running: 0, settled: true },
        }),
      ),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-failure-row="7"]')!.click();
    await vi.waitFor(() => expect(m.container.querySelector('[data-failure-detail="7"]')).not.toBeNull());
    flushSync();
    expect(m.container.textContent).toContain('attempts (2)');
  });

  it('index errors with a repeated folder+file render once each', () => {
    jobsState.failuresLoaded = true;
    jobsState.indexErrors = [
      { id: 11, folder_id: 'f', file_path: '/r/bad.ts', error: 'first scan', adapter: null, phase: null, created_at: 'x' },
      { id: 12, folder_id: 'f', file_path: '/r/bad.ts', error: 'second scan', adapter: null, phase: null, created_at: 'y' },
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-failure-index-error]')).toHaveLength(2);
  });

  it('caps failures and index errors at 200 rows with honest counts', () => {
    jobsState.failuresLoaded = true;
    jobsState.failed = Array.from({ length: 201 }, (_, i) => ({ ...failed(2000 + i), id: `f-${i}` }));
    jobsState.indexErrors = Array.from({ length: 201 }, (_, i) => ({
      id: 3000 + i, folder_id: 'f', file_path: `/r/e${i}.ts`, error: 'x', adapter: null, phase: null, created_at: 'x',
    }));
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-failure-row]')).toHaveLength(200);
    expect(m.container.querySelector('[data-failures-overflow]')?.textContent).toContain('200 of 201');
    expect(m.container.querySelectorAll('[data-failure-index-error]')).toHaveLength(200);
    expect(m.container.querySelector('[data-index-errors-overflow]')?.textContent).toContain('200 of 201');
  });
});
