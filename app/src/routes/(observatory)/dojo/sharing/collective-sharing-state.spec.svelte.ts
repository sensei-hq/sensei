import { describe, it, expect, vi } from 'vitest';
import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { CollectivePreferences } from '$lib/types.js';
import {
  CollectiveSharing,
  ATTRIBUTION_OPTIONS,
  CADENCE_OPTIONS,
  CATEGORY_META,
  collectiveDateLabel,
  destinationFromToggles,
  setDojo,
  setGlobal,
  togglesFromDestination,
  updatedLabel,
  withAttribution,
  withCadence,
  withCategory,
  withDestination,
} from './collective-sharing-state.svelte.js';

const prefs = (over: Partial<CollectivePreferences> = {}): CollectivePreferences => ({
  destination: 'none',
  cadence: 'manual',
  categories: {
    memory: true, pattern: true, rule: true, prompt: true, guard: true, skill: true, agent: true,
  },
  attribution_default: 'dereferenced',
  updated_at: null,
  ...over,
});

// ── destination <-> two-toggle mapping ───────────────────────────────────────
describe('destinationFromToggles', () => {
  it('both on -> both', () => expect(destinationFromToggles(true, true)).toBe('both'));
  it('global only -> global', () => expect(destinationFromToggles(true, false)).toBe('global'));
  it('dojo only -> dojo', () => expect(destinationFromToggles(false, true)).toBe('dojo'));
  it('neither -> none', () => expect(destinationFromToggles(false, false)).toBe('none'));
});

describe('togglesFromDestination', () => {
  it('both -> both on', () => expect(togglesFromDestination('both')).toEqual({ global: true, dojo: true }));
  it('global -> global on only', () =>
    expect(togglesFromDestination('global')).toEqual({ global: true, dojo: false }));
  it('dojo -> dojo on only', () =>
    expect(togglesFromDestination('dojo')).toEqual({ global: false, dojo: true }));
  it('none -> both off', () => expect(togglesFromDestination('none')).toEqual({ global: false, dojo: false }));
  it('an unexpected value degrades to both off (never a broken toggle)', () =>
    expect(togglesFromDestination('galaxy')).toEqual({ global: false, dojo: false }));
});

// ── pure read-modify-write builders ──────────────────────────────────────────
describe('withDestination', () => {
  it('replaces destination and leaves the input untouched', () => {
    const p = prefs();
    expect(withDestination(p, 'both').destination).toBe('both');
    expect(p.destination).toBe('none');
  });
});

describe('setGlobal / setDojo', () => {
  it('setGlobal on from none -> global', () => expect(setGlobal(prefs(), true).destination).toBe('global'));
  it('setDojo on from none -> dojo', () => expect(setDojo(prefs(), true).destination).toBe('dojo'));
  it('setGlobal on while dojo is already on -> both (preserves the other toggle)', () =>
    expect(setGlobal(prefs({ destination: 'dojo' }), true).destination).toBe('both'));
  it('setDojo on while global is already on -> both', () =>
    expect(setDojo(prefs({ destination: 'global' }), true).destination).toBe('both'));
  it('setGlobal off from both -> dojo (only the global toggle drops)', () =>
    expect(setGlobal(prefs({ destination: 'both' }), false).destination).toBe('dojo'));
  it('setDojo off from both -> global', () =>
    expect(setDojo(prefs({ destination: 'both' }), false).destination).toBe('global'));
  it('setGlobal off from global -> none', () =>
    expect(setGlobal(prefs({ destination: 'global' }), false).destination).toBe('none'));
});

describe('withCadence / withAttribution', () => {
  it('withCadence replaces cadence only', () => {
    const p = prefs();
    const next = withCadence(p, 'weekly');
    expect(next.cadence).toBe('weekly');
    expect(next.destination).toBe(p.destination);
    expect(p.cadence).toBe('manual');
  });
  it('withAttribution replaces attribution_default only', () => {
    const p = prefs();
    expect(withAttribution(p, 'named').attribution_default).toBe('named');
    expect(p.attribution_default).toBe('dereferenced');
  });
});

describe('withCategory', () => {
  it('flips exactly one key and preserves the other six, leaving the input untouched', () => {
    const p = prefs();
    const next = withCategory(p, 'memory', false);
    expect(next.categories.memory).toBe(false);
    // the other six keys are untouched
    expect(next.categories.pattern).toBe(true);
    expect(next.categories.rule).toBe(true);
    expect(next.categories.prompt).toBe(true);
    expect(next.categories.guard).toBe(true);
    expect(next.categories.skill).toBe(true);
    expect(next.categories.agent).toBe(true);
    // input immutability
    expect(p.categories.memory).toBe(true);
    expect(next.categories).not.toBe(p.categories);
  });
});

