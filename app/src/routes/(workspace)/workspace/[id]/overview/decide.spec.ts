import { describe, it, expect, vi } from 'vitest';
import { decideRecommendation, type DecideApi } from './overview-view.svelte.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

function api(over: Partial<DecideApi> = {}) {
  const calls: string[] = [];
  const a: DecideApi = {
    tryPreviewRecommendation: async () => (calls.push('preview'), ok({ materializable: true, kind: 'skill', path: '.claude/skills/x', consent_required: true })),
    tryMaterializeRecommendation: async (_p, _r, opts) => (calls.push(`materialize ${JSON.stringify(opts)}`), ok({ ok: true })),
    tryAcceptWorkspaceRecommendation: async () => (calls.push('accept'), ok({ ok: true })),
    tryRejectWorkspaceRecommendation: async () => (calls.push('reject'), ok({ ok: true })),
    ...over,
  };
  return { a, calls };
}

// #288: accepting a recommendation materializes its artifact, or says why it
// could not — a failed preview must never fall through to a plain accept that
// marks the rec done with nothing written.
describe('decideRecommendation', () => {
  it('a failed preview stops: nothing accepted, the reason returned', async () => {
    const { a, calls } = api({ tryPreviewRecommendation: async () => down });
    expect(await decideRecommendation(a, 'p', 'r', 'accept', () => true)).toEqual({ done: false, error: 'Service Unavailable' });
    expect(calls).toEqual([]);
  });

  it('a file-class rec asks first, then writes it', async () => {
    const { a, calls } = api();
    const ask = vi.fn(() => true);
    expect(await decideRecommendation(a, 'p', 'r', 'accept', ask)).toEqual({ done: true, error: null });
    expect(ask).toHaveBeenCalledWith('Create a skill at .claude/skills/x?\n\nsensei will write a new file into this repo (git-tracked, reversible).');
    expect(calls).toEqual(['preview', 'materialize {}']);
  });

  it('declining the write leaves the rec pending', async () => {
    const { a, calls } = api();
    expect(await decideRecommendation(a, 'p', 'r', 'accept', () => false)).toEqual({ done: false, error: null });
    expect(calls).toEqual(['preview']);
  });

  it('a rule-class rec materializes at workspace scope, recommended', async () => {
    const { a, calls } = api({ tryPreviewRecommendation: async () => ok({ materializable: true }) });
    await decideRecommendation(a, 'p', 'r', 'accept', () => true);
    expect(calls).toEqual(['materialize {"gov_scope":"workspace","enforcement":"recommended"}']);
  });

  it('anything else is a plain accept', async () => {
    const { a, calls } = api({ tryPreviewRecommendation: async () => ok({ materializable: false }) });
    await decideRecommendation(a, 'p', 'r', 'accept', () => true);
    expect(calls).toEqual(['accept']);
  });

  it('a failed write, accept or reject is returned, not swallowed', async () => {
    expect((await decideRecommendation(api({ tryMaterializeRecommendation: async () => down }).a, 'p', 'r', 'accept', () => true)).error).toBe('Service Unavailable');
    expect((await decideRecommendation(api({ tryPreviewRecommendation: async () => ok({ materializable: false }), tryAcceptWorkspaceRecommendation: async () => down }).a, 'p', 'r', 'accept', () => true)).error).toBe('Service Unavailable');
    expect(await decideRecommendation(api({ tryRejectWorkspaceRecommendation: async () => down }).a, 'p', 'r', 'reject', () => true)).toEqual({ done: false, error: 'Service Unavailable' });
  });
});
