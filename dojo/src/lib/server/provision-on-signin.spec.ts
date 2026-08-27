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
	provisionWithToken: vi.fn()
}));

vi.mock('./dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('./principal-resolve', () => ({ resolvePrincipalId: mocks.resolvePrincipalId }));
vi.mock('./provisioning', () => ({ provisionWithToken: mocks.provisionWithToken }));

const { provisionOnSignIn } = await import('./provision-on-signin');

const SESSION = {
	access_token: 'at',
	refresh_token: 'rt',
	provider_token: 'gho_forge_token',
	user: { id: 'u1', email: 'j@example.com' }
};

beforeEach(() => {
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
		expect(fallback).toEqual({ email: 'j@example.com' });
	});

	it('still runs without a provider token, so the personal dōjō is created', async () => {
		// D1 is unconditional. A magic-link user has no forge, and must still get
		// their personal dōjō; provisioning reports `no_forge_token` for the rest.
		const { provider_token: _drop, ...noToken } = SESSION;
		await provisionOnSignIn(noToken, 'SIGNED_IN');
		expect(mocks.provisionWithToken.mock.calls[0][2]).toBeNull();
	});

	it('does nothing when there is no user id to provision for', async () => {
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
