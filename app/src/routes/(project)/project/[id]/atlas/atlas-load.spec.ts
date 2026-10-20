import { describe, it, expect } from 'vitest';
import { loadAtlas, type AtlasApi } from './atlas-load.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const project = { id: 'p1', name: 'proj', folders: [{ id: 'f1', kind: 'git', name: 'repo-a' }] };

const api = (over: Partial<AtlasApi> = {}): AtlasApi => ({
  tryDetectCommunities: async () => ok({ ok: true, communities: 1 }),
  tryGetCommunities: async () => ok([]),
  tryGetCallFlow: async () => ok({ modules: [], calls: [], moduleCount: 4, exportCount: 9, callCount: 12 }),
  tryGetGraphNodes: async () => ok({ nodes: [], edges: [] }),
  tryGetProjectGraph: async () => ok({ repos: 1, nodes: 3, edges: 2 }),
  ...over,
});

// #288: the atlas is drawn from the daemon's reads or not at all — a read that
// failed is not an empty graph, zero calls or a 0-repo solution.
describe('loadAtlas', () => {
  it('builds the atlas from the reads, scoped to the first repo', async () => {
    const d = await loadAtlas(api(), project, null);
    expect(d.error).toBeNull();
    expect(d.repoId).toBe('repo-a');
    expect(d.atlas?.stats).toEqual({ modules: 4, exports: 9, calls: 12 });
    expect(d.atlas?.solution).toEqual({ repos: 1, nodes: 3, edges: 2 });
  });

  for (const read of ['tryDetectCommunities', 'tryGetCommunities', 'tryGetCallFlow', 'tryGetGraphNodes', 'tryGetProjectGraph'] as const) {
    it(`a failed ${read} is the page's error, naming it — no atlas from defaults`, async () => {
      const d = await loadAtlas(api({ [read]: async () => down }), project, 'repo-a');
      expect(d.atlas).toBeNull();
      expect(d.error).toContain('Service Unavailable');
      expect(d.repoId).toBe('repo-a');
    });
  }
});
