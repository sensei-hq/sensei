import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi } from './api.js';

// WP8 (L2): the sync toggle's two daemon calls. A refusal carries the daemon's
// reason (`{ error }`), so a 409 says who may change the repository, not
// "Conflict"; a failed list is an error, never an empty list.

const response = (status: number, statusText: string, body: unknown) =>
  ({ ok: status < 400, status, statusText, json: async () => body }) as unknown as Response;

afterEach(() => vi.unstubAllGlobals());
const api = () => senseiApi(7744);

describe('tryGetDojoRepositories', () => {
  it('asks for one identity’s repositories', async () => {
    const fetch = vi.fn(async () => response(200, 'OK', { repositories: [] }));
    vi.stubGlobal('fetch', fetch);
    expect(await api().tryGetDojoRepositories('slot-1')).toEqual({ ok: true, data: { repositories: [] } });
    const [url, init] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe('http://127.0.0.1:7744/api/dojo/repositories?persona=slot-1');
    expect(init.method).toBe('GET');
  });

  it('a list the dōjō could not give is an error, never []', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => response(502, 'Bad Gateway', { error: 'dōjō returned 500' })));
    const r = await api().tryGetDojoRepositories('slot-1');
    expect(r).toEqual({ ok: false, error: { status: 502, message: 'dōjō returned 500', body: { error: 'dōjō returned 500' } } });
  });
});

describe('tryPatchRepositoryElection', () => {
  it('PATCHes {persona, elected} to the repository, its key one path segment', async () => {
    const fetch = vi.fn(async () => response(200, 'OK', { repo_key: 'github.com/me/notes', elected: false, sync_enabled: false }));
    vi.stubGlobal('fetch', fetch);
    await api().tryPatchRepositoryElection('github.com/me/notes', 'slot-1', false);
    const [url, init] = fetch.mock.calls[0] as unknown as [string, RequestInit];
    expect(url).toBe('http://127.0.0.1:7744/api/repositories/github.com%2Fme%2Fnotes');
    expect(init.method).toBe('PATCH');
    expect(JSON.parse(init.body as string)).toEqual({ persona: 'slot-1', elected: false });
  });

  it('a refusal carries the reason, not the status text', async () => {
    vi.stubGlobal('fetch', vi.fn(async () =>
      response(409, 'Conflict', { error: 'not_yours_to_change', repo_key: 'github.com/acme/web', reason: 'only an admin may change this' })));
    const r = await api().tryPatchRepositoryElection('github.com/acme/web', 'slot-1', false);
    expect(r.ok).toBe(false);
    if (!r.ok && 'body' in r.error) {
      expect(r.error.status).toBe(409);
      expect(r.error.message).toBe('not_yours_to_change');
      expect(r.error.body?.reason).toBe('only an admin may change this');
    } else {
      throw new Error('expected a 409 refusal');
    }
  });
});
