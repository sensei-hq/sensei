import { describe, it, expect, vi } from 'vitest';
import {
  InsightsBoardState,
  RECOMMENDED_VERB,
  EMPTY_INSIGHTS_BOARD,
} from './insights-board.svelte.js';
import type { InsightsBoard } from '$lib/types.js';
import type { SenseiApi } from '$lib/api.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

// Hand-rolled mock api — only the three methods the state touches are stubbed;
// cast to SenseiApi since the state uses nothing else.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    tryGetInsights: vi.fn().mockResolvedValue(ok(EMPTY_INSIGHTS_BOARD)),
    tryAcceptWorkspaceRecommendation: vi.fn().mockResolvedValue(ok({ ok: true })),
    tryRejectWorkspaceRecommendation: vi.fn().mockResolvedValue(ok({ ok: true })),
    ...overrides,
  } as unknown as SenseiApi;
}

// A board with one item in each source/column so bucketing is observable.
// now:     r1 (rec) · m1 (violated memory) · c1 (correction)   → count 3
// soon:    r2 (rec) · pat1 (pattern)                           → count 2
// settled: m2 (in-force memory)                                → count 1
const fixture = (over: Partial<InsightsBoard> = {}): InsightsBoard => ({
  counts: { now: 3, soon: 2, settled: 1 },
  workspaces: [{ id: 'p1', name: 'sensei', kanji: '禅' }],
  recommendations: [
    { id: 'r1', urgency: 'high', title: 'Now rec', why: 'w1', impact: 'i1', evidence: [], workspace_id: 'p1', name: 'sensei', column: 'now' },
    { id: 'r2', urgency: 'medium', title: 'Soon rec', why: '', impact: '', evidence: [], workspace_id: 'p1', name: 'sensei', column: 'soon' },
  ],
  memories: [
    { id: 'm1', status: 'active', title: 'Violated', content: 'c', violated_count: 3, strength: 2, scope: 'workspace', workspace_id: 'p1', column: 'now' },
    { id: 'm2', status: 'active', title: 'Settled', content: 'c', violated_count: 0, strength: 1, scope: 'workspace', workspace_id: 'p1', column: 'settled' },
  ],
  patterns: [
    { id: 'pat1', name: 'rule-candidates', family: null, lifecycle: 'suggested', instance_count: 8, workspace_id: 'p1', column: 'soon' },
  ],
  corrections: [
    { id: 'c1', text: 'oops', suggestion: null, count: 3, column: 'now' },
  ],
  ...over,
});

describe('InsightsBoardState — column bucketing', () => {
  it('buckets Now by the server column across sources', () => {
    const s = new InsightsBoardState(fixture());
    expect(s.now.recs.map((r) => r.id)).toEqual(['r1']);
    expect(s.now.violations.map((m) => m.id)).toEqual(['m1']);
    expect(s.now.corrections.map((c) => c.id)).toEqual(['c1']);
    expect(s.now.count).toBe(3);
  });

  it('buckets Soon into recs, patterns and memories', () => {
    const s = new InsightsBoardState(fixture());
    expect(s.soon.recs.map((r) => r.id)).toEqual(['r2']);
    expect(s.soon.patterns.map((p) => p.id)).toEqual(['pat1']);
    expect(s.soon.count).toBe(2);
  });

  it('buckets Settled memories (strength desc) and exposes honest counts', () => {
    const s = new InsightsBoardState(fixture());
    expect(s.settled.memories.map((m) => m.id)).toEqual(['m2']);
    expect(s.settled.count).toBe(1);
    expect(s.counts).toEqual({ now: 3, soon: 2, settled: 1 });
  });

  it('sorts Settled memories by strength descending', () => {
    const s = new InsightsBoardState(fixture({
      memories: [
        { id: 'weak', status: 'active', title: 'weak', content: '', violated_count: 0, strength: 1.5, scope: 'workspace', workspace_id: 'p1', column: 'settled' },
        { id: 'strong', status: 'active', title: 'strong', content: '', violated_count: 0, strength: 4.5, scope: 'workspace', workspace_id: 'p1', column: 'settled' },
      ],
    }));
    expect(s.settled.memories.map((m) => m.id)).toEqual(['strong', 'weak']);
  });

  it('shows a memory\'s strength on the daemon\'s 1–5 scale, not clamped to a full bar', () => {
    // Audit 2026-10-28, item 1: clamping 1–5 to 0–1 drew every memory full.
    const s = new InsightsBoardState(fixture());
    expect(s.settled.memories[0].strength).toMatchObject({ label: '1.0 / 5', width: '20%', full: false });
  });

  it('joins the workspace kanji onto rec view-models', () => {
    const s = new InsightsBoardState(fixture());
    expect(s.now.recs[0].workspaceKanji).toBe('禅');
    expect(s.now.recs[0].workspaceName).toBe('sensei');
  });

  it('reports isEmpty only when every column is empty', () => {
    expect(new InsightsBoardState(fixture()).isEmpty).toBe(false);
    expect(new InsightsBoardState().isEmpty).toBe(true);
  });
});

describe('InsightsBoardState — one decision, one default', () => {
  it('Apply is the recommended verb', () => {
    expect(RECOMMENDED_VERB).toBe('Apply');
  });
});

