import { describe, it, expect, vi } from 'vitest';
import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { DojoUpgrade } from '$lib/types.js';
import {
  UpgradeActions,
  attributionSummary,
  orderUpgrades,
  receivedLabel,
  scopeSummary,
  stateChip,
  stateTone,
  typePill,
  unreadCount,
} from './dojo-upgrades-state.svelte.js';

const upgrade = (over: Partial<DojoUpgrade> = {}): DojoUpgrade => ({
  id: 'u1',
  membership_id: 'm1',
  type: 'principle',
  title: 'prefer small functions',
  body: 'keep units testable',
  scope: {},
  attribution: { mode: 'anonymous', dereferenced: false },
  state: 'pending',
  received_at: '2026-07-01T00:00:00Z',
  ...over,
});

describe('typePill', () => {
  it('skill -> 技, accent', () =>
    expect(typePill('skill')).toEqual({ kanji: '技', label: 'skill', bg: 'bg-accent-soft', text: 'text-accent' }));
  it('agent -> 作, accent', () => expect(typePill('agent').kanji).toBe('作'));
  it('principle -> 理, success', () =>
    expect(typePill('principle')).toMatchObject({ kanji: '理', text: 'text-success' }));
  it('pattern -> 紋, info', () => expect(typePill('pattern')).toMatchObject({ kanji: '紋', text: 'text-info' }));
  it('prompt -> 問, info', () => expect(typePill('prompt').kanji).toBe('問'));
  it('guard -> 守, warning', () => expect(typePill('guard')).toMatchObject({ kanji: '守', text: 'text-warning' }));
  it('unknown type degrades to muted ink but keeps the raw label', () => {
    const p = typePill('freshly_invented');
    expect(p.bg).toBe('bg-paper-mute');
    expect(p.text).toBe('text-ink-mute');
    expect(p.label).toBe('freshly_invented');
  });
});

describe('stateTone', () => {
  it('pending -> info', () => expect(stateTone('pending')).toBe('info'));
  it('applied -> success', () => expect(stateTone('applied')).toBe('success'));
  it('pinned -> accent', () => expect(stateTone('pinned')).toBe('accent'));
  it('muted -> ink', () => expect(stateTone('muted')).toBe('ink'));
  it('degrades an unexpected state to ink (never dropped)', () => expect(stateTone('weird')).toBe('ink'));
});

describe('stateChip', () => {
  it('applied -> success tokens, labelled with the raw state', () =>
    expect(stateChip('applied')).toEqual({ bg: 'bg-success-soft', text: 'text-success', label: 'applied' }));
  it('pending -> info tokens', () =>
    expect(stateChip('pending')).toEqual({ bg: 'bg-info-soft', text: 'text-info', label: 'pending' }));
  it('pinned -> accent tokens', () =>
    expect(stateChip('pinned')).toEqual({ bg: 'bg-accent-soft', text: 'text-accent', label: 'pinned' }));
  it('muted -> muted ink tokens, still labelled', () =>
    expect(stateChip('muted')).toEqual({ bg: 'bg-paper-mute', text: 'text-ink-mute', label: 'muted' }));
});

describe('scopeSummary', () => {
  it('project wins over stack', () =>
    expect(scopeSummary({ project: 'lumen', stack: 'rust' })).toBe('project · lumen'));
  it('stack is next', () => expect(scopeSummary({ stack: 'rust' })).toBe('stack · rust'));
  it('team is next', () => expect(scopeSummary({ team: 'mobile' })).toBe('team · mobile'));
  it('company is next', () => expect(scopeSummary({ company: 'acme' })).toBe('company · acme'));
  it('an empty scope reads all projects', () => expect(scopeSummary({})).toBe('all projects'));
  it('a nullish scope reads all projects', () => expect(scopeSummary(undefined)).toBe('all projects'));
});

describe('attributionSummary', () => {
  it('named with author and org', () =>
    expect(attributionSummary({ mode: 'named', author: 'Ada', org: 'Acme', dereferenced: false })).toBe('by Ada · Acme'));
  it('named with author only', () =>
    expect(attributionSummary({ mode: 'named', author: 'Ada', dereferenced: false })).toBe('by Ada'));
  it('named with org only', () =>
    expect(attributionSummary({ mode: 'named', org: 'Acme', dereferenced: false })).toBe('from Acme'));
  it('anonymous', () => expect(attributionSummary({ mode: 'anonymous', dereferenced: false })).toBe('anonymous'));
  it('dereferenced source is withheld', () =>
    expect(attributionSummary({ mode: 'dereferenced', dereferenced: true })).toBe('source withheld'));
  it('a nullish attribution reads unattributed', () => expect(attributionSummary(undefined)).toBe('unattributed'));
});

