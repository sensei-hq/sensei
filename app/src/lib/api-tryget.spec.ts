import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi } from './api.js';

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
