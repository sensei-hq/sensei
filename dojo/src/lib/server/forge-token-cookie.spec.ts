// The GitHub token a "Connect GitHub" re-authorization yields (#308 R5a): it
// arrives once, in the session the browser POSTs to /auth/session, and the
// Integrations reads, claim and map need it on LATER requests. It is never
// stored in the database: it rides back in a cookie that is sealed (AES-GCM, a
// key derived from the server secret), bound to the signed-in login, HttpOnly,
// scoped to /v1/t, and gone after an hour. Anything off — tampered, expired,
// another login's, unreadable — is "not connected", never a guess.
import { describe, it, expect } from 'vitest';
import {
	FORGE_TOKEN_COOKIE,
	FORGE_TOKEN_TTL_SECONDS,
	sealForgeToken,
	openForgeToken,
	forgeTokenCookie,
	clearForgeTokenCookie,
	captureForgeToken
} from './forge-token-cookie';
import { fakeFetch } from './fake-forge-fetch';

const SECRET = 'server-secret-for-tests';
const ALICE = 'auth-alice';
const NOW = 1_800_000_000_000;

describe('sealing the token', () => {
	it('opens to the same token for the same login, within its hour', async () => {
		const sealed = await sealForgeToken('gho_abc', ALICE, SECRET, NOW);
		expect(sealed).not.toContain('gho_abc');
		expect(await openForgeToken(sealed, ALICE, SECRET, NOW + 59 * 60_000)).toBe('gho_abc');
	});

	it('is not the token for another login, after its hour, under another secret, or tampered with', async () => {
		const sealed = await sealForgeToken('gho_abc', ALICE, SECRET, NOW);
		expect(await openForgeToken(sealed, 'auth-bob', SECRET, NOW)).toBeNull();
		expect(await openForgeToken(sealed, ALICE, SECRET, NOW + FORGE_TOKEN_TTL_SECONDS * 1000 + 1)).toBeNull();
		expect(await openForgeToken(sealed, ALICE, 'another-secret', NOW)).toBeNull();
		// Tamper for certain: a middle character (all data bits, unlike the last
		// base64url one) replaced by a DIFFERENT one. Choosing the replacement from
		// the wrong position left the cookie unchanged once in ~64 seals (random IV).
		const at = Math.floor(sealed.length / 2);
		const flipped = sealed.slice(0, at) + (sealed[at] === 'A' ? 'B' : 'A') + sealed.slice(at + 1);
		expect(flipped).not.toBe(sealed);
		expect(await openForgeToken(flipped, ALICE, SECRET, NOW)).toBeNull();
		for (const junk of [undefined, '', 'not-base64!', 'AAAA']) {
			expect(await openForgeToken(junk, ALICE, SECRET, NOW)).toBeNull();
		}
	});

	it('refuses to seal with no secret — never an unsealed token', async () => {
		await expect(sealForgeToken('gho_abc', ALICE, '', NOW)).rejects.toThrow();
	});
});

describe('the cookie', () => {
	it('is HttpOnly, SameSite=Lax, scoped to /v1/t, and lives an hour', () => {
		const c = forgeTokenCookie('SEALED', true);
		expect(c).toBe(`${FORGE_TOKEN_COOKIE}=SEALED; Path=/v1/t; Max-Age=${FORGE_TOKEN_TTL_SECONDS}; HttpOnly; SameSite=Lax; Secure`);
		expect(forgeTokenCookie('SEALED', false)).not.toContain('Secure');
		expect(clearForgeTokenCookie(true)).toBe(`${FORGE_TOKEN_COOKIE}=; Path=/v1/t; Max-Age=0; HttpOnly; SameSite=Lax; Secure`);
	});
});