describe('receivedLabel', () => {
  it('takes the date portion of an ISO timestamp', () =>
    expect(receivedLabel('2026-07-01T12:34:56Z')).toBe('2026-07-01'));
  it('is empty for a missing timestamp', () => expect(receivedLabel(undefined)).toBe(''));
  it('is empty for an unparseable value', () => expect(receivedLabel('not-a-date')).toBe(''));
});

describe('orderUpgrades', () => {
  it('floats pinned to the top, then newest-first, without mutating the input', () => {
    const items = [
      upgrade({ id: 'a', state: 'pending', received_at: '2026-07-01T00:00:00Z' }),
      upgrade({ id: 'b', state: 'pinned', received_at: '2026-06-01T00:00:00Z' }), // oldest, but pinned
      upgrade({ id: 'c', state: 'pending', received_at: '2026-07-03T00:00:00Z' }),
    ];
    const ordered = orderUpgrades(items);
    expect(ordered.map((u) => u.id)).toEqual(['b', 'c', 'a']);
    expect(items.map((u) => u.id)).toEqual(['a', 'b', 'c']); // input untouched
  });
});

describe('unreadCount', () => {
  it('counts only pending items', () => {
    expect(
      unreadCount([
        upgrade({ id: 'a', state: 'pending' }),
        upgrade({ id: 'b', state: 'pinned' }),
        upgrade({ id: 'c', state: 'applied' }),
        upgrade({ id: 'd', state: 'pending' }),
      ]),
    ).toBe(2);
  });
});

// ── UpgradeActions ────────────────────────────────────────────────────────────
// Hand-rolled mock api — only the three upgrade mutations are exercised; cast to
// SenseiApi since the controller touches nothing else.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  const ok: ApiResult<Record<string, unknown>> = { ok: true, data: {} };
  return {
    applyUpgrade: vi.fn().mockResolvedValue(ok),
    muteUpgrade: vi.fn().mockResolvedValue(ok),
    pinUpgrade: vi.fn().mockResolvedValue(ok),
    ...overrides,
  } as unknown as SenseiApi;
}

describe('UpgradeActions', () => {
  it('apply posts, reloads, resolves true and clears busy + error', async () => {
    const applyUpgrade = vi.fn().mockResolvedValue({ ok: true, data: {} });
    const reload = vi.fn().mockResolvedValue(undefined);
    const actions = new UpgradeActions(mockApi({ applyUpgrade }), reload);

    const ok = await actions.apply('u1');
    expect(ok).toBe(true);
    expect(applyUpgrade).toHaveBeenCalledWith('u1');
    expect(reload).toHaveBeenCalledOnce();
    expect(actions.busyId).toBeNull();
    expect(actions.error).toBeNull();
  });

  it('mute and pin call their endpoints and reload on success', async () => {
    const muteUpgrade = vi.fn().mockResolvedValue({ ok: true, data: {} });
    const pinUpgrade = vi.fn().mockResolvedValue({ ok: true, data: {} });
    const reload = vi.fn().mockResolvedValue(undefined);
    const actions = new UpgradeActions(mockApi({ muteUpgrade, pinUpgrade }), reload);

    expect(await actions.mute('u1')).toBe(true);
    expect(await actions.pin('u2')).toBe(true);
    expect(muteUpgrade).toHaveBeenCalledWith('u1');
    expect(pinUpgrade).toHaveBeenCalledWith('u2');
    expect(reload).toHaveBeenCalledTimes(2);
  });

  it('a failed action surfaces the wire error, does not reload, resolves false', async () => {
    const applyUpgrade = vi
      .fn()
      .mockResolvedValue({ ok: false, error: { status: 404, message: 'upgrade not found' } });
    const reload = vi.fn();
    const actions = new UpgradeActions(mockApi({ applyUpgrade }), reload);

    const ok = await actions.apply('gone');
    expect(ok).toBe(false);
    expect(actions.error).toBe('upgrade not found');
    expect(reload).not.toHaveBeenCalled();
    expect(actions.busyId).toBeNull();
  });

  it('isBusy tracks the in-flight id for the duration of the call', async () => {
    let resolveCall: (v: ApiResult<unknown>) => void = () => {};
    const applyUpgrade = vi.fn().mockReturnValue(
      new Promise<ApiResult<unknown>>((r) => {
        resolveCall = r;
      }),
    );
    const actions = new UpgradeActions(mockApi({ applyUpgrade }), vi.fn());

    const pending = actions.apply('u1');
    expect(actions.isBusy('u1')).toBe(true);
    expect(actions.isBusy('other')).toBe(false);
    resolveCall({ ok: true, data: {} });
    await pending;
    expect(actions.isBusy('u1')).toBe(false);
  });
});
