import { describe, it, expect } from 'vitest';
import { loadProjectInstruments, type InstrumentsApi } from './instruments-load.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const api = (over: Partial<InstrumentsApi> = {}): InstrumentsApi => ({
  tryGetProjectInstruments: async () => ok({ tools: [{ id: 't', name: 'skill-a', kind: 'skill', scope: 'project' as const, enabled: true }] }),
  tryGetProjectMcpToolStats: async () => ok({ tools: [] }),
  tryListProjectServices: async () => ok({ services: [] }),
  ...over,
});

// #288: the project's instruments, or why they could not be read — not
// "0 tools · 0 calls · 0 errors".
describe('loadProjectInstruments', () => {
  it('carries the three reads', async () => {
    const d = await loadProjectInstruments(api(), 'p1');
    expect([d.error, d.tools.map((t) => t.name), d.mcpToolStats, d.services]).toEqual([null, ['skill-a'], [], []]);
  });
  for (const read of ['tryGetProjectInstruments', 'tryGetProjectMcpToolStats', 'tryListProjectServices'] as const) {
    it(`a failed ${read} is the page's error`, async () => {
      expect((await loadProjectInstruments(api({ [read]: async () => down }), 'p1')).error).toBe('Service Unavailable');
    });
  }
});
