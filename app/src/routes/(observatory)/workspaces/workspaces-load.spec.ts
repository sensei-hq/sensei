import { describe, it, expect } from 'vitest';
import { loadWorkspacesIndex } from './workspaces-load.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const workspace = (id: string, sessions7d: number) => ({ id, name: id, maturity: 'learning', stack: {}, sessions7d });

// #288: the workspaces index — a list that could not be read is an error, not
// "no workspaces"; only active workspaces pay for the quality fan-out.
describe('loadWorkspacesIndex', () => {
  it('a failed list read is an error, never an empty index', async () => {
    const api = {
      tryListWorkspaces: async () => down,
      tryGetWorkspaceFtr: async () => ok({ ftr14d: 0.9 }),
      tryGetWorkspaceQualitySignals: async () => ok({ open_drift_count: 0, ftr_7d: 0.9 }),
    };
    expect(await loadWorkspacesIndex(api)).toEqual({ buckets: null, error: 'Service Unavailable' });
  });

  it('reads quality for active workspaces only, and carries a failed read on its card', async () => {
    const asked: string[] = [];
    const api = {
      tryListWorkspaces: async () => ok([workspace('busy', 3), workspace('quiet', 0)]),
      tryGetWorkspaceFtr: async (id: string) => (asked.push(id), down),
      tryGetWorkspaceQualitySignals: async () => ok({ open_drift_count: 0, ftr_7d: 0.9 }),
    };
    const { buckets, error } = await loadWorkspacesIndex(api);
    expect(error).toBeNull();
    expect(asked).toEqual(['busy']);
    expect(buckets?.active.map((p) => [p.id, p.readError])).toEqual([['busy', 'Service Unavailable']]);
    expect(buckets?.dormant.map((p) => [p.id, p.readError])).toEqual([['quiet', null]]);
  });
});
