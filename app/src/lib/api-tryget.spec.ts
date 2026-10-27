import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi, failureReason, readOutcome, dataOrThrow } from './api.js';

/**
 * F8 (mockup-drift-audit) — the no-fabrication guarantee at the API seam.
 *
 * The loaders that used to call `get(path, fallback)` (which resolves the
 * fallback on ANY failure, making a fetch error indistinguishable from an
 * honest-empty result) now call the error-propagating `tryGet*` variants. These
 * tests pin that: on a non-ok response OR a thrown fetch, the method resolves
 * `{ ok: false, error }` — NOT a fabricated success. On ok, it resolves
 * `{ ok: true, data }`. Reverting any of these to a fallback-returning `get`
 * would make the failure assertions fail.
 */

const okResponse = (body: unknown) => ({
  ok: true,
  status: 200,
  statusText: 'OK',
  json: async () => body,
}) as unknown as Response;

const errResponse = (status = 500, statusText = 'Internal Server Error') => ({
  ok: false,
  status,
  statusText,
  json: async () => ({}),
}) as unknown as Response;

afterEach(() => { vi.unstubAllGlobals(); });

const api = () => senseiApi(7744);

// Each converted method: (name, invoke, a success body).
const cases: Array<{ name: string; call: () => Promise<unknown>; body: unknown }> = [
  { name: 'tryGetConsolidatedRuleset', call: () => api().tryGetConsolidatedRuleset(), body: null },
  { name: 'tryGetShareReviewBatch', call: () => api().tryGetShareReviewBatch(), body: { batch: null } },
  { name: 'tryGetCollectivePreferences', call: () => api().tryGetCollectivePreferences(), body: { destination: 'none' } },
  { name: 'tryGetWorkspaceLibraries', call: () => api().tryGetWorkspaceLibraries('p1'), body: { libraries: [] } },
  { name: 'tryGetWorkspaceLibraryVersionConflicts', call: () => api().tryGetWorkspaceLibraryVersionConflicts('p1'), body: { conflicts: [] } },
  // #288 — Observatory · Today.
  { name: 'tryGetObservatoryToday', call: () => api().tryGetObservatoryToday(), body: { dataMaturity: 'mature' } },
  { name: 'tryGetObservatoryFtr', call: () => api().tryGetObservatoryFtr(), body: { ftr14d: 0.8 } },
  // #288 — workspaces index and workspace layout.
  { name: 'tryGetWorkspaceFtr', call: () => api().tryGetWorkspaceFtr('p1'), body: { ftr14d: 0.8 } },
  { name: 'tryGetWorkspaceQualitySignals', call: () => api().tryGetWorkspaceQualitySignals('p1'), body: { ftr_7d: 0.9 } },
  { name: 'tryListWorkspaces', call: () => api().tryListWorkspaces(), body: [] },
  // #288 — Impact and Traceability.
  { name: 'tryGetWorkspaceImpact', call: () => api().tryGetWorkspaceImpact('p1'), body: [] },
  { name: 'tryListImpactVerdicts', call: () => api().tryListImpactVerdicts('p1'), body: { verdicts: [] } },
  { name: 'tryGetWorkspaceDrift', call: () => api().tryGetWorkspaceDrift('p1'), body: { items: [], total: 0 } },
  { name: 'tryScanWorkspaceDrift', call: () => api().tryScanWorkspaceDrift('p1'), body: { scannedDocs: 2 } },
  // #288 — Upgrades and the recommendation decision.
  { name: 'tryGetUpgrades', call: () => api().tryGetUpgrades(), body: { artifacts: [], unread_count: 0 } },
  { name: 'tryPreviewRecommendation', call: () => api().tryPreviewRecommendation('p1', 'r1'), body: { materializable: true } },
  { name: 'tryMaterializeRecommendation', call: () => api().tryMaterializeRecommendation('p1', 'r1', {}), body: { ok: true } },
  { name: 'tryAcceptWorkspaceRecommendation', call: () => api().tryAcceptWorkspaceRecommendation('p1', 'r1'), body: { ok: true } },
  { name: 'tryRejectWorkspaceRecommendation', call: () => api().tryRejectWorkspaceRecommendation('p1', 'r1'), body: { ok: true } },
  // #288 — Insights.
  { name: 'tryGetInsights', call: () => api().tryGetInsights('p1'), body: { counts: { now: 0 } } },
  // #288 — Sessions.
  { name: 'tryGetSessionsDigest', call: () => api().tryGetSessionsDigest('7d', 'p1'), body: { sessions: [] } },
  { name: 'tryGetSessions', call: () => api().tryGetSessions(), body: { stats: null, sessions: [] } },
  // #288 — workspace Instruments and Memories.
  { name: 'tryGetWorkspaceInstruments', call: () => api().tryGetWorkspaceInstruments('p1'), body: { tools: [] } },
  { name: 'tryGetWorkspaceMcpToolStats', call: () => api().tryGetWorkspaceMcpToolStats('p1'), body: { tools: [] } },
  { name: 'tryListWorkspaceServices', call: () => api().tryListWorkspaceServices('p1'), body: { services: [] } },
  { name: 'tryGetWorkspaceMemories', call: () => api().tryGetWorkspaceMemories('p1'), body: { active: [], total: 0 } },
  { name: 'tryListMemoryShareBatches', call: () => api().tryListMemoryShareBatches('p1', 'proposed'), body: { batches: [] } },
  // #288 — Libraries.
  { name: 'tryGetLibs', call: () => api().tryGetLibs({}), body: { total: 0, libs: [] } },
  { name: 'tryGetLibraryUsage', call: () => api().tryGetLibraryUsage('l1'), body: { usage: [] } },
  // #288 — Settings.
  { name: 'tryDetectAssistants', call: () => api().tryDetectAssistants(), body: [] },
  { name: 'tryGetInstalledItems', call: () => api().tryGetInstalledItems(), body: [] },
  { name: 'tryListGatewayChains', call: () => api().tryListGatewayChains(), body: { chains: [] } },
  { name: 'tryProvisionStatus', call: () => api().tryProvisionStatus(), body: { models: [] } },
  // #288 — the Instruments store.
  { name: 'tryGetToolUsage', call: () => api().tryGetToolUsage(), body: { tools: [] } },
  { name: 'tryGetToolSignals', call: () => api().tryGetToolSignals(), body: { signals: [] } },
  { name: 'tryGetToolInsights', call: () => api().tryGetToolInsights(), body: { insights: [] } },
  { name: 'tryGetToolsHealth', call: () => api().tryGetToolsHealth(), body: { sources: [] } },
  { name: 'tryGetSessionReplay', call: () => api().tryGetSessionReplay('s1'), body: { calls: [] } },
  { name: 'tryGetMcpServerTools', call: () => api().tryGetMcpServerTools('srv'), body: { tools: [] } },
  // #288 — the setup wizard.
  { name: 'tryDetectAssistantFamilies', call: () => api().tryDetectAssistantFamilies(), body: [] },
  { name: 'tryListInstruments', call: () => api().tryListInstruments(), body: { total: 0, mcps: [], stack: [] } },
  // #288 — scan state.
  { name: 'tryGetIndexStatus', call: () => api().tryGetIndexStatus(), body: { queue: { pending: 0 } } },
  { name: 'tryGetScanRoots', call: () => api().tryGetScanRoots(), body: [] },
  // #288 — the last five.
  { name: 'tryGetHealth', call: () => api().tryGetHealth(), body: { daemonDbMode: 'full' } },
  { name: 'tryGetWorkspaceTools', call: () => api().tryGetWorkspaceTools('p1'), body: { tools: [], count: 0 } },
  { name: 'tryGetIntakeGuide', call: () => api().tryGetIntakeGuide(), body: { frame: '', axes: [], playbooks: [] } },
  { name: 'tryListMemories', call: () => api().tryListMemories({ status: 'active' }), body: { memories: [] } },
  { name: 'tryListKnowledgeSources', call: () => api().tryListKnowledgeSources(), body: { sources: [] } },
  // #288 — Atlas.
  { name: 'tryGetGraphNodes', call: () => api().tryGetGraphNodes('r'), body: { nodes: [], edges: [] } },
  { name: 'tryGetCommunities', call: () => api().tryGetCommunities('r'), body: [] },
  { name: 'tryDetectCommunities', call: () => api().tryDetectCommunities('r'), body: { ok: true, communities: 2 } },
  { name: 'tryGetCallFlow', call: () => api().tryGetCallFlow('r'), body: { moduleCount: 1 } },
  { name: 'tryGetWorkspaceGraph', call: () => api().tryGetWorkspaceGraph('p1'), body: { repos: 1 } },
];

