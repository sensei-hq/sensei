import { describe, it, expect, afterEach, vi } from 'vitest';
import { readFileSync } from 'fs';
import { senseiApi } from './api.js';

/**
 * #304 — the no-fabrication guarantee for WRITES, as #288 gave it to reads.
 *
 * The legacy write helpers turned a failure into something a caller could not
 * tell from success: `post(path, body, fallback)` resolved the caller's
 * fallback, `put`/`del` warned and resolved nothing. Every write is now a
 * try-write: `{ ok: true, data }` when the daemon took it, `{ ok: false, error }`
 * on a non-ok answer or a thrown fetch.
 */

const okResponse = (body: unknown) => new Response(JSON.stringify(body), { status: 200, statusText: 'OK' });
const errResponse = () => new Response('{}', { status: 500, statusText: 'Internal Server Error' });

afterEach(() => { vi.unstubAllGlobals(); });

const api = () => senseiApi(7744);

// Each converted write: (name, invoke, the method it sends, a success body).
const cases: Array<{ name: string; call: () => Promise<unknown>; method: string; body: unknown }> = [
  { name: 'tryRegisterRepo', call: () => api().tryRegisterRepo('r', 'n', '/p'), method: 'POST', body: { ok: true } },
  { name: 'tryUpdateRepo', call: () => api().tryUpdateRepo('r', { name: 'n' }), method: 'PUT', body: {} },
  { name: 'tryDeleteRepo', call: () => api().tryDeleteRepo('r'), method: 'DELETE', body: {} },
  { name: 'tryExcludeRepo', call: () => api().tryExcludeRepo('r'), method: 'POST', body: { ok: true } },
  { name: 'tryAddRepoTag', call: () => api().tryAddRepoTag('r', 't'), method: 'POST', body: { ok: true } },
  { name: 'tryRemoveRepoTag', call: () => api().tryRemoveRepoTag('r', 't'), method: 'DELETE', body: {} },
  { name: 'tryCreateWorkspace', call: () => api().tryCreateWorkspace({ name: 'w' }), method: 'POST', body: { ok: true, id: 'w1' } },
  { name: 'tryUpdateWorkspace', call: () => api().tryUpdateWorkspace('w', { name: 'w' }), method: 'PUT', body: {} },
  { name: 'tryDeleteWorkspace', call: () => api().tryDeleteWorkspace('w'), method: 'DELETE', body: {} },
  { name: 'tryUpdateFolder', call: () => api().tryUpdateFolder('f', { role: 'app' }), method: 'PUT', body: {} },
  { name: 'tryAddWorkspaceRepo', call: () => api().tryAddWorkspaceRepo('w', { repoId: 'r' }), method: 'POST', body: { ok: true } },
  { name: 'tryRemoveWorkspaceRepo', call: () => api().tryRemoveWorkspaceRepo('w', 'r'), method: 'DELETE', body: {} },
  { name: 'tryAnalyzeWorkspace', call: () => api().tryAnalyzeWorkspace('w'), method: 'POST', body: { folders: [] } },
  { name: 'trySetWorkspaceServiceScope', call: () => api().trySetWorkspaceServiceScope('w', 's', true), method: 'PUT', body: {} },
  { name: 'tryIndexRepo', call: () => api().tryIndexRepo('r', '/p'), method: 'POST', body: { ok: true, queued: true, taskId: 1 } },
  { name: 'tryIndexLib', call: () => api().tryIndexLib('lib', 'https://x'), method: 'POST', body: { ok: true } },
  { name: 'tryQuery', call: () => api().tryQuery('q'), method: 'POST', body: { results: [] } },
  { name: 'tryAddWatchRoot', call: () => api().tryAddWatchRoot('/p'), method: 'POST', body: { ok: true, id: 'r1', path: '/p' } },
  { name: 'tryScanFolder', call: () => api().tryScanFolder('/p'), method: 'POST', body: { ok: true, scanning: true } },
  { name: 'tryMarketplaceInstall', call: () => api().tryMarketplaceInstall('claude', '/m'), method: 'POST', body: { ok: true } },
  { name: 'trySetConfig', call: () => api().trySetConfig({ k: 'v' }), method: 'PUT', body: {} },
  { name: 'tryDeleteConfig', call: () => api().tryDeleteConfig('k'), method: 'DELETE', body: {} },
  { name: 'tryConfigureAssistants', call: () => api().tryConfigureAssistants(['claude-code']), method: 'POST', body: { configured: [], skipped: [], errors: [] } },
  { name: 'tryRemoveAssistants', call: () => api().tryRemoveAssistants(['claude-code']), method: 'POST', body: { assistants_removed: [], errors: [] } },
  { name: 'tryInstallAll', call: () => api().tryInstallAll(['claude-code']), method: 'POST', body: { hooks_installed: 1, errors: [] } },
  { name: 'tryInstallHooks', call: () => api().tryInstallHooks(), method: 'POST', body: { ok: true, count: 3 } },
  { name: 'tryInstallItem', call: () => api().tryInstallItem('n', 'skill'), method: 'POST', body: { ok: true } },
  { name: 'tryRemoveItem', call: () => api().tryRemoveItem('n', 'skill'), method: 'POST', body: { ok: true } },
  { name: 'tryRemoveAll', call: () => api().tryRemoveAll(), method: 'POST', body: { assistants_removed: [], errors: [] } },
  { name: 'tryGenerateImage', call: () => api().tryGenerateImage({ prompt: 'p' }), method: 'POST', body: { ok: true, paths: ['/i.png'] } },
  { name: 'tryStop', call: () => api().tryStop(), method: 'POST', body: { ok: true } },
];

describe('#304 · every write is the daemon\'s answer or an ApiResult error', () => {
  for (const c of cases) {
    describe(c.name, () => {
      it(`sends ${c.method} and resolves { ok: true } when the daemon takes it`, async () => {
        const fetch = vi.fn(async () => okResponse(c.body));
        vi.stubGlobal('fetch', fetch);
        const res = await c.call() as { ok: boolean };
        expect(res.ok).toBe(true);
        expect((fetch.mock.calls[0] as unknown as [string, RequestInit])[1]?.method).toBe(c.method);
      });

      it('resolves { ok: false, error } on a non-ok answer — not a fallback', async () => {
        vi.stubGlobal('fetch', vi.fn(async () => errResponse()));
        const res = await c.call() as { ok: boolean; error?: { status: number } };
        expect([res.ok, res.error?.status]).toEqual([false, 500]);
      });

      it('resolves { ok: false, error } on a network throw', async () => {
        vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('ECONNREFUSED')));
        const res = await c.call() as { ok: boolean; error?: { status: number } };
        expect([res.ok, res.error?.status]).toEqual([false, 0]);
      });
    });
  }
});

// The legacy write helpers are gone and must not come back: a write that can
// fail returns a try- helper's ApiResult.
describe('no fallback writes (#304)', () => {
  it('api.ts has no post/put/del helper, nor a write through one', () => {
    const src = readFileSync('src/lib/api.ts', 'utf8');
    expect(src).not.toMatch(/function (post|put|del)\b/);
    expect(src.split('\n').filter((l) => /(^|[^\w.])(post|put|del)(<[^>]*>)?\(/.test(l))).toEqual([]);
  });
});
