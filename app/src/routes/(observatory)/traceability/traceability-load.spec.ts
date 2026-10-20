import { describe, it, expect } from 'vitest';
import { loadDriftAcross, type DriftApi } from './traceability-load.js';
import type { DriftItem } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const item = (id: string, status: string) => ({ id, status, detail: id }) as unknown as DriftItem;
const drift = (items: DriftItem[]) => ok({ items, total: items.length, drifted: items.length, broken: 0 });

const api = (over: Partial<DriftApi> = {}): DriftApi => ({
  tryListProjects: async () => ok([{ id: 'a', name: 'alpha' }, { id: 'b', name: 'beta' }]),
  tryGetProjectDrift: async (id) => drift([item(`${id}-1`, 'drifted')]),
  ...over,
});

// #288: drift across projects, or why it could not be read — a project whose
// read failed is named, never counted as clean.
describe('loadDriftAcross', () => {
  it('groups every project\'s drift', async () => {
    const d = await loadDriftAcross(api());
    expect([d.error, d.rollup?.total, d.groups?.map((g) => g.projectName), d.unread]).toEqual([null, 2, ['alpha', 'beta'], []]);
  });
  it('a failed list read is the page\'s error', async () => {
    const d = await loadDriftAcross(api({ tryListProjects: async () => down }));
    expect([d.error, d.groups, d.rollup]).toEqual(['Service Unavailable', null, null]);
  });
  it('names a project it could not read', async () => {
    const d = await loadDriftAcross(api({ tryGetProjectDrift: async (id) => (id === 'b' ? down : drift([item('a-1', 'drifted')])) }));
    expect([d.rollup?.total, d.unread]).toEqual([1, [{ name: 'beta', reason: 'Service Unavailable' }]]);
  });
});
