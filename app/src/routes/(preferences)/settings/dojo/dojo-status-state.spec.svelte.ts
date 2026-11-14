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
  syncToneClass,
  entityLabel,
  agreementLine,
  detailLine,
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
    persona: 'jerrythomas',
    remedy: null,
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

describe('detailLine', () => {
  // A checkout's graph push records WHY it was skipped as a code (#225). The
  // screen says what that means for this checkout instead of printing the code.
  const graph = (over: Partial<SyncStateRow>) =>
    row({ entity: 'repository_graph', entity_key: 'github.com/sensei-hq/sensei@feat-x', ...over });

  it('says a checkout on an untracked branch keeps its graph on this machine', () => {
    expect(detailLine(graph({ state: 'skipped', last_error: 'not_a_tracked_branch' }))).toBe(
      'this branch is not one the dōjō tracks, so its graph stays on this machine',
    );
  });

  it('says a checkout whose index is behind its branch syncs once re-indexed', () => {
    expect(detailLine(graph({ state: 'skipped', last_error: 'index_not_current' }))).toBe(
      'the checkout moved to another branch since it was indexed; it syncs once re-indexed',
    );
  });

  it('passes an unknown reason through rather than hiding it', () => {
    expect(detailLine(graph({ state: 'skipped', last_error: 'branch_unknown' }))).toBe('branch_unknown');
  });

  it('shows a failure message as it was recorded', () => {
    const why = '1 file(s) over the 3145728-byte request cap, not shared: src/big.rs';
    expect(detailLine(graph({ state: 'error', last_error: why }))).toBe(why);
  });

  it('does not translate a failure that happens to spell a reason code', () => {
    expect(detailLine(graph({ state: 'error', last_error: 'not_a_tracked_branch' }))).toBe(
      'not_a_tracked_branch',
    );
  });

  it('is null when there is nothing to say', () => {
    expect(detailLine(graph({ state: 'synced', last_error: null }))).toBeNull();
  });
});

describe('a partial sync (#225)', () => {
  // A push that landed all but a named part: the sides agreed, and something
  // is missing. Neither a fault nor a clean agreement — and never "never agreed".
  const partial = row({ state: 'partial', last_error: '1 file(s) over the cap, not shared: big.rs' });

  it('reads as a warning, not a fault and not a success', () => {
    expect(syncTone('partial')).toBe('warn');
  });

  it('says it agreed, partly, and when', () => {
    expect(agreementLine(partial)).toBe('partly agreed 2026-08-31');
  });

  it('shows what is missing as recorded', () => {
    expect(detailLine(partial)).toBe('1 file(s) over the cap, not shared: big.rs');
  });

});

describe('syncToneClass', () => {
  it('colours each sync state from the state module, skipped muted and never as a fault', () => {
    expect(syncToneClass('synced')).toBe('text-success');
    expect(syncToneClass('partial')).toBe('text-warning');
    expect(syncToneClass('error')).toBe('text-danger');
    expect(syncToneClass('pending')).toBe('text-info');
    expect(syncToneClass('skipped')).toBe('text-ink-soft');
    expect(syncToneClass('something_new')).toBe('text-ink-soft');
  });
});

describe('repository verdicts', () => {
  const skippedRepo = (over: Partial<SyncStateRow>) =>
    row({
      entity: 'repository',
      entity_key: 'github.com/sensei-hq/dbd',
      state: 'skipped',
      synced_at: null,
      ...over,
    });

  it('a_skipped_repository_reads_muted_with_its_remedy', () => {
    // The remedy arrives server-joined from `sensei.reason_codes`; the screen
    // renders it, never a TS copy of the text.
    const r = skippedRepo({ last_error: 'denied:not_subscribed', remedy: 'Ask an admin to subscribe' });
    expect(syncTone(r.state)).toBe('muted');
    expect(detailLine(r)).toBe('Ask an admin to subscribe');
  });

  it('an_unknown_reason_passes_through_verbatim', () => {
    // No `repository_sharing` code names `no_connection`: without a joined
    // remedy the recorded reason shows as-is rather than blanking.
    const r = skippedRepo({ last_error: 'unmapped:no_connection', remedy: null });
    expect(detailLine(r)).toBe('unmapped:no_connection');
  });
});
