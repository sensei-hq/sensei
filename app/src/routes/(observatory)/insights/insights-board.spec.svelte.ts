import { describe, it, expect, vi } from 'vitest';
import {
  InsightsBoardState,
  RECOMMENDED_VERB,
  EMPTY_INSIGHTS_BOARD,
} from './insights-board.svelte.js';
import type { InsightsBoard } from '$lib/types.js';
import type { SenseiApi } from '$lib/api.js';

// Hand-rolled mock api — only the three methods the state touches are stubbed;
// cast to SenseiApi since the state uses nothing else.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    getInsights: vi.fn().mockResolvedValue(EMPTY_INSIGHTS_BOARD),
    acceptProjectRecommendation: vi.fn().mockResolvedValue({ ok: true }),
    rejectProjectRecommendation: vi.fn().mockResolvedValue({ ok: true }),
    ...overrides,
  } as unknown as SenseiApi;
}

// A board with one item in each source/column so bucketing is observable.
// now:     r1 (rec) · m1 (violated memory) · c1 (correction)   → count 3
// soon:    r2 (rec) · pat1 (pattern)                           → count 2
// settled: m2 (in-force memory)                                → count 1
const fixture = (over: Partial<InsightsBoard> = {}): InsightsBoard => ({
  counts: { now: 3, soon: 2, settled: 1 },
  projects: [{ id: 'p1', name: 'sensei', kanji: '禅' }],
  recommendations: [
    { id: 'r1', urgency: 'high', title: 'Now rec', why: 'w1', impact: 'i1', evidence: [], project_id: 'p1', name: 'sensei', column: 'now' },
    { id: 'r2', urgency: 'medium', title: 'Soon rec', why: '', impact: '', evidence: [], project_id: 'p1', name: 'sensei', column: 'soon' },
  ],
  memories: [
    { id: 'm1', status: 'active', title: 'Violated', content: 'c', violated_count: 3, strength: 0.4, scope: 'project', project_id: 'p1', column: 'now' },
    { id: 'm2', status: 'active', title: 'Settled', content: 'c', violated_count: 0, strength: 1, scope: 'project', project_id: 'p1', column: 'settled' },
  ],
  patterns: [
    { id: 'pat1', name: 'rule-candidates', family: null, lifecycle: 'suggested', instance_count: 8, project_id: 'p1', column: 'soon' },
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
        { id: 'weak', status: 'active', title: 'weak', content: '', violated_count: 0, strength: 0.2, scope: 'project', project_id: 'p1', column: 'settled' },
        { id: 'strong', status: 'active', title: 'strong', content: '', violated_count: 0, strength: 0.9, scope: 'project', project_id: 'p1', column: 'settled' },
      ],
    }));
    expect(s.settled.memories.map((m) => m.id)).toEqual(['strong', 'weak']);
  });

  it('joins the project kanji onto rec view-models', () => {
    const s = new InsightsBoardState(fixture());
    expect(s.now.recs[0].projectKanji).toBe('禅');
    expect(s.now.recs[0].projectName).toBe('sensei');
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
    const accept = vi.fn().mockResolvedValue({ ok: true });
    const api = mockApi({ acceptProjectRecommendation: accept });
    const s = new InsightsBoardState(fixture());
    const rec = s.now.recs[0];

    await s.apply(api, rec);

    expect(accept).toHaveBeenCalledWith('p1', 'r1');
    expect(s.now.recs.map((r) => r.id)).toEqual([]);
    expect(s.now.count).toBe(2);
    expect(s.actionError).toBeNull();
  });

  it('apply() restores the card and surfaces an error when accept fails', async () => {
    const accept = vi.fn().mockResolvedValue({ ok: false });
    const api = mockApi({ acceptProjectRecommendation: accept });
    const s = new InsightsBoardState(fixture());
    const rec = s.now.recs[0];

    await s.apply(api, rec);

    expect(s.now.recs.map((r) => r.id)).toEqual(['r1']);
    expect(s.actionError).not.toBeNull();
    expect(s.actionError).toContain('Now rec');
  });

  it('dismiss() optimistically removes the rec and calls reject', async () => {
    const reject = vi.fn().mockResolvedValue({ ok: true });
    const api = mockApi({ rejectProjectRecommendation: reject });
    const s = new InsightsBoardState(fixture());
    const rec = s.soon.recs[0];

    await s.dismiss(api, rec);

    expect(reject).toHaveBeenCalledWith('p1', 'r2');
    expect(s.soon.recs.map((r) => r.id)).toEqual([]);
  });

  it('dismiss() restores the card and surfaces an error when reject fails', async () => {
    const reject = vi.fn().mockResolvedValue({ ok: false });
    const api = mockApi({ rejectProjectRecommendation: reject });
    const s = new InsightsBoardState(fixture());
    const rec = s.now.recs[0];

    await s.dismiss(api, rec);

    expect(s.now.recs.map((r) => r.id)).toEqual(['r1']);
    expect(s.actionError).not.toBeNull();
  });
});

describe('InsightsBoardState — project filter re-scope', () => {
  it('selectProject(id) refetches scoped and resets optimistic removals', async () => {
    const scoped = fixture({
      counts: { now: 1, soon: 0, settled: 0 },
      recommendations: [
        { id: 'only', urgency: 'high', title: 'Scoped', why: '', impact: '', evidence: [], project_id: 'p1', name: 'sensei', column: 'now' },
      ],
      memories: [],
      patterns: [],
      corrections: [],
    });
    const getInsights = vi.fn().mockResolvedValue(scoped);
    const api = mockApi({ getInsights });
    const s = new InsightsBoardState(fixture());

    await s.selectProject(api, 'p1');

    expect(getInsights).toHaveBeenCalledWith('p1');
    expect(s.selectedProjectId).toBe('p1');
    expect(s.now.recs.map((r) => r.id)).toEqual(['only']);
    expect(s.loading).toBe(false);
  });

  it('selectProject(null) refetches the unscoped board', async () => {
    const getInsights = vi.fn().mockResolvedValue(fixture());
    const api = mockApi({ getInsights });
    const s = new InsightsBoardState(fixture());
    s.selectedProjectId = 'p1';

    await s.selectProject(api, null);

    expect(getInsights).toHaveBeenCalledWith(undefined);
    expect(s.selectedProjectId).toBeNull();
  });

  it('selectProject is a no-op when the scope is unchanged', async () => {
    const getInsights = vi.fn().mockResolvedValue(fixture());
    const api = mockApi({ getInsights });
    const s = new InsightsBoardState(fixture());

    await s.selectProject(api, null);

    expect(getInsights).not.toHaveBeenCalled();
  });
});
