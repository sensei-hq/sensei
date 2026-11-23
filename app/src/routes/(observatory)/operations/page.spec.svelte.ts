// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { alertsState, jobsState, schedulesState, syncState } from '$lib/ops.js';
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
  alertsState.reset();
  vi.restoreAllMocks();
});

describe('Operations page', () => {
  it('attaches every domain, refreshes failures, and renders all seven sections', async () => {
    const jobs = vi.spyOn(jobsState, 'attach').mockResolvedValue(undefined);
    const schedules = vi.spyOn(schedulesState, 'attach').mockResolvedValue(undefined);
    const sync = vi.spyOn(syncState, 'attach').mockResolvedValue(undefined);
    const alerts = vi.spyOn(alertsState, 'refresh');
    const failures = vi.spyOn(jobsState, 'refreshFailures').mockResolvedValue(undefined);
    const dispositions = vi.spyOn(alertsState, 'loadDispositions').mockResolvedValue(undefined);
    const m = mountComponent(OperationsPage, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(jobs).toHaveBeenCalledOnce());
    await vi.waitFor(() => expect(schedules).toHaveBeenCalledOnce());
    await vi.waitFor(() => expect(sync).toHaveBeenCalledOnce());
    await vi.waitFor(() => expect(failures).toHaveBeenCalledOnce());
    await vi.waitFor(() => expect(dispositions).toHaveBeenCalledOnce());
    await vi.waitFor(() => expect(alerts).toHaveBeenCalled());
    for (const section of ['ops-alerts', 'ops-now', 'ops-queue', 'ops-failures', 'ops-schedules', 'ops-pipelines', 'ops-sync']) {
      expect(m.container.querySelector(`[data-component="${section}"]`), section).not.toBeNull();
    }
  });
});
