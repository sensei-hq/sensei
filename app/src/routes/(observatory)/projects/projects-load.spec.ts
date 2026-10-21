import { describe, it, expect } from 'vitest';
import { loadProjectsIndex } from './projects-load.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const project = (id: string, sessions7d: number) => ({ id, name: id, maturity: 'learning', stack: {}, sessions7d });

// #288: the projects index — a list that could not be read is an error, not
// "no projects"; only active projects pay for the quality fan-out.
describe('loadProjectsIndex', () => {
  it('a failed list read is an error, never an empty index', async () => {
    const api = {
      tryListProjects: async () => down,
      tryGetProjectFtr: async () => ok({ ftr14d: 0.9 }),
      tryGetProjectQualitySignals: async () => ok({ open_drift_count: 0, ftr_7d: 0.9 }),
    };
    expect(await loadProjectsIndex(api)).toEqual({ buckets: null, error: 'Service Unavailable' });
  });

  it('reads quality for active projects only, and carries a failed read on its card', async () => {
    const asked: string[] = [];
    const api = {
      tryListProjects: async () => ok([project('busy', 3), project('quiet', 0)]),
      tryGetProjectFtr: async (id: string) => (asked.push(id), down),
      tryGetProjectQualitySignals: async () => ok({ open_drift_count: 0, ftr_7d: 0.9 }),
    };
    const { buckets, error } = await loadProjectsIndex(api);
    expect(error).toBeNull();
    expect(asked).toEqual(['busy']);
    expect(buckets?.active.map((p) => [p.id, p.readError])).toEqual([['busy', 'Service Unavailable']]);
    expect(buckets?.dormant.map((p) => [p.id, p.readError])).toEqual([['quiet', null]]);
  });
});
