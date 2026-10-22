// #262: the About loader reports a failed read as an error, never as the
// empty list or "no suggestion" a broken daemon would otherwise look like.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const api = {
  tryGetWorkspaceRepos: vi.fn(),
  tryGetDojoSuggestion: vi.fn(),
  tryGetDojoMemberships: vi.fn(),
};
vi.mock('$lib/api.js', () => ({ senseiApi: () => api }));
vi.mock('$lib/appstate.svelte.js', () => ({ appState: { port: 7744 } }));

const { load } = await import('./+page.js');

const ok = <T>(data: T) => ({ ok: true, data });
const fail = (message: string) => ({ ok: false, error: { status: 0, message } });
const run = () =>
  (load as unknown as (e: unknown) => Promise<Record<string, unknown>>)({
    params: { id: 'p1' },
    parent: async () => ({ workspace: { id: 'p1' } }),
  });

beforeEach(() => {
  api.tryGetWorkspaceRepos.mockResolvedValue(ok({ repos: [{ id: 'r1', name: 'api', path: '/x', kind: 'git' }] }));
  api.tryGetDojoSuggestion.mockResolvedValue(ok({ suggestion: null }));
  api.tryGetDojoMemberships.mockResolvedValue(ok([]));
});

describe('About loader', () => {
  it('passes the reads through when they succeed', async () => {
    const data = await run();
    expect(data).toMatchObject({ repos: [{ id: 'r1' }], reposError: null, suggestion: null, memberships: [], bindingError: null });
  });

  it('a failed membership read is a binding error, not an empty list', async () => {
    api.tryGetDojoMemberships.mockResolvedValue(fail('daemon unreachable'));
    expect(await run()).toMatchObject({ bindingError: 'daemon unreachable' });
  });

  it('a failed suggestion read is a binding error, not "no suggestion"', async () => {
    api.tryGetDojoSuggestion.mockResolvedValue(fail('500'));
    expect(await run()).toMatchObject({ bindingError: '500' });
  });

  it('a failed repos read is a repos error, not zero repos', async () => {
    api.tryGetWorkspaceRepos.mockResolvedValue(fail('500'));
    expect(await run()).toMatchObject({ reposError: '500' });
  });
});
