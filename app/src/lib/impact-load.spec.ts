import { describe, it, expect } from 'vitest';
import { loadImpactAcross, loadProjectImpact, type ImpactApi } from './impact-load.js';
import type { ProjectImpactWire } from './impact.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const row = (id: string, verdict: string): ProjectImpactWire => ({
  id, title: id, actionType: 'audit_stale', status: 'applied', verdict,
  baselineFtr: 0.5, currentFtr: 0.7, ftrDelta: 0.2, props: {}, reasoning: null,
});
const api = (over: Partial<ImpactApi> = {}): ImpactApi => ({
  tryListProjects: async () => ok([{ id: 'a', name: 'alpha' }, { id: 'b', name: 'beta' }]),
  tryGetProjectImpact: async (id: string) => ok([row(`${id}-1`, 'positive')]),
  tryListImpactVerdicts: async () => ok({ verdicts: [] }),
  ...over,
});

// #288: Impact is the daemon's measured verdicts, or why it could not read them.
describe('loadImpactAcross', () => {
  it('buckets every project\'s rows', async () => {
    const d = await loadImpactAcross(api());
    expect([d.error, d.total, d.buckets?.positive.map((r) => r.projectName), d.unread]).toEqual([
      null, 2, ['alpha', 'beta'], [],
    ]);
  });

  it('a failed list read is the page\'s error', async () => {
    const d = await loadImpactAcross(api({ tryListProjects: async () => down }));
    expect([d.error, d.buckets]).toEqual(['Service Unavailable', null]);
  });

  it('a project whose read failed is named, not dropped silently', async () => {
    const d = await loadImpactAcross(api({ tryGetProjectImpact: async (id) => (id === 'b' ? down : ok([row('a-1', 'positive')])) }));
    expect([d.total, d.unread]).toEqual([1, [{ name: 'beta', reason: 'Service Unavailable' }]]);
  });
});

describe('loadProjectImpact', () => {
  const project = { id: 'a', name: 'alpha' };
  it('carries the verdicts and the impact log', async () => {
    const d = await loadProjectImpact(api(), project);
    expect([d.error, d.total, d.impactLog]).toEqual([null, 1, []]);
  });
  it('a failed impact read or log read is the page\'s error', async () => {
    expect((await loadProjectImpact(api({ tryGetProjectImpact: async () => down }), project)).error).toBe('Service Unavailable');
    expect((await loadProjectImpact(api({ tryListImpactVerdicts: async () => down }), project)).error).toBe('Service Unavailable');
  });
});
