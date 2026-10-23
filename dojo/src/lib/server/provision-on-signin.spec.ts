// Provisioning at the one moment the forge token exists.
//
// kavach hands us the INCOMING provider session on its server-side sync hook.
// That is the only place `provider_token` is reachable server-side: the cookie
// keeps only access_token/refresh_token, deliberately, so by the next request it
// is gone. Spec §II.7 ("provider_token exists only immediately after the OAuth
// exchange") and §VIII.3.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({
	resolvePrincipalId: vi.fn(),
	provisionWithToken: vi.fn(),
	getUser: vi.fn()
}));

vi.mock('./dojo-supabase', () => ({ dojoDb: () => ({ auth: { getUser: mocks.getUser } }) }));
vi.mock('./principal-resolve', () => ({ resolvePrincipalId: mocks.resolvePrincipalId }));
vi.mock('./provisioning', () => ({ provisionWithToken: mocks.provisionWithToken }));

const { provisionOnSignIn } = await import('./provision-on-signin');

const SESSION = {
	access_token: 'at',
	refresh_token: 'rt',
	provider_token: 'gho_forge_token',
	user: { id: 'u1', email: 'j@example.com' }
};

// What Supabase Auth says the access token is — the login, and the GitHub
// account linked to it. Never the session's own `user` claim.
const VALIDATED = { id: 'u1', email: 'j@example.com', identities: [{ provider: 'github', id: '4242' }] };

beforeEach(() => {
	mocks.getUser.mockReset().mockResolvedValue({ data: { user: VALIDATED }, error: null });
	mocks.resolvePrincipalId.mockClear().mockResolvedValue('p1');
	mocks.provisionWithToken.mockClear().mockResolvedValue({ synced: true, personal: null, tenants: [] });
});

describe('provisionOnSignIn', () => {
	it('provisions with the provider token and the caller PRINCIPAL id', async () => {
		await provisionOnSignIn(SESSION, 'SIGNED_IN');
		// The login id goes to the resolver…
		expect(mocks.resolvePrincipalId.mock.calls[0][1]).toBe('u1');
		// …and the PRINCIPAL id is what provisioning keys on (§VIII.2).
		const [, principalId, token, fallback] = mocks.provisionWithToken.mock.calls[0];
		expect(principalId).toBe('p1');
		expect(token).toBe('gho_forge_token');
		expect(fallback).toEqual({ email: 'j@example.com', githubId: '4242', microsoft: { state: 'none' } });
		expect(mocks.getUser).toHaveBeenCalledWith('at');
	});

	it('keys on the login Supabase validated, not the one the session claims', async () => {
		await provisionOnSignIn({ ...SESSION, user: { id: 'someone-else', email: 'x@example.com' } }, 'SIGNED_IN');
		expect(mocks.resolvePrincipalId.mock.calls[0][1]).toBe('u1');
		expect(mocks.provisionWithToken.mock.calls[0][3]).toEqual({ email: 'j@example.com', githubId: '4242', microsoft: { state: 'none' } });
	});

	it('passes the Microsoft account Supabase linked to the login, by its Entra object id (#308 R5c)', async () => {
		// A Microsoft sign-in: GoTrue recorded the verified ID token's claims as the
		// azure identity's identity_data.custom_claims (oid among them).
		mocks.getUser.mockResolvedValue({
			data: {
				user: {
					id: 'u1',
					email: 'j@example.com',
					identities: [{ provider: 'azure', id: 'pairwise-sub', identity_data: { full_name: 'J Example', custom_claims: { oid: '00000000-0000-4000-8000-0000000000a1' } } }]
				}
			},
			error: null
		});
		await provisionOnSignIn(SESSION, 'SIGNED_IN');
		expect(mocks.provisionWithToken.mock.calls[0][3]).toEqual({
			email: 'j@example.com',
			githubId: null,
			microsoft: { state: 'ok', objectId: '00000000-0000-4000-8000-0000000000a1', name: 'J Example' }
		});
	});

	it('provisions nothing when Supabase will not vouch for the session', async () => {
		mocks.getUser.mockResolvedValue({ data: { user: null }, error: { message: 'invalid JWT' } });
		await expect(provisionOnSignIn(SESSION, 'SIGNED_IN')).rejects.toThrow();
		expect(mocks.provisionWithToken).not.toHaveBeenCalled();
	});

	it('still runs without a provider token, so the personal dōjō is created', async () => {
		// D1 is unconditional. A magic-link user has no forge, and must still get
		// their personal dōjō; provisioning reports `no_forge_token` for the rest.
		const { provider_token: _drop, ...noToken } = SESSION;
		await provisionOnSignIn(noToken, 'SIGNED_IN');
		expect(mocks.provisionWithToken.mock.calls[0][2]).toBeNull();
	});

	it('does nothing when there is no session token to provision for', async () => {
		await provisionOnSignIn({ user: null }, 'SIGNED_IN');
		expect(mocks.resolvePrincipalId).not.toHaveBeenCalled();
		expect(mocks.provisionWithToken).not.toHaveBeenCalled();
	});

	it('does nothing on sign-out', async () => {
		await provisionOnSignIn(null, 'SIGNED_OUT');
		expect(mocks.provisionWithToken).not.toHaveBeenCalled();
	});

	it('ignores a token-refresh event rather than re-reading the forge each hour', async () => {
		// TOKEN_REFRESHED fires on every silent renewal and carries no
		// provider_token, so acting on it would be a pointless GitHub round trip
		// that can only ever report no_forge_token.
		await provisionOnSignIn(SESSION, 'TOKEN_REFRESHED');
		expect(mocks.provisionWithToken).not.toHaveBeenCalled();
	});

	it('lets a failure propagate to kavach, which isolates it from the sign-in', async () => {
		// Deliberately NOT swallowed here: kavach logs and continues, so the
		// session survives, and swallowing twice would hide it from both.
		mocks.provisionWithToken.mockRejectedValue(new Error('db down'));
		await expect(provisionOnSignIn(SESSION, 'SIGNED_IN')).rejects.toThrow('db down');
	});
});
