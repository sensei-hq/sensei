// Settings · Dōjō — the sync-state derivations.
//
// The credential half reuses `PersonaList.describe`/`tone`/`actionLabel`
// wholesale (tested in personas.spec) — this file covers only what is new: how a
// sync row reads, and what a screen says when nothing has ever synced.
//
// The property under all of it: `skipped` is deliberate and `error` is a fault,
// and the screen must never merge them. That distinction is the reason
// `sensei.sync_state` has four states instead of a boolean.
import { describe, it, expect } from 'vitest';
import {
  syncTone,
  entityLabel,
  agreementLine,
  worstSyncState,
  summarise,
} from './dojo-status-state.svelte.js';
import type { SyncStateRow } from '$lib/types.js';

function row(over: Partial<SyncStateRow> = {}): SyncStateRow {
  return {
    entity: 'repository_metric',
    entity_key: 'github.com/sensei-hq/dbd',
    direction: 'push',
    state: 'synced',
    last_error: null,
    attempted_at: '2026-08-31T19:57:55Z',
    synced_at: '2026-08-31T19:57:55Z',
    updated_at: '2026-08-31T19:57:55Z',
    ...over,
  };
}

describe('syncTone', () => {
  it('separates a deliberate skip from a fault', () => {
    // The whole reason sync_state has four states. A screen that paints both red
    // cries wolf about every private repository; one that paints both grey stays
    // silent about real failures.
    expect(syncTone('error')).toBe('danger');
    expect(syncTone('skipped')).toBe('muted');
  });

  it('reads pending as in-flight, not as broken', () => {
    expect(syncTone('pending')).toBe('info');
    expect(syncTone('synced')).toBe('success');
  });

  it('surfaces an unrecognised state rather than dropping the row', () => {
    // Wire-API-wins: `state` arrives as a string. An unknown value still renders
    // — silently hiding a row is how a broken sync becomes invisible.
    expect(syncTone('something_new')).toBe('ink');
  });
});

describe('entityLabel', () => {
  it('reads the enum as English', () => {
    expect(entityLabel('repository_metric')).toBe('repository metric');
    expect(entityLabel('dojo_sync_plan')).toBe('dōjō sync plan');
  });

  it('passes an unknown entity through instead of blanking it', () => {
    expect(entityLabel('brand_new_thing')).toBe('brand new thing');
  });
});

describe('agreementLine', () => {
  it('says when the two sides last agreed', () => {
    expect(agreementLine(row())).toBe('agreed 2026-08-31');
  });

  it('keeps the last agreement on a FAILED row, and says both things', () => {
    // `mark_sync_error` deliberately preserves `synced_at`. If the line dropped
    // it, "broken since Tuesday" would read the same as "never synced" — which is
    // the difference between a regression and a thing that never worked.
    const line = agreementLine(
      row({ state: 'error', last_error: 'the dojo refused: 402', synced_at: '2026-08-20T10:00:00Z' }),
    );
    expect(line).toContain('2026-08-20');
    expect(line).toContain('failing');
  });

  it('says never rather than showing a blank for an entity that has not synced', () => {
    expect(agreementLine(row({ state: 'pending', synced_at: null }))).toBe('never agreed');
  });

  it('does not call a skipped entity a failure', () => {
    const line = agreementLine(row({ state: 'skipped', synced_at: null, last_error: 'private' }));
    expect(line).not.toContain('failing');
    expect(line).toContain('skipped');
  });
});

describe('worstSyncState', () => {
  it('ranks a fault above everything', () => {
    expect(worstSyncState({ synced: 40, skipped: 5, error: 1 })).toBe('error');
  });

  it('ranks pending above skipped, since one resolves and one is a decision', () => {
    expect(worstSyncState({ synced: 3, skipped: 2, pending: 1 })).toBe('pending');
  });

  it('is null when there is nothing at all, not a fabricated all-clear', () => {
    expect(worstSyncState({})).toBeNull();
  });

  it('ignores an unknown state rather than ranking it worst', () => {
    expect(worstSyncState({ mystery: 9, synced: 1 })).toBe('synced');
  });
});

describe('summarise', () => {
  it('counts what it names', () => {
    expect(summarise({ synced: 40, error: 2 })).toBe('42 tracked · 2 failing');
  });

  it('says everything agrees when nothing is wrong', () => {
    expect(summarise({ synced: 3 })).toBe('3 tracked · all agreed');
  });

  it('counts a skip without calling it a failure', () => {
    expect(summarise({ synced: 3, skipped: 2 })).toBe('5 tracked · 2 skipped');
  });

  it('reports nothing tracked rather than claiming everything agrees', () => {
    // The honest-empty case. "all agreed" over zero entities would say the sync
    // is healthy on an install that has never synced anything.
    expect(summarise({})).toBe('nothing tracked yet');
  });
});
