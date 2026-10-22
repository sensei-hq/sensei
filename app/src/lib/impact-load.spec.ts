import { describe, it, expect } from 'vitest';
import { loadImpactAcross, loadWorkspaceImpact, type ImpactApi } from './impact-load.js';
import type { WorkspaceImpactWire } from './impact.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const row = (id: string, verdict: string): WorkspaceImpactWire => ({
  id, title: id, actionType: 'audit_stale', status: 'applied', verdict,
  baselineFtr: 0.5, currentFtr: 0.7, ftrDelta: 0.2, props: {}, reasoning: null,
});
const api = (over: Partial<ImpactApi> = {}): ImpactApi => ({
  tryListWorkspaces: async () => ok([{ id: 'a', name: 'alpha' }, { id: 'b', name: 'beta' }]),
  tryGetWorkspaceImpact: async (id: string) => ok([row(`${id}-1`, 'positive')]),
  tryListImpactVerdicts: async () => ok({ verdicts: [] }),
  ...over,
});

// #288: Impact is the daemon's measured verdicts, or why it could not read them.
describe('loadImpactAcross', () => {
  it('buckets every workspace\'s rows', async () => {
    const d = await loadImpactAcross(api());
    expect([d.error, d.total, d.buckets?.positive.map((r) => r.workspaceName), d.unread]).toEqual([
      null, 2, ['alpha', 'beta'], [],
    ]);
  });

  it('a failed list read is the page\'s error', async () => {
    const d = await loadImpactAcross(api({ tryListWorkspaces: async () => down }));
    expect([d.error, d.buckets]).toEqual(['Service Unavailable', null]);
  });

  it('a workspace whose read failed is named, not dropped silently', async () => {
    const d = await loadImpactAcross(api({ tryGetWorkspaceImpact: async (id) => (id === 'b' ? down : ok([row('a-1', 'positive')])) }));
    expect([d.total, d.unread]).toEqual([1, [{ name: 'beta', reason: 'Service Unavailable' }]]);
  });
});

describe('loadWorkspaceImpact', () => {
  const workspace = { id: 'a', name: 'alpha' };
  it('carries the verdicts and the impact log', async () => {
    const d = await loadWorkspaceImpact(api(), workspace);
    expect([d.error, d.total, d.impactLog]).toEqual([null, 1, []]);
  });
  it('a failed impact read or log read is the page\'s error', async () => {
    expect((await loadWorkspaceImpact(api({ tryGetWorkspaceImpact: async () => down }), workspace)).error).toBe('Service Unavailable');
    expect((await loadWorkspaceImpact(api({ tryListImpactVerdicts: async () => down }), workspace)).error).toBe('Service Unavailable');
  });
});