describe('InsightsBoardState — actions', () => {
  it('apply() optimistically removes the rec and calls accept (schedules MeasureVerdicts)', async () => {
    const accept = vi.fn().mockResolvedValue(ok({ ok: true }));
    const api = mockApi({ tryAcceptWorkspaceRecommendation: accept });
    const s = new InsightsBoardState(fixture());
    const rec = s.now.recs[0];

    await s.apply(api, rec);

    expect(accept).toHaveBeenCalledWith('p1', 'r1');
    expect(s.now.recs.map((r) => r.id)).toEqual([]);
    expect(s.now.count).toBe(2);
    expect(s.actionError).toBeNull();
  });

  it('apply() restores the card and surfaces an error when accept fails', async () => {
    const accept = vi.fn().mockResolvedValue(down);
    const api = mockApi({ tryAcceptWorkspaceRecommendation: accept });
    const s = new InsightsBoardState(fixture());
    const rec = s.now.recs[0];

    await s.apply(api, rec);

    expect(s.now.recs.map((r) => r.id)).toEqual(['r1']);
    expect(s.actionError).not.toBeNull();
    expect(s.actionError).toContain('Now rec');
    // The daemon's reason, not a guess that it "may already be decided" (#288).
    expect(s.actionError).toContain('Service Unavailable');
  });

  it('dismiss() optimistically removes the rec and calls reject', async () => {
    const reject = vi.fn().mockResolvedValue(ok({ ok: true }));
    const api = mockApi({ tryRejectWorkspaceRecommendation: reject });
    const s = new InsightsBoardState(fixture());
    const rec = s.soon.recs[0];

    await s.dismiss(api, rec);

    expect(reject).toHaveBeenCalledWith('p1', 'r2');
    expect(s.soon.recs.map((r) => r.id)).toEqual([]);
  });

  it('dismiss() restores the card and surfaces an error when reject fails', async () => {
    const reject = vi.fn().mockResolvedValue(down);
    const api = mockApi({ tryRejectWorkspaceRecommendation: reject });
    const s = new InsightsBoardState(fixture());
    const rec = s.now.recs[0];

    await s.dismiss(api, rec);

    expect(s.now.recs.map((r) => r.id)).toEqual(['r1']);
    expect(s.actionError).not.toBeNull();
  });
});

describe('InsightsBoardState — workspace filter re-scope', () => {
  it('selectWorkspace(id) refetches scoped and resets optimistic removals', async () => {
    const scoped = fixture({
      counts: { now: 1, soon: 0, settled: 0 },
      recommendations: [
        { id: 'only', urgency: 'high', title: 'Scoped', why: '', impact: '', evidence: [], workspace_id: 'p1', name: 'sensei', column: 'now' },
      ],
      memories: [],
      patterns: [],
      corrections: [],
    });
    const getInsights = vi.fn().mockResolvedValue(ok(scoped));
    const api = mockApi({ tryGetInsights: getInsights });
    const s = new InsightsBoardState(fixture());

    await s.selectWorkspace(api, 'p1');

    expect(getInsights).toHaveBeenCalledWith('p1');
    expect(s.selectedWorkspaceId).toBe('p1');
    expect(s.now.recs.map((r) => r.id)).toEqual(['only']);
    expect(s.loading).toBe(false);
  });

  it('selectWorkspace(null) refetches the unscoped board', async () => {
    const getInsights = vi.fn().mockResolvedValue(ok(fixture()));
    const api = mockApi({ tryGetInsights: getInsights });
    const s = new InsightsBoardState(fixture());
    s.selectedWorkspaceId = 'p1';

    await s.selectWorkspace(api, null);

    expect(getInsights).toHaveBeenCalledWith(undefined);
    expect(s.selectedWorkspaceId).toBeNull();
  });

  it('selectWorkspace is a no-op when the scope is unchanged', async () => {
    const getInsights = vi.fn().mockResolvedValue(ok(fixture()));
    const api = mockApi({ tryGetInsights: getInsights });
    const s = new InsightsBoardState(fixture());

    await s.selectWorkspace(api, null);

    expect(getInsights).not.toHaveBeenCalled();
  });
});

// #288: a board that could not be read is an error — not the quiet board.
describe('InsightsBoardState — a failed read', () => {
  it('starts in error when the first load failed', () => {
    const s = new InsightsBoardState(null, 'Service Unavailable');
    expect([s.loadError, s.isEmpty]).toEqual(['Service Unavailable', true]);
  });

  it('a failed re-scope is an error, and drops the previous scope\'s board', async () => {
    const s = new InsightsBoardState(fixture());
    await s.selectWorkspace(mockApi({ tryGetInsights: vi.fn().mockResolvedValue(down) }), 'p1');
    expect([s.loadError, s.now.count, s.loading]).toEqual(['Service Unavailable', 0, false]);
  });

  it('a successful re-scope clears the error', async () => {
    const s = new InsightsBoardState(null, 'Service Unavailable');
    await s.selectWorkspace(mockApi({ tryGetInsights: vi.fn().mockResolvedValue(ok(fixture())) }), 'p1');
    expect(s.loadError).toBeNull();
    expect(s.now.count).toBeGreaterThan(0);
  });

  it('retry() re-reads the current scope', async () => {
    const getInsights = vi.fn().mockResolvedValue(ok(fixture()));
    const s = new InsightsBoardState(null, 'Service Unavailable');
    s.selectedWorkspaceId = 'p1';
    await s.retry(mockApi({ tryGetInsights: getInsights }));
    expect(getInsights).toHaveBeenCalledWith('p1');
    expect(s.loadError).toBeNull();
  });
});
