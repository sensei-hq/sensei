import { describe, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { AUTH_LANDING_FRAGMENT, AUTH_PENDING_ATTR, isAuthLanding, revealOnSync, REVEAL_AFTER_MS, signinErrorFrom } from './auth-landing';

// #329. A magic link lands on a URL whose fragment carries the new session. The
// server cannot see a fragment, so it renders with whatever cookie the browser
// still holds — the PREVIOUS account's — until kavach posts the new session and
// reloads the data. The page stays hidden from first paint until then.

describe('isAuthLanding', () => {
	it('is true when the fragment carries an access token', () => {
		expect(isAuthLanding('#access_token=eyJ.a.b&expires_in=3600&type=magiclink')).toBe(true);
		expect(isAuthLanding('#expires_in=3600&access_token=eyJ')).toBe(true);
	});

	it('is false otherwise, including a failed link (nothing to wait for)', () => {
		for (const h of ['', '#', '#section', '#error=access_denied&error_description=expired', '#xaccess_token=1']) {
			expect(isAuthLanding(h)).toBe(false);
		}
	});
});

function root(pending = true) {
	const attrs = new Set(pending ? [AUTH_PENDING_ATTR] : []);
	return {
		hasAttribute: (n: string) => attrs.has(n),
		removeAttribute: (n: string) => void attrs.delete(n)
	};
}

describe('revealOnSync', () => {
	it('reveals only after the reload as the new session has finished', async () => {
		const r = root();
		let release!: () => void;
		const invalidate = vi.fn(() => new Promise<void>((res) => (release = res)));
		const wrapped = revealOnSync(r, invalidate, () => () => {});
		const done = wrapped();
		expect(invalidate).toHaveBeenCalledOnce();
		expect(r.hasAttribute(AUTH_PENDING_ATTR)).toBe(true);
		release();
		await done;
		expect(r.hasAttribute(AUTH_PENDING_ATTR)).toBe(false);
	});

	it('reveals after a bounded wait when no sync ever completes (the server is still on the old session)', () => {
		const r = root();
		let fire!: () => void;
		let waited = 0;
		revealOnSync(r, async () => {}, (fn, ms) => {
			fire = fn;
			waited = ms;
			return () => {};
		});
		expect(waited).toBe(REVEAL_AFTER_MS);
		fire();
		expect(r.hasAttribute(AUTH_PENDING_ATTR)).toBe(false);
	});

	it('cancels the wait once revealed, and later reloads still run', async () => {
		const r = root();
		const cancel = vi.fn();
		const invalidate = vi.fn(async () => {});
		const wrapped = revealOnSync(r, invalidate, () => cancel);
		await wrapped();
		await wrapped();
		expect(cancel).toHaveBeenCalledOnce();
		expect(invalidate).toHaveBeenCalledTimes(2);
	});
});

describe('the first-paint guard in app.html', () => {
	const html = readFileSync('src/app.html', 'utf8');
	const css = readFileSync('src/app.css', 'utf8');

	it('runs the same fragment test before anything paints, and marks <html> pending', () => {
		expect(html).toContain(AUTH_LANDING_FRAGMENT.source);
		expect(html).toContain(`document.documentElement.setAttribute('${AUTH_PENDING_ATTR}'`);
		expect(html.indexOf(AUTH_LANDING_FRAGMENT.source)).toBeLessThan(html.indexOf('%sveltekit.body%'));
	});

	it('hides the page while pending and says why', () => {
		expect(css).toContain(`html[${AUTH_PENDING_ATTR}] body`);
		expect(css).toMatch(/Signing you in/);
	});
});

// #297. Supabase reports a failed sign-in in the URL fragment (`#error=…&
// error_description=…`), sometimes in the query. The sign-in page used to ignore
// it, so a failed sign-in looked like nothing happened.
describe('signinErrorFrom', () => {
	it('reads the description from the fragment, decoded', () => {
		expect(
			signinErrorFrom('#error=access_denied&error_code=otp_expired&error_description=Email+link+is+invalid+or+has+expired', '')
		).toEqual({ code: 'otp_expired', message: 'Email link is invalid or has expired' });
	});

	it('reads the query form too, and falls back to the error code when there is no description', () => {
		expect(signinErrorFrom('', '?error=server_error')).toEqual({ code: 'server_error', message: 'server_error' });
	});

	it('explains Supabase’s refusal to link two accounts that share an email', () => {
		const e = signinErrorFrom(
			'#error=server_error&error_code=unexpected_failure&error_description=Multiple+accounts+with+the+same+email+address+in+the+same+linking+domain+detected',
			''
		);
		expect(e?.code).toBe('unexpected_failure');
		expect(e?.message).toMatch(/more than one dōjō account/i);
	});

	it('is null when nothing went wrong, including a successful landing', () => {
		expect(signinErrorFrom('', '')).toBeNull();
		expect(signinErrorFrom('#access_token=x&type=magiclink', '?next=/you')).toBeNull();
	});
});