describe('F8 · error-propagating loaders (no fabrication on failure)', () => {
  for (const c of cases) {
    describe(c.name, () => {
      it('resolves { ok: true, data } on a successful fetch', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okResponse(c.body)));
        const res = await c.call() as { ok: boolean; data?: unknown };
        expect(res.ok).toBe(true);
        expect(res.data).toEqual(c.body);
      });

      it('resolves { ok: false, error } on a non-ok response (NOT a fabricated fallback)', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(errResponse(500)));
        const res = await c.call() as { ok: boolean; error?: { status: number; message: string } };
        expect(res.ok).toBe(false);
        expect(res.error?.status).toBe(500);
      });

      it('resolves { ok: false, error } on a network throw', async () => {
        vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('ECONNREFUSED')));
        const res = await c.call() as { ok: boolean; error?: { status: number; message: string } };
        expect(res.ok).toBe(false);
        expect(res.error?.status).toBe(0);
      });
    });
  }
});

describe('failureReason / readOutcome (#288)', () => {
  it('names the failure: the status text, else the status', () => {
    expect(failureReason({ status: 503, message: 'Service Unavailable' })).toBe('Service Unavailable');
    expect(failureReason({ status: 502, message: '' })).toBe('HTTP 502');
    expect(failureReason({ status: 0, message: 'ECONNREFUSED' })).toBe('ECONNREFUSED');
  });

  it('a read is its data, or why it could not be read — never both, never a default', () => {
    expect(readOutcome({ ok: true, data: [] })).toEqual({ data: [], error: null });
    expect(readOutcome({ ok: false, error: { status: 500, message: 'boom' } })).toEqual({ data: null, error: 'boom' });
  });
});

