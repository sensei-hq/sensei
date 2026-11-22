// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { syncState } from '$lib/ops.js';
import OpsSyncSection from './OpsSyncSection.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  syncState.reset();
});

function mount() {
  const m = mountComponent(OpsSyncSection, {});
  cleanup.push(m.destroy);
  return m;
}

const syncRow = (entity_key: string, over: Record<string, unknown> = {}) => ({
  entity: 'repository', entity_key, direction: 'push', state: 'synced', last_error: null,
  attempted_at: null, synced_at: null, updated_at: '2026-11-21T10:00:00.000Z', persona: 'alice',
  remedy: null, ...over,
});

describe('OpsSyncSection', () => {
  it('says there is no sync state when the daemon reports none', () => {
    const m = mount();
    expect(m.container.querySelector('[data-component="ops-sync"]')).not.toBeNull();
    expect(m.container.textContent).toContain('No sync state');
  });

  it('rows name the entity, its state, and what went wrong', () => {
    syncState.rows = [
      syncRow('github.com/a/ok'),
      syncRow('github.com/a/denied', {
        state: 'skipped', last_error: 'sharing denied', remedy: 'Ask an owner to approve sharing.',
      }),
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-sync-row]')).toHaveLength(2);
    expect(m.container.textContent).toContain('github.com/a/denied');
    expect(m.container.textContent).toContain('sharing denied');
    expect(m.container.textContent).toContain('Ask an owner to approve sharing.');
  });

  it('an open pass shows its progress', () => {
    syncState.wavesState.apply({
      v: 1, id: 1, ts: '2026-11-21T10:00:00.000Z', entity: 'sync', action: 'add',
      key: 'pass-1', data: { total: 2 },
    });
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-sync-wave]')).toHaveLength(1);
    expect(m.container.textContent).toContain('0 of 2');
  });

  it('per-persona plan/push activity renders while a pass runs', () => {
    syncState.activity = { alice: { stage: 'push', state: 'started' } };
    flushSync();
    const m = mount();
    expect(m.container.querySelector('[data-sync-activity="alice"]')).not.toBeNull();
    expect(m.container.textContent).toContain('pushing');
  });

  it('a failed stage keeps its reason', () => {
    syncState.activity = { alice: { stage: 'push', state: 'failed', error: 'push refused' } };
    flushSync();
    const m = mount();
    expect(m.container.textContent).toContain('push refused');
  });

  it('recorded passes render as history with their outcome', () => {
    syncState.waveHistory = [
      { wave: 'sync', key: 'pass-1', parent_task_id: null, total: 2, settled: 1, outcome: 'done', opened_at: '', closed_at: '' },
    ];
    flushSync();
    const m = mount();
    expect(m.container.querySelectorAll('[data-sync-history]')).toHaveLength(1);
    expect(m.container.textContent).toContain('1 of 2');
  });
});
