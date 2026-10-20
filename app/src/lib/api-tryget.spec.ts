import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi, failureReason, readOutcome } from './api.js';

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
  { name: 'tryGetProjectLibraries', call: () => api().tryGetProjectLibraries('p1'), body: { libraries: [] } },
  { name: 'tryGetProjectLibraryVersionConflicts', call: () => api().tryGetProjectLibraryVersionConflicts('p1'), body: { conflicts: [] } },
  // #288 — Observatory · Today.
  { name: 'tryGetObservatoryToday', call: () => api().tryGetObservatoryToday(), body: { dataMaturity: 'mature' } },
  { name: 'tryGetObservatoryFtr', call: () => api().tryGetObservatoryFtr(), body: { ftr14d: 0.8 } },
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

// #288: reads that turn a failure into a default are being moved to `tryGet`,
// screen by screen. A ratchet: the count only goes down, and at 0 the legacy
// `get` itself goes. Lower it as screens convert; never raise it.
describe('legacy fallback reads (#288)', () => {
  it('no new get<T>(path, fallback) read is added', async () => {
    const { readFileSync } = await import('fs');
    const src = readFileSync('src/lib/api.ts', 'utf8');
    const reads = src.split('\n').filter((l) => /\bget</.test(l) && !l.includes('async function get<'));
    expect(reads.length).toBeLessThanOrEqual(46);
  });
});
