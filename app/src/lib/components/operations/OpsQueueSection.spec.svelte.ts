// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { jobsState } from '$lib/ops.js';
import OpsQueueSection from './OpsQueueSection.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  jobsState.reset();
});

function mount() {
  const m = mountComponent(OpsQueueSection, {});
  cleanup.push(m.destroy);
  return m;
}

const view = (id: number) => ({
  id, kind: 'process_file', path: `/r/f${id}.ts`, folder_path: '/r', parent_task_id: null,
  retry_number: 0, wait_ms: 4000, running_ms: null, blocked_on: [], budget_secs: null,
  nearing_budget: false,
});

describe('OpsQueueSection', () => {
  it('says the queue is empty when every list is', () => {
    const m = mount();
    expect(m.container.querySelector('[data-component="ops-queue"]')).not.toBeNull();
    expect(m.container.textContent).toContain('Queue empty');
  });

  it('lists pending rows with kind, path, and wait', () => {
    jobsState.pending = [view(1)];
    jobsState.totals = { pending: 1, blocked: 0, running: 0 };
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-queue-pending]')).toHaveLength(1);
    expect(m.container.textContent).toContain('process_file');
    expect(m.container.textContent).toContain('/r/f1.ts');
    expect(m.container.textContent).toContain('pending 1');
  });

  it('names what a blocked row waits on', () => {
    jobsState.blocked = [{ ...view(2), blocked_on: [1] }];
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('waiting on #1');
  });

  it('shows backoff rows with their retry delay', () => {
    jobsState.backoff = [
      { task_id: 5, kind: 'process_file', path: '/r/f5.ts', folder_path: '/r', attempt: 2, retry_in_ms: 4000 },
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-queue-backoff]')).toHaveLength(1);
    expect(m.container.textContent).toContain('attempt 2');
  });

  it('says when the daemon capped the lists', () => {
    jobsState.pending = [view(1)];
    jobsState.totals = { pending: 250, blocked: 0, running: 0 };
    jobsState.truncated = true;
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('showing 1 of 250');
  });

  it('shows per-kind depth', () => {
    jobsState.pending = [view(1)];
    jobsState.byKind = { process_file: { pending: 1, blocked: 0, running: 0 } };
    flushSync();
    const m = mount();
    expect(m.container.querySelector('[data-queue-depth="process_file"]')).not.toBeNull();
  });
});
