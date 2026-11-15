import { describe, it, expect, vi } from 'vitest';
import { decideRecommendation, RecommendationDecision, type DecideApi } from './overview-view.svelte.js';

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

// The decision's state — moved out of +page.svelte so the page only renders
// (frontend guidelines §2): in flight, why it did not land, and what to say.
describe('RecommendationDecision', () => {
  it('records a decision, reloading only when it landed', async () => {
    const { a } = api();
    const reload = vi.fn(async () => {});
    const d = new RecommendationDecision(a, reload, () => true);
    await d.decide('p', 'r', 'reject');
    expect(reload).toHaveBeenCalledOnce();
    expect([d.deciding, d.error, d.notice]).toEqual([false, null, null]);
  });

  it('a decision that did not land is said, as a fault Notice — and nothing reloads', async () => {
    const { a } = api({ tryRejectWorkspaceRecommendation: async () => down });
    const reload = vi.fn(async () => {});
    const d = new RecommendationDecision(a, reload, () => true);
    await d.decide('p', 'r', 'reject');
    expect(reload).not.toHaveBeenCalled();
    expect(d.notice).toEqual({ tone: 'fault', title: "Couldn't record that decision", detail: 'Service Unavailable', remedy: 'Retry. If it keeps failing, reload the page.' });
  });

  it('a second press while one is in flight is ignored', async () => {
    let release!: () => void;
    const { a, calls } = api({ tryRejectWorkspaceRecommendation: () => new Promise((r) => (release = () => r(ok({ ok: true })))) });
    const d = new RecommendationDecision(a, async () => {}, () => true);
    const first = d.decide('p', 'r', 'reject');
    await d.decide('p', 'r', 'reject');
    expect(d.deciding).toBe(true);
    release();
    await first;
    expect(calls.filter((c) => c === 'reject')).toHaveLength(0); // the stub resolves without recording
    expect(d.deciding).toBe(false);
  });
});
