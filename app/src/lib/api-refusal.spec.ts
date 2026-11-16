import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi } from './api.js';

/**
 * #451 — every helper in the app's API client reports the daemon's reason for a
 * refusal, not its code: `message`, else `error`, else the status text, with the
 * parsed body kept for callers that map the code (the tracker forms).
 */

afterEach(() => { vi.unstubAllGlobals(); });

const api = () => senseiApi(7744);

// One method per helper: tryGet, tryPost, tryPut, tryDelete, tryDeleteJson, tryJson.
const helpers: Array<{ helper: string; call: () => Promise<unknown> }> = [
  { helper: 'tryGet', call: () => api().tryGetWorkspaceLibraries('p1') },
  { helper: 'tryPost', call: () => api().tryRegisterRepo('r', 'n', '/p') },
  { helper: 'tryPut', call: () => api().tryUpdateRepo('r', { name: 'n' }) },
  { helper: 'tryDelete', call: () => api().tryDeleteRepo('r') },
  { helper: 'tryDeleteJson', call: () => api().tryRemoveWatchRoot('root-1', 'keep') },
  { helper: 'tryJson', call: () => api().tryListTrackers('w1') },
];

type Refused = { ok: false; error: { status: number; message: string; body?: Record<string, unknown> } };

describe('#451 · a refusal carries the daemon’s reason', () => {
  for (const h of helpers) {
    describe(h.helper, () => {
      it('a coded refusal with a reason: the person reads the reason; the code stays in the body', async () => {
        const body = { error: 'conflict', message: 'the board is connected to another project' };
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status: 409, statusText: 'Conflict' })));
        const res = (await h.call()) as Refused;
        expect(res.ok).toBe(false);
        expect(res.error).toEqual({ status: 409, message: 'the board is connected to another project', body });
      });

      it('a reason in error alone: the person reads it', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: 'name is required' }), { status: 400, statusText: 'Bad Request' })));
        const res = (await h.call()) as Refused;
        expect(res.error.message).toBe('name is required');
      });

      it('no JSON body: the status text', async () => {
        vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('down', { status: 503, statusText: 'Service Unavailable' })));
        const res = (await h.call()) as Refused;
        expect(res.error).toEqual({ status: 503, message: 'Service Unavailable' });
      });
    });
  }
});