// #288: every read is the daemon's answer or an ApiResult error. The legacy
// `get(path, fallback)` — which turned any failure into a default — is gone,
// and must not come back: a read that can fail returns `tryGet`'s result.
describe('no fallback reads (#288)', () => {
  it('api.ts has no get(path, fallback) — neither the helper nor a read through it', async () => {
    const { readFileSync } = await import('fs');
    const src = readFileSync('src/lib/api.ts', 'utf8');
    expect(src).not.toMatch(/function get</);
    expect(src.split('\n').filter((l) => /\bget</.test(l))).toEqual([]);
  });
});

describe('dataOrThrow (#288)', () => {
  it('is the data of a read that landed', () => {
    expect(dataOrThrow({ ok: true, data: [1] }, 'roots')).toEqual([1]);
  });
  it('throws a read that failed, naming what could not be read', () => {
    expect(() => dataOrThrow({ ok: false, error: { status: 503, message: 'Service Unavailable' } }, 'roots'))
      .toThrow('reading the roots failed: Service Unavailable');
  });
});

// #296: with no persona named, the daemon resolves the DEFAULT identity (the
// one owning the global git email). Sending the literal `default` slot would
// address whoever first signed in without naming one.
describe('auth requests leave the default to the daemon (#296)', () => {
  const urls = async (call: () => Promise<unknown>) => {
    const seen: string[] = [];
    vi.stubGlobal('fetch', vi.fn(async (u: string) => { seen.push(String(u)); return okResponse({}); }));
    await call();
    return seen;
  };
  it('a status with no persona names none', async () => {
    const [u] = await urls(() => api().tryGetAuthStatus());
    expect(u).toMatch(/\/api\/auth\/status$/);
  });
  it('a sign-in with no persona names none', async () => {
    const [u] = await urls(() => api().tryStartSignIn());
    expect(u).toMatch(/\/api\/auth\/signin$/);
  });
  it('a named persona is passed through, with its expected login', async () => {
    const [u] = await urls(() => api().tryStartSignIn('jerrythomas', 'jerrythomas'));
    expect(u).toMatch(/\/api\/auth\/signin\?persona=jerrythomas&github_login=jerrythomas$/);
  });
});
