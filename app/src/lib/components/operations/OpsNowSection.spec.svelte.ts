// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { jobsState, syncState } from '$lib/ops.js';
import OpsNowSection from './OpsNowSection.svelte';

// The Now section reads the jobs + sync singletons: drive the state, assert
// the DOM. No attach() here — that would open sockets and timers — so the
// state is set directly and flushed.
let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  jobsState.reset();
  syncState.reset();
});

function mount() {
  const m = mountComponent(OpsNowSection, {});
  cleanup.push(m.destroy);
  return m;
}

describe('OpsNowSection', () => {
  it('says nothing is running when the queue is idle', () => {
    const m = mount();
    expect(m.container.querySelector('[data-component="ops-now"]')).not.toBeNull();
    expect(m.container.textContent).toContain('Nothing running');
  });

  it('lists running jobs with kind, path, and elapsed', () => {
    jobsState.running = [
      {
        id: 3, kind: 'process_file', path: '/r/a.ts', folder_path: '/r', parent_task_id: null,
        retry_number: 0, wait_ms: null, running_ms: 12000, blocked_on: [], budget_secs: null,
        nearing_budget: false,
      },
    ];
    flushSync();
    const m = mount();
    const rows = m.container.querySelectorAll('[data-now-running]');
    expect(rows).toHaveLength(1);
    expect(m.container.textContent).toContain('process_file');
    expect(m.container.textContent).toContain('/r/a.ts');
  });

  it('shows active waves with settled/total progress', () => {
    jobsState.wavesState.apply({
      v: 1, id: 1, ts: '2026-11-21T10:00:00.000Z', entity: 'transcript', action: 'add',
      key: '9', data: { total: 4 },
    });
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-now-wave]')).toHaveLength(1);
    expect(m.container.textContent).toContain('0 of 4');
  });

  it('a stream fault renders the interrupted notice, not the lists', () => {
    jobsState.streamFault = 'the live stream dropped; reconnecting';
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('Live updates interrupted');
  });

  it('an unreachable daemon renders the unreachable notice', () => {
    jobsState.daemonReachable = false;
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('Daemon unreachable');
  });
});
