// Route tests for POST /v1/you/forge/refresh.
//
// The wiring, not the redemption logic (that is forge-refresh.spec.ts): that an
// unauthenticated caller cannot spend a refresh token, that a dōjō with no
// client secret says so instead of failing obscurely, and that a terminal
// refusal is reported as one so the daemon knows to stop retrying.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}
class ForgeRefreshError extends AdminError {
	constructor(
		message: string,
		readonly needsSignIn: boolean
	) {
		super(502, message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveCaller: vi.fn(),
	refreshForgeToken: vi.fn(),
	forgeAppFromEnv: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/forge-refresh', () => ({
	refreshForgeToken: mocks.refreshForgeToken,
	ForgeRefreshError
}));
vi.mock('$lib/server/forge-app-env', () => ({ forgeAppFromEnv: mocks.forgeAppFromEnv }));

const route = await import('./+server');

const CREDS = { clientId: 'Ov23xxxx', clientSecret: 's3cret' };
const OK = {
	accessToken: 'gho_new',
	refreshToken: 'ghr_new',
	expiresAt: 1_788_148_800,
	scope: 'read:org,repo,user:email'
};

function post(body: unknown) {
	return route.POST({
		request: new Request('http://x/v1/you/forge/refresh', {
			method: 'POST',
			body: JSON.stringify(body),
			headers: { 'content-type': 'application/json' }
		}),
		locals: {}
	} as never);
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.resolveCaller.mockResolvedValue({ userId: 'p-alice', db: {} });
	mocks.forgeAppFromEnv.mockReturnValue(CREDS);
	mocks.refreshForgeToken.mockResolvedValue(OK);
});

describe('POST /v1/you/forge/refresh', () => {
	it('redeems the token and returns an absolute expiry', async () => {
		const res = await post({ refresh_token: 'ghr_old' });
		expect(res.status).toBe(200);
		expect(mocks.refreshForgeToken).toHaveBeenCalledWith('ghr_old', CREDS);
		expect(await res.json()).toEqual({
			access_token: 'gho_new',
			refresh_token: 'ghr_new',
			expires_at: 1_788_148_800,
			scope: 'read:org,repo,user:email'
		});
	});

	it('requires a caller before spending anything', async () => {
		// `resolveCaller` throws a Response for an unauthenticated request. If that
		// escaped as a 500 the daemon would retry, and each retry redeems — GitHub
		// rotates on redemption, so a retry loop burns the token it is recovering.
		mocks.resolveCaller.mockRejectedValue(
			new Response(JSON.stringify({ error: 'unauthenticated' }), { status: 401 })
		);
		const res = await post({ refresh_token: 'ghr_old' });
		expect(res.status).toBe(401);
		expect(mocks.refreshForgeToken).not.toHaveBeenCalled();
	});

	it('refuses a missing refresh token before calling GitHub', async () => {
		const res = await post({});
		expect(res.status).toBe(400);
		expect(mocks.refreshForgeToken).not.toHaveBeenCalled();
	});

	it('says the dōjō is unconfigured rather than failing at GitHub', async () => {
		// Without the client secret every redemption comes back
		// `incorrect_client_credentials` — which is in the TERMINAL set, so the
		// daemon would conclude the user's grant was revoked and demand a sign-in
		// that cannot help. A deployment fault must not read as a user fault.
		mocks.forgeAppFromEnv.mockReturnValue(null);
		const res = await post({ refresh_token: 'ghr_old' });
		expect(res.status).toBe(503);
		expect((await res.json()).error).toMatch(/not configured/i);
		expect(mocks.refreshForgeToken).not.toHaveBeenCalled();
	});

	it('passes needsSignIn through so the daemon knows to stop retrying', async () => {
		// A revoked grant fails identically forever. Without this flag the
		// scheduled check retries every interval until the user happens to sign
		// in — the poison-pill shape.
		mocks.refreshForgeToken.mockRejectedValue(
			new ForgeRefreshError('GitHub refused the refresh: bad_refresh_token', true)
		);
		const res = await post({ refresh_token: 'ghr_old' });
		expect(res.status).toBe(502);
		const body = await res.json();
		expect(body.needsSignIn).toBe(true);
		expect(body.error).toMatch(/bad_refresh_token/);
	});

	it('marks a transient failure as one', async () => {
		mocks.refreshForgeToken.mockRejectedValue(
			new ForgeRefreshError('GitHub refused the refresh (HTTP 502)', false)
		);
		const res = await post({ refresh_token: 'ghr_old' });
		expect(res.status).toBe(502);
		expect((await res.json()).needsSignIn).toBe(false);
	});
});
