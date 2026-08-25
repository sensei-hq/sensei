import { describe, it, expect } from 'vitest';
import { authorizeUrl, daemonRedirect, isForwardablePort } from './cli-auth';

const base = {
	supabaseUrl: 'https://proj.supabase.co',
	origin: 'https://dojo.sensei-hq.com',
	port: 7744,
	challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM'
};

describe('authorizeUrl', () => {
	it('sends the provider back to dōjō, not to the daemon', () => {
		// The whole reason dōjō sits in the middle: one allow-list entry covers
		// every machine, instead of one per loopback port a daemon might bind.
		const u = new URL(authorizeUrl(base));
		expect(u.searchParams.get('redirect_to')).toBe(
			'https://dojo.sensei-hq.com/v1/auth/cli/callback/7744'
		);
	});

	it('passes the code challenge through with the S256 method', () => {
		const u = new URL(authorizeUrl(base));
		expect(u.searchParams.get('code_challenge')).toBe(base.challenge);
		expect(u.searchParams.get('code_challenge_method')).toBe('S256');
	});

	it('requests only the EXTRA scopes', () => {
		// `scopes` appends to the provider default, so repeating user:email would
		// duplicate it on the consent screen the user actually reads.
		const u = new URL(authorizeUrl(base));
		expect(u.searchParams.get('scopes')).toBe('read:org');
	});

	it('forwards the account hint only when one is given', () => {
		// Without a hint the browser reuses whichever GitHub session it already
		// holds, so connecting a second identity silently links the first again.
		expect(new URL(authorizeUrl({ ...base, login: 'sensei-hq-org' })).searchParams.get('login')).toBe(
			'sensei-hq-org'
		);
		expect(new URL(authorizeUrl(base)).searchParams.has('login')).toBe(false);
		expect(new URL(authorizeUrl({ ...base, login: null })).searchParams.has('login')).toBe(false);
	});

	it('does not double up slashes on a trailing-slash config value', () => {
		// Config values routinely carry one, and //auth/v1/authorize 404s.
		const u = authorizeUrl({ ...base, supabaseUrl: 'https://proj.supabase.co/' });
		expect(u).toContain('proj.supabase.co/auth/v1/authorize');
		expect(u).not.toContain('supabase.co//auth');
	});
});

describe('isForwardablePort', () => {
	it('accepts an unprivileged port, as a number or a string', () => {
		expect(isForwardablePort(7744)).toBe(true);
		expect(isForwardablePort('7744')).toBe(true);
	});

	it('rejects anything that would widen the redirect', () => {
		// This value chooses where a browser carrying an auth code is sent, so
		// everything outside the bound has to fail closed.
		for (const bad of ['', '0', '80', '65536', '-1', '77.4', 'abc', '7744abc', null, undefined]) {
			expect(isForwardablePort(bad)).toBe(false);
		}
	});
});

describe('daemonRedirect', () => {
	it('always targets 127.0.0.1', () => {
		// Never a hostname from the request — that could resolve anywhere, and
		// this URL is handed to the browser with an auth code attached.
		const u = new URL(daemonRedirect(7744, new URLSearchParams({ code: 'abc' })));
		expect(u.hostname).toBe('127.0.0.1');
		expect(u.port).toBe('7744');
		expect(u.pathname).toBe('/api/auth/callback');
		expect(u.searchParams.get('code')).toBe('abc');
	});

	it('forwards a provider error, not just a code', () => {
		// A denied consent arrives with no code at all. Dropping it would leave
		// the daemon waiting on a callback that already happened.
		const params = new URLSearchParams({
			error: 'access_denied',
			error_description: 'the user declined'
		});
		const u = new URL(daemonRedirect(7744, params));
		expect(u.searchParams.get('error')).toBe('access_denied');
		expect(u.searchParams.get('error_description')).toBe('the user declined');
	});
});
