import { describe, it, expect } from 'vitest';
import { loadDriftAcross, type DriftApi } from './traceability-load.js';
import type { DriftItem } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const item = (id: string, status: string) => ({ id, status, detail: id }) as unknown as DriftItem;
const drift = (items: DriftItem[]) => ok({ items, total: items.length, drifted: items.length, broken: 0, indexedDocs: 2 });

const api = (over: Partial<DriftApi> = {}): DriftApi => ({
  tryListWorkspaces: async () => ok([{ id: 'a', name: 'alpha' }, { id: 'b', name: 'beta' }]),
  tryGetWorkspaceDrift: async (id) => drift([item(`${id}-1`, 'drifted')]),
  ...over,
});

// #288: drift across workspaces, or why it could not be read — a workspace whose
// read failed is named, never counted as clean.
describe('loadDriftAcross', () => {
  it('groups every workspace\'s drift', async () => {
    const d = await loadDriftAcross(api());
    expect([d.error, d.rollup?.total, d.groups?.map((g) => g.workspaceName), d.unread, d.indexedDocs]).toEqual([null, 2, ['alpha', 'beta'], [], 4]);
  });
  it('a failed list read is the page\'s error', async () => {
    const d = await loadDriftAcross(api({ tryListWorkspaces: async () => down }));
    expect([d.error, d.groups, d.rollup]).toEqual(['Service Unavailable', null, null]);
  });
  it('names a workspace it could not read', async () => {
    const d = await loadDriftAcross(api({ tryGetWorkspaceDrift: async (id) => (id === 'b' ? down : drift([item('a-1', 'drifted')])) }));
    expect([d.rollup?.total, d.unread]).toEqual([1, [{ name: 'beta', reason: 'Service Unavailable' }]]);
  });
});
