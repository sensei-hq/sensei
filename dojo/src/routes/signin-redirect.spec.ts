// Where an authenticated visitor lands.
//
// Observed 2026-08-27: a successful GitHub sign-in left the user staring at the
// login page. Supabase returns the browser to `site_url`, which is the origin —
// `/` — and `/+page.ts` redirected to `/signin` UNCONDITIONALLY. kavach then did
// its job perfectly (POST /auth/session → 200, invalidateAll), so the user held
// a valid session while looking at a sign-in form, and `/signin` had no reason
// to send them anywhere.
//
// Both loads are session-aware now. `invalidateAll()` re-runs them after the
// cookie is set, which is what makes the /signin case fire at all: the session
// does not exist during the first server render of the OAuth return, only after
// the client has settled.
import { describe, it, expect } from 'vitest';
import { load as rootLoad } from './+page';
import { load as signinLoad } from './signin/+page';

type Redirect = { status?: number; location?: string };

/** Run a load that is expected to throw a redirect, and return it. */
async function redirectFrom(
	fn: (arg: unknown) => unknown,
	session: unknown
): Promise<Redirect> {
	try {
		await fn({ parent: async () => ({ session }) });
	} catch (e) {
		return e as Redirect;
	}
	throw new Error('expected the load to throw a redirect');
}

/** Run a load that is expected NOT to redirect. */
async function noRedirectFrom(fn: (arg: unknown) => unknown, session: unknown): Promise<void> {
	await fn({ parent: async () => ({ session }) });
}

const SESSION = { access_token: 'at', user: { id: 'u1' } };

describe('/ — the OAuth return target', () => {
	it('sends a signed-in visitor to their personal zone, not back to the login page', async () => {
		// The actual defect. Supabase redirects here after GitHub, so a blanket
		// bounce to /signin makes a successful sign-in indistinguishable from a
		// failed one.
		const r = await redirectFrom(rootLoad as never, SESSION);
		expect(r.status).toBe(307);
		expect(r.location).toBe('/you');
	});

	it('still sends an anonymous visitor to sign in', async () => {
		const r = await redirectFrom(rootLoad as never, null);
		expect(r.status).toBe(307);
		expect(r.location).toBe('/signin');
	});
});

describe('/signin — reached while already authenticated', () => {
	it('forwards a signed-in visitor to their personal zone', async () => {
		// kavach calls invalidateAll() once the session cookie is set, which
		// re-runs this load — so this is what actually completes the OAuth return
		// after the first server render saw no session.
		const r = await redirectFrom(signinLoad as never, SESSION);
		expect(r.status).toBe(307);
		expect(r.location).toBe('/you');
	});

	it('renders the form for an anonymous visitor', async () => {
		// Must NOT redirect — otherwise nobody can ever reach the sign-in page.
		await expect(noRedirectFrom(signinLoad as never, null)).resolves.toBeUndefined();
	});
});
