import { describe, it, expect } from 'vitest';
import { loadWorkspaceMemories, type MemoriesApi } from './memories-load.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const api = (over: Partial<MemoriesApi> = {}): MemoriesApi => ({
  tryGetWorkspaceMemories: async () => ok({ active: [], total: 11 }),
  tryListMemoryShareBatches: async () => ok({ batches: [] }),
  ...over,
});

// #288: the workspace's memories, or why they could not be read — not
// "Nothing to remember yet."
describe('loadWorkspaceMemories', () => {
  it('carries the memories and the proposed batches', async () => {
    let asked: string | undefined;
    const d = await loadWorkspaceMemories(api({ tryListMemoryShareBatches: async (_id, status) => ((asked = status), ok({ batches: [] })) }), 'p1');
    expect([d.error, d.total, d.memories, d.proposedBatches, asked]).toEqual([null, 11, [], [], 'proposed']);
  });
  it('a failed memories or batches read is the page\'s error', async () => {
    expect((await loadWorkspaceMemories(api({ tryGetWorkspaceMemories: async () => down }), 'p1')).error).toBe('Service Unavailable');
    expect((await loadWorkspaceMemories(api({ tryListMemoryShareBatches: async () => down }), 'p1')).error).toBe('Service Unavailable');
  });
});