describe('collectiveDateLabel / updatedLabel', () => {
  it('takes the date portion of an RFC-3339 timestamp', () =>
    expect(collectiveDateLabel('2026-07-08T12:34:56Z')).toBe('2026-07-08'));
  it('is empty for a missing timestamp', () => expect(collectiveDateLabel(null)).toBe(''));
  it('updatedLabel reads "last changed …" when saved', () =>
    expect(updatedLabel('2026-07-08T00:00:00Z')).toBe('last changed 2026-07-08'));
  it('updatedLabel signals defaults when never saved', () =>
    expect(updatedLabel(null)).toBe('using defaults · never changed'));
});

describe('option metadata', () => {
  it('carries all seven categories in the wire key order', () =>
    expect(CATEGORY_META.map((c) => c.key)).toEqual([
      'memory', 'pattern', 'rule', 'prompt', 'guard', 'skill', 'agent',
    ]));
  it('offers the three cadences', () =>
    expect(CADENCE_OPTIONS.map((c) => c.value)).toEqual(['manual', 'daily', 'weekly']));
  it('offers the three attribution modes, dereferenced last (the default)', () =>
    expect(ATTRIBUTION_OPTIONS.map((a) => a.value)).toEqual(['named', 'anonymous', 'dereferenced']));
});

// ── CollectiveSharing controller ─────────────────────────────────────────────
// Hand-rolled mock api — only putCollectivePreferences is exercised; cast to
// SenseiApi since the controller touches nothing else. By default it echoes the
// PUT body back with a fresh updated_at, mirroring the whole-object full-replace.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    putCollectivePreferences: vi
      .fn()
      .mockImplementation((body: CollectivePreferences): Promise<ApiResult<CollectivePreferences>> =>
        Promise.resolve({ ok: true, data: { ...body, updated_at: '2026-07-08T09:00:00Z' } }),
      ),
    ...overrides,
  } as unknown as SenseiApi;
}

describe('CollectiveSharing — defaults render', () => {
  it('reflects the seeded defaults through its getters', () => {
    const state = new CollectiveSharing(prefs(), mockApi());
    expect(state.destination).toBe('none');
    expect(state.globalOn).toBe(false);
    expect(state.dojoOn).toBe(false);
    expect(state.sharing).toBe(false);
    expect(state.cadence).toBe('manual');
    expect(state.attribution).toBe('dereferenced');
    expect(CATEGORY_META.every((c) => state.isCategoryOn(c.key))).toBe(true);
    expect(state.neverSaved).toBe(true);
    expect(state.updatedLabel).toBe('using defaults · never changed');
  });
});

describe('CollectiveSharing — destination enum mapping via saves', () => {
  it('setGlobal(true) from none PUTs destination "global"', async () => {
    const putCollectivePreferences = vi
      .fn()
      .mockResolvedValue({ ok: true, data: prefs({ destination: 'global', updated_at: 'x' }) });
    const state = new CollectiveSharing(prefs(), mockApi({ putCollectivePreferences }));

    await state.setGlobal(true);
    expect(putCollectivePreferences.mock.calls[0][0].destination).toBe('global');
  });

  it('setDojo(true) while global is on PUTs destination "both"', async () => {
    const putCollectivePreferences = vi
      .fn()
      .mockResolvedValue({ ok: true, data: prefs({ destination: 'both', updated_at: 'x' }) });
    const state = new CollectiveSharing(prefs({ destination: 'global' }), mockApi({ putCollectivePreferences }));

    await state.setDojo(true);
    expect(putCollectivePreferences.mock.calls[0][0].destination).toBe('both');
  });

  it('setGlobal(false) from both PUTs destination "dojo"', async () => {
    const putCollectivePreferences = vi
      .fn()
      .mockResolvedValue({ ok: true, data: prefs({ destination: 'dojo', updated_at: 'x' }) });
    const state = new CollectiveSharing(prefs({ destination: 'both' }), mockApi({ putCollectivePreferences }));

    await state.setGlobal(false);
    expect(putCollectivePreferences.mock.calls[0][0].destination).toBe('dojo');
  });

  it('turning both destinations off PUTs "none"', async () => {
    const putCollectivePreferences = vi
      .fn()
      .mockResolvedValue({ ok: true, data: prefs({ destination: 'none', updated_at: 'x' }) });
    const state = new CollectiveSharing(prefs({ destination: 'global' }), mockApi({ putCollectivePreferences }));

    await state.setGlobal(false);
    expect(putCollectivePreferences.mock.calls[0][0].destination).toBe('none');
  });
});

