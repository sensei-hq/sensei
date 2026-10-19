// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { ScheduledTask } from '$lib/types.js';
import ScheduledTaskRowHarness from './ScheduledTaskRow.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const NOW = Date.parse('2026-07-13T02:19:29.612354+00:00');

function makeTask(over: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    name: over.name ?? 'analyzer',
    description: over.description ?? 'derives signals and insights',
    last_run_at: 'last_run_at' in over ? (over.last_run_at ?? null) : null,
    last_ok: over.last_ok ?? null,
    last_error: over.last_error ?? null,
    next_run_at: over.next_run_at ?? null,
    interval_secs: over.interval_secs ?? null,
    avg_ms: over.avg_ms ?? null,
    enabled: over.enabled ?? true,
    window: 'window' in over ? (over.window ?? null) : null,
    days: over.days ?? null,
  };
}

describe('ScheduledTaskRow', () => {
  it('carries the scheduled-task testid and worker name/description', () => {
    const m = mountComponent(ScheduledTaskRowHarness, { task: makeTask(), now: NOW });
    cleanup.push(m.destroy);
    const row = m.container.querySelector('[data-testid="scheduled-task"]');
    expect(row).not.toBeNull();
    const t = row?.textContent ?? '';
    expect(t).toContain('analyzer');
    expect(t).toContain('derives signals and insights');
  });

  it('renders a relative last-run time when last_run_at is set', () => {
    const task = makeTask({ last_run_at: '2026-07-13T02:19:29.612354+00:00' });
    const m = mountComponent(ScheduledTaskRowHarness, { task, now: NOW + 3 * 3600_000 });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-task-last-run]')?.textContent).toContain('3h ago');
  });

  it('renders "never" when the worker has no last-run watermark', () => {
    const m = mountComponent(ScheduledTaskRowHarness, {
      task: makeTask({ last_run_at: null }),
      now: NOW,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-task-last-run]')?.textContent).toContain('never');
  });

  // #96: the daemon now sends when a worker runs next and how long it takes.
  it('renders the next run and the average duration', () => {
    const m = mountComponent(ScheduledTaskRowHarness, {
      task: makeTask({ next_run_at: '2026-07-13T02:49:30Z', avg_ms: 1400 }),
      now: NOW,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-task-next-run]')?.textContent).toBe('in 30m');
    expect(m.container.querySelector('[data-task-avg]')?.textContent?.trim()).toBe('1.4s');
  });

  // #96: a failing worker says why, where it is listed — `last_error` was sent
  // and never shown.
  it('shows the last error of a failing worker, and nothing for a healthy one', () => {
    const failing = mountComponent(ScheduledTaskRowHarness, {
      task: makeTask({ last_ok: false, last_error: 'dōjō returned 502' }),
      now: NOW,
    });
    cleanup.push(failing.destroy);
    expect(failing.container.querySelector('[data-task-error]')?.textContent).toContain('dōjō returned 502');

    const ok = mountComponent(ScheduledTaskRowHarness, { task: makeTask({ last_ok: true }), now: NOW });
    cleanup.push(ok.destroy);
    expect(ok.container.querySelector('[data-task-error]')).toBeNull();
  });

  it('shows a muted em dash for the null health fields', () => {
    const m = mountComponent(ScheduledTaskRowHarness, { task: makeTask(), now: NOW });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-task-health]')?.textContent).toContain('—');
    expect(m.container.querySelector('[data-task-interval]')?.textContent).toContain('—');
  });

  it('renders the schedule where the dead avg column used to be', () => {
    // `avg_ms` had no source on either handler — the cell was permanently an em
    // dash. WHEN a worker runs is real data the wire now carries.
    const m = mountComponent(ScheduledTaskRowHarness, {
      task: makeTask({ enabled: true, days: [1, 2, 3, 4, 5], window: '09:00-17:00' }),
      now: NOW,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-task-schedule]')?.textContent).toContain(
      'Mon-Fri 09:00-17:00'
    );
  });

  it('a disabled worker reads "off" rather than showing a cadence that will not fire', () => {
    const m = mountComponent(ScheduledTaskRowHarness, {
      task: makeTask({ enabled: false, interval_secs: 3600 }),
      now: NOW,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-task-schedule]')?.textContent).toContain('off');
  });
});
