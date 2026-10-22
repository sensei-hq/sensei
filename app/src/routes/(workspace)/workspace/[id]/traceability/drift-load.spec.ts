import { describe, it, expect } from 'vitest';
import { loadWorkspaceDrift } from './drift-load.js';

const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

// #288: a drift read that failed is the page's error — not "0 tracked".
describe('loadWorkspaceDrift', () => {
  it('carries the drift and its counts', async () => {
    const api = { tryGetWorkspaceDrift: async () => ({ ok: true as const, data: { items: [], total: 18, drifted: 3, broken: 1, indexedDocs: 4 } }) };
    expect(await loadWorkspaceDrift(api, 'p1')).toEqual({ driftItems: [], total: 18, drifted: 3, broken: 1, indexedDocs: 4, error: null });
  });
  it('a failed read is an error', async () => {
    const d = await loadWorkspaceDrift({ tryGetWorkspaceDrift: async () => down }, 'p1');
    expect([d.error, d.total]).toEqual(['Service Unavailable', 0]);
  });
});
