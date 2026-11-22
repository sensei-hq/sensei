// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { alertsState, jobsState } from '$lib/ops.js';
import OpsAlertsSection from './OpsAlertsSection.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  alertsState.reset();
  jobsState.reset();
  vi.unstubAllGlobals();
});

const ok = (body: unknown) =>
  new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } });

function mount() {
  const m = mountComponent(OpsAlertsSection, {});
  cleanup.push(m.destroy);
  return m;
}

const failedTask = (taskId: number) => ({
  taskId, parentTaskId: null, kind: 'process_file', folderPath: '/r', path: `/r/f${taskId}.ts`,
  status: 'failed', itemsProcessed: 0, durationMs: null, retryNumber: 0, error: 'boom',
  startedAt: '2026-11-21T10:00:00.000Z', completedAt: null,
});

describe('OpsAlertsSection', () => {
  it('says all is clear when no alert stands', () => {
    const m = mount();
    expect(m.container.querySelector('[data-component="ops-alerts"]')).not.toBeNull();
    expect(m.container.textContent).toContain('All clear');
  });

  it('standing alerts render with title, reason, and count', () => {
    jobsState.failed = [failedTask(7)];
    alertsState.refresh();
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-alert-row]')).toHaveLength(1);
    expect(m.container.textContent).toContain('process_file failed');
    expect(m.container.textContent).toContain('boom');
  });

  it('an ack marks the alert without dropping it', async () => {
    jobsState.failed = [failedTask(7)];
    alertsState.refresh();
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ok({ class: 'task', key: '7', acked: true, seen_clears: 0 })),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-alert-ack="task:7"]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('Acked'));
    // Still listed — an ack silences, it never deletes standing work.
    expect(m.container.querySelectorAll('[data-alert-row]')).toHaveLength(1);
  });

  it('a refused ack says why and leaves the alert unacked', async () => {
    jobsState.failed = [failedTask(7)];
    alertsState.refresh();
    flushSync();
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response(JSON.stringify({ message: 'db locked' }), { status: 500 })),
    );
    const m = mount();
    m.container.querySelector<HTMLButtonElement>('[data-alert-ack="task:7"]')!.click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('db locked'));
    expect(m.container.textContent).not.toContain('Acked');
    expect(m.container.querySelector('[data-alert-ack="task:7"]')).not.toBeNull();
  });

  it('an escalated recurrence says so', () => {
    jobsState.failed = [failedTask(7)];
    alertsState.refresh();
    jobsState.failed = [];
    alertsState.refresh();
    jobsState.failed = [failedTask(7)];
    alertsState.refresh();
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('Escalated');
  });
});