describe('captureForgeToken — the /auth/session response', () => {
	const sync = (body: unknown) => new Request('http://127.0.0.1:5173/auth/session', { method: 'POST', body: JSON.stringify(body) });
	const answered = (session: unknown, status = 200) =>
		new Response(JSON.stringify({ session, error: null }), { status, headers: { 'set-cookie': 'session=x; Path=/' } });
	const GITHUB_SESSION = {
		access_token: 'jwt',
		provider_token: 'gho_abc',
		user: { id: ALICE, app_metadata: { provider: 'github' } }
	};
	// What kavach validated: the login, and the GitHub account Supabase linked to
	// it (identities[].id is GitHub's user id) — 111 here.
	const SYNCED = {
		access_token: 'jwt',
		user: {
			id: ALICE,
			app_metadata: { provider: 'email', providers: ['email', 'github'] },
			identities: [{ provider: 'github', id: '111' }]
		}
	};
	/** GitHub's GET /user for the posted token. */
	const githubSays = (id: number, status = 200) => fakeFetch({ '/user': { status, body: { id, login: 'whoever' } } });

	it('seals a GitHub sign-in`s provider token into the cookie, bound to the synced login — GitHub judged from the synced login, not the request', async () => {
		const gh = githubSays(111);
		const res = await captureForgeToken(sync({ event: 'SIGNED_IN', session: GITHUB_SESSION }), answered(SYNCED), SECRET, NOW, gh.fn);
		expect(gh.calls.map((c) => c.url)).toEqual(['https://api.github.com/user']);
		const cookies = res.headers.getSetCookie();
		expect(cookies[0]).toBe('session=x; Path=/');
		const mine = cookies.find((c) => c.startsWith(`${FORGE_TOKEN_COOKIE}=`))!;
		const sealed = mine.split(';')[0].slice(FORGE_TOKEN_COOKIE.length + 1);
		expect(await openForgeToken(sealed, ALICE, SECRET, NOW)).toBe('gho_abc');
		expect(await res.json()).toEqual({ session: SYNCED, error: null });
	});

	it('clears it on sign-out, and leaves every other answer alone', async () => {
		const out = await captureForgeToken(sync({ event: 'SIGNED_OUT', session: null }), answered(null), SECRET, NOW, githubSays(111).fn);
		expect(out.headers.getSetCookie()).toContain(clearForgeTokenCookie(false));

		for (const [body, res] of [
			[{ event: 'SIGNED_IN', session: { ...GITHUB_SESSION, provider_token: undefined } }, answered(SYNCED)],
			// The REQUEST claims GitHub; the validated login has no GitHub linked.
			[{ event: 'SIGNED_IN', session: GITHUB_SESSION }, answered({ ...SYNCED, user: { id: ALICE, app_metadata: { provider: 'email', providers: ['email'] }, identities: [] } })],
			[{ event: 'TOKEN_REFRESHED', session: GITHUB_SESSION }, answered(SYNCED)],
			[{ event: 'SIGNED_IN', session: GITHUB_SESSION }, answered(null, 500)]
		] as const) {
			const r = await captureForgeToken(sync(body), res, SECRET, NOW, githubSays(111).fn);
			expect(r.headers.getSetCookie().some((c) => c.startsWith(`${FORGE_TOKEN_COOKIE}=`))).toBe(false);
		}
	});

	it('does not seal another GitHub account`s token: the login is linked to 111, the token is 222`s', async () => {
		const res = await captureForgeToken(sync({ event: 'SIGNED_IN', session: GITHUB_SESSION }), answered(SYNCED), SECRET, NOW, githubSays(222).fn);
		expect(res.headers.getSetCookie().some((c) => c.startsWith(`${FORGE_TOKEN_COOKIE}=`))).toBe(false);
		// …and the sign-in itself still answers as kavach did.
		expect(res.status).toBe(200);
		expect(res.headers.getSetCookie()).toContain('session=x; Path=/');
	});

	it('does not seal a token GitHub would not identify', async () => {
		const res = await captureForgeToken(sync({ event: 'SIGNED_IN', session: GITHUB_SESSION }), answered(SYNCED), SECRET, NOW, githubSays(111, 401).fn);
		expect(res.headers.getSetCookie().some((c) => c.startsWith(`${FORGE_TOKEN_COOKIE}=`))).toBe(false);
	});
});
