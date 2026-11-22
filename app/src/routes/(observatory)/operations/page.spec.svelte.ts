// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { jobsState, schedulesState, syncState } from '$lib/ops.js';
import OperationsPage from './+page.svelte';

// The page only attaches the singletons and stacks the sections — the
// sections' own specs pin their rendering. Attach is spied so no socket or
// timer opens.
let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  jobsState.reset();
  schedulesState.reset();
  syncState.reset();
  vi.restoreAllMocks();
});

describe('Operations page', () => {
  it('attaches every domain once and renders all six sections', () => {
    const jobs = vi.spyOn(jobsState, 'attach').mockResolvedValue(undefined);
    const schedules = vi.spyOn(schedulesState, 'attach').mockResolvedValue(undefined);
    const sync = vi.spyOn(syncState, 'attach').mockResolvedValue(undefined);
    const m = mountComponent(OperationsPage, {});
    cleanup.push(m.destroy);
    expect(jobs).toHaveBeenCalledOnce();
    expect(schedules).toHaveBeenCalledOnce();
    expect(sync).toHaveBeenCalledOnce();
    for (const section of ['ops-now', 'ops-queue', 'ops-failures', 'ops-schedules', 'ops-pipelines', 'ops-sync']) {
      expect(m.container.querySelector(`[data-component="${section}"]`), section).not.toBeNull();
    }
  });
});
