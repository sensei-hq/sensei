// Route tests for POST /v1/you/github/sync (F3c). Uses the session provider_token
// to resolve orgs → provision; best-effort no-op without a token or on a GitHub
// hiccup; a DB error fails closed (500); 401 propagates. Store + GitHub mocked.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveCaller: vi.fn(),
	fetchGithubOrgLogins: vi.fn(),
	syncGithubMemberships: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/github-sync-data', () => ({
	fetchGithubOrgLogins: mocks.fetchGithubOrgLogins,
	syncGithubMemberships: mocks.syncGithubMemberships,
	AdminError
}));

const route = await import('./+server');

function ev(providerToken: string | null) {
	return {
		request: new Request('http://x/', { method: 'POST', headers: { authorization: 'Bearer jwt' } }),
		locals: { session: { provider_token: providerToken } },
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', email: 'a@b.co', db: {} });
	mocks.fetchGithubOrgLogins.mockClear().mockResolvedValue(['acme']);
	mocks.syncGithubMemberships.mockClear().mockResolvedValue({ joined: ['github/acme'] });
});

describe('POST /v1/you/github/sync', () => {
	it('resolves orgs with the provider token and returns the joined dōjōs', async () => {
		const res = await route.POST(ev('gh-token'));
		expect(await res.json()).toEqual({ joined: ['github/acme'], synced: true });
		expect(mocks.fetchGithubOrgLogins.mock.calls[0][0]).toBe('gh-token');
		expect(mocks.syncGithubMemberships.mock.calls[0][1]).toBe('u1');
	});
	it('best-effort no-op when there is no GitHub token (not a GitHub sign-in)', async () => {
		const res = await route.POST(ev(null));
		expect(await res.json()).toMatchObject({ joined: [], synced: false });
		expect(mocks.fetchGithubOrgLogins).not.toHaveBeenCalled();
	});
	it('best-effort no-op (never 500) when the GitHub API is unreachable', async () => {
		mocks.fetchGithubOrgLogins.mockRejectedValueOnce(new AdminError(502, 'gh down'));
		const res = await route.POST(ev('gh-token'));
		expect(res.status).toBe(200);
		expect(await res.json()).toMatchObject({ synced: false });
		expect(mocks.syncGithubMemberships).not.toHaveBeenCalled();
	});
	it('fails closed (500) on a DB error during provisioning', async () => {
		mocks.syncGithubMemberships.mockRejectedValueOnce(new AdminError(500, 'db boom'));
		expect((await route.POST(ev('gh-token'))).status).toBe(500);
	});
	it('propagates a 401 without syncing', async () => {
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		expect((await route.POST(ev('gh-token'))).status).toBe(401);
		expect(mocks.fetchGithubOrgLogins).not.toHaveBeenCalled();
	});
});
