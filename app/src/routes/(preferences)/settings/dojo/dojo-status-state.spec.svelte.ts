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
  detailLine,
  groupSync,
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

  it('ranks after pending and before a skip', () => {
    expect(worstSyncState({ partial: 1, skipped: 1, synced: 1 })).toBe('partial');
    expect(worstSyncState({ pending: 1, partial: 1 })).toBe('pending');
  });

  it('is counted in the summary by name', () => {
    expect(summarise({ synced: 3, partial: 1 })).toBe('4 tracked · 1 partial');
  });
});


// The user (2026-10-27): one flat list mixed identities, days and branches —
// `torii` and `torii@develop` read as two repositories, and "default" (a
// Keychain slot) read as "the default identity". Grouped: identity (by its
// NAME) → day → repository, each branch under its repository.
describe('groupSync', () => {
  const who = [
    { slot: 'default', label: 'sensei-hq-org' },
    { slot: 'jerrythomas', label: 'jerrythomas' },
  ];

  it('groups by identity name, then day (newest first), then repository, with branches folded in', () => {
    const rows = [
      row({ entity: 'repository_metric', entity_key: 'github.com/sensei-hq/torii', synced_at: '2026-10-27T10:00:00Z' }),
      row({ entity: 'repository_graph', entity_key: 'github.com/sensei-hq/torii@develop', state: 'skipped', synced_at: null, attempted_at: '2026-10-27T11:00:00Z' }),
      row({ entity: 'repository_metric', entity_key: 'github.com/jerrythomas/rokkit', synced_at: '2026-10-20T10:00:00Z' }),
      row({ entity: 'forge_mirror', entity_key: 'default', direction: 'pull', state: 'error', synced_at: null, attempted_at: '2026-10-27T09:00:00Z', persona: 'default' }),
    ];
    const g = groupSync(rows, who);
    expect(g.map((i) => i.label)).toEqual(['jerrythomas', 'sensei-hq-org']);
    const mine = g[0];
    expect(mine.days.map((d) => d.day)).toEqual(['2026-10-27', '2026-10-20']);
    const torii = mine.days[0].repositories;
    expect(torii.map((r) => r.name)).toEqual(['github.com/sensei-hq/torii']);
    expect(torii[0].items.map((i) => [i.row.entity, i.branch])).toEqual([
      ['repository_metric', null],
      ['repository_graph', 'develop'],
    ]);
    // An identity's own entities (its sync plan, its forge mirror) are its own,
    // not a repository's.
    expect(g[1].days[0].own.map((i) => i.row.entity)).toEqual(['forge_mirror']);
  });

  it('keeps rows no identity has written yet, in a group of their own, last', () => {
    const g = groupSync([row({ persona: null }), row({ entity_key: 'x', persona: 'jerrythomas' })], who);
    expect(g.map((i) => i.label)).toEqual(['jerrythomas', null]);
  });

  it('a row never agreed nor tried has no day, and sorts after the dated ones', () => {
    const g = groupSync([row({ synced_at: null, attempted_at: null }), row({ entity_key: 'y' })], who);
    expect(g[0].days.map((d) => d.day)).toEqual(['2026-08-31', null]);
  });
});
