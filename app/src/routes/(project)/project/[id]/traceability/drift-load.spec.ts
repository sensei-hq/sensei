import { describe, it, expect } from 'vitest';
import { loadProjectDrift } from './drift-load.js';

const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

// #288: a drift read that failed is the page's error — not "0 tracked".
describe('loadProjectDrift', () => {
  it('carries the drift and its counts', async () => {
    const api = { tryGetProjectDrift: async () => ({ ok: true as const, data: { items: [], total: 18, drifted: 3, broken: 1 } }) };
    expect(await loadProjectDrift(api, 'p1')).toEqual({ driftItems: [], total: 18, drifted: 3, broken: 1, error: null });
  });
  it('a failed read is an error', async () => {
    const d = await loadProjectDrift({ tryGetProjectDrift: async () => down }, 'p1');
    expect([d.error, d.total]).toEqual(['Service Unavailable', 0]);
  });
});
