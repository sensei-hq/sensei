import { describe, it, expect } from 'vitest';
import { loadUpgrades, type UpgradesApi } from './upgrades-load.js';
import type { Recommendation } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const rec = (id: string, action_type: string) =>
  ({ id, title: id, why: 'w', urgency: 'high', status: 'pending', action_type }) as unknown as Recommendation;

const api = (over: Partial<UpgradesApi> = {}): UpgradesApi => ({
  tryGetUpgrades: async () => ok({ artifacts: [], unread_count: 2 }),
  tryListWorkspaces: async () => ok([{ id: 'a', name: 'alpha' }, { id: 'b', name: 'beta' }]),
  tryGetWorkspaceRecommendations: async (id) => ok([rec(`${id}-skill`, 'write_skill'), rec(`${id}-other`, 'not_installable')]),
  ...over,
});

// #288: each lane is the daemon's read or why it failed — a dōjō that could
// not be read is not "no upgrades yet", a workspace that could not be read is not
// a workspace with nothing waiting.
describe('loadUpgrades', () => {
  it('keeps the installable recs of every workspace, and the dōjō inbox', async () => {
    const d = await loadUpgrades(api());
    expect([d.localError, d.total, d.unread, d.dojoError, d.dojoUnread]).toEqual([null, 2, [], null, 2]);
  });

  it('a failed dōjō read is that lane\'s error; the local lane still loads', async () => {
    const d = await loadUpgrades(api({ tryGetUpgrades: async () => down }));
    expect([d.dojoError, d.dojoUpgrades, d.total]).toEqual(['Service Unavailable', [], 2]);
  });

  it('a failed list read is the local lane\'s error', async () => {
    const d = await loadUpgrades(api({ tryListWorkspaces: async () => down }));
    expect([d.localError, d.buckets]).toEqual(['Service Unavailable', null]);
  });

  it('names a workspace whose recommendations could not be read', async () => {
    const d = await loadUpgrades(api({ tryGetWorkspaceRecommendations: async (id) => (id === 'b' ? down : ok([rec('a-skill', 'write_skill')])) }));
    expect([d.total, d.unread]).toEqual([1, [{ name: 'beta', reason: 'Service Unavailable' }]]);
  });
});