describe('CollectiveSharing — read-modify-write sends the WHOLE object', () => {
  it('a single cadence change PUTs every field, not just the changed one', async () => {
    const putCollectivePreferences = vi
      .fn()
      .mockResolvedValue({ ok: true, data: prefs({ cadence: 'daily', updated_at: 'x' }) });
    const state = new CollectiveSharing(prefs({ destination: 'both' }), mockApi({ putCollectivePreferences }));

    await state.setCadence('daily');
    const sent = putCollectivePreferences.mock.calls[0][0] as CollectivePreferences;
    // the changed field
    expect(sent.cadence).toBe('daily');
    // every other field carried through — whole-object full-replace
    expect(sent.destination).toBe('both');
    expect(sent.attribution_default).toBe('dereferenced');
    expect(Object.keys(sent.categories).sort()).toEqual([
      'agent', 'guard', 'memory', 'pattern', 'prompt', 'rule', 'skill',
    ]);
    expect(sent).toHaveProperty('updated_at');
  });
});

describe('CollectiveSharing — category toggle flips one key only', () => {
  it('PUTs the whole categories map with exactly one key flipped', async () => {
    const putCollectivePreferences = vi.fn().mockResolvedValue({
      ok: true,
      data: prefs({ updated_at: 'x' }),
    });
    const state = new CollectiveSharing(prefs({ destination: 'global' }), mockApi({ putCollectivePreferences }));

    await state.toggleCategory('rule');
    const sent = putCollectivePreferences.mock.calls[0][0] as CollectivePreferences;
    expect(sent.categories.rule).toBe(false);
    // the other six stay on
    expect(sent.categories.memory).toBe(true);
    expect(sent.categories.pattern).toBe(true);
    expect(sent.categories.prompt).toBe(true);
    expect(sent.categories.guard).toBe(true);
    expect(sent.categories.skill).toBe(true);
    expect(sent.categories.agent).toBe(true);
  });
});

describe('CollectiveSharing — adopts the server-returned saved object', () => {
  it('current becomes exactly what the daemon echoed back (fresh updated_at)', async () => {
    const saved = prefs({ destination: 'global', updated_at: '2026-07-08T09:00:00Z' });
    const putCollectivePreferences = vi.fn().mockResolvedValue({ ok: true, data: saved });
    const state = new CollectiveSharing(prefs(), mockApi({ putCollectivePreferences }));

    const ok = await state.setGlobal(true);
    expect(ok).toBe(true);
    expect(state.current).toBe(saved); // the server object IS the new truth
    expect(state.neverSaved).toBe(false);
    expect(state.updatedLabel).toBe('last changed 2026-07-08');
    expect(state.saving).toBe(false);
    expect(state.error).toBeNull();
  });
});

describe('CollectiveSharing — a 400 surfaces the error and leaves state untouched', () => {
  it('sets error to the daemon message and does not mutate current', async () => {
    const putCollectivePreferences = vi
      .fn()
      .mockResolvedValue({ ok: false, error: { status: 400, message: 'invalid destination' } });
    const before = prefs();
    const state = new CollectiveSharing(before, mockApi({ putCollectivePreferences }));

    const ok = await state.setGlobal(true);
    expect(ok).toBe(false);
    expect(state.error).toBe('invalid destination');
    expect(state.current).toBe(before); // unchanged — the toggle visually reverts
    expect(state.destination).toBe('none');
    expect(state.saving).toBe(false);
  });
});

describe('CollectiveSharing — one write at a time', () => {
  it('ignores a second toggle while a save is in flight', async () => {
    let resolveCall: (v: ApiResult<CollectivePreferences>) => void = () => {};
    const putCollectivePreferences = vi.fn().mockReturnValue(
      new Promise<ApiResult<CollectivePreferences>>((r) => {
        resolveCall = r;
      }),
    );
    const state = new CollectiveSharing(prefs(), mockApi({ putCollectivePreferences }));

    const first = state.setGlobal(true);
    expect(state.saving).toBe(true);
    const second = await state.setDojo(true); // ignored while busy
    expect(second).toBe(false);
    expect(putCollectivePreferences).toHaveBeenCalledOnce();

    resolveCall({ ok: true, data: prefs({ destination: 'global', updated_at: 'x' }) });
    await first;
    expect(state.saving).toBe(false);
  });
});
