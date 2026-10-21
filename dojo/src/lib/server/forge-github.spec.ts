// The forge reads that provisioning is built on.
//
// The contract under test is fail-closed: these functions feed tenant creation,
// so anything they return becomes a governance boundary. A fabricated or
// defaulted org is far worse than an error.
import { describe, it, expect } from 'vitest';
import { AdminError } from './admin-data';
import {
	fetchGithubUser,
	fetchGithubOrgs,
	fetchGithubFacts,
	fetchGithubRepoFacts,
	fetchGithubRepoPermissions,
	permissionOf
} from './forge-github';

/** A fetch stub that answers by URL substring and records the calls. */
function fakeFetch(routes: Record<string, { status?: number; body: unknown }>) {
	const calls: { url: string; init?: RequestInit }[] = [];
	const fn = (async (url: string | URL | Request, init?: RequestInit) => {
		const u = String(url);
		calls.push({ url: u, init });
		const key = Object.keys(routes).find((k) => u.includes(k));
		const route = key ? routes[key] : undefined;
		const status = route?.status ?? (route ? 200 : 404);
		return {
			ok: status >= 200 && status < 300,
			status,
			json: async () => route?.body ?? null
		} as Response;
	}) as unknown as typeof fetch;
	return { fn, calls };
}

const USER_OK = { id: 4242, login: 'devuser', name: 'Dev User', email: 'j@example.com' };

describe('fetchGithubUser', () => {
	it('returns the stable id as text, plus login/name/email', async () => {
		const { fn, calls } = fakeFetch({ '/user': { body: USER_OK } });
		expect(await fetchGithubUser('tok', fn)).toEqual({
			id: '4242',
			login: 'devuser',
			name: 'Dev User',
			email: 'j@example.com'
		});
		// The token travels as a header, never in the URL (where it would land in
		// logs and referrers).
		expect(calls[0].url).not.toContain('tok');
		expect((calls[0].init?.headers as Record<string, string>).Authorization).toBe('Bearer tok');
	});

	it('nulls a missing name/email rather than inventing one', async () => {
		const { fn } = fakeFetch({ '/user': { body: { id: 1, login: 'ghost' } } });
		expect(await fetchGithubUser('tok', fn)).toMatchObject({ name: null, email: null });
	});

	it('throws 502 on a non-2xx — never a placeholder user', async () => {
		const { fn } = fakeFetch({ '/user': { status: 401, body: {} } });
		const err = await fetchGithubUser('tok', fn).catch((e) => e);
		expect(err).toBeInstanceOf(AdminError);
		expect(err.status).toBe(502);
	});

	it('throws when the response has no id or no login', async () => {
		// `String(undefined)` is "undefined", which would cheerfully become an
		// identity subject. A read this broken is a failed read.
		const noId = fakeFetch({ '/user': { body: { login: 'x' } } });
		await expect(fetchGithubUser('tok', noId.fn)).rejects.toBeInstanceOf(AdminError);
		const noLogin = fakeFetch({ '/user': { body: { id: 7 } } });
		await expect(fetchGithubUser('tok', noLogin.fn)).rejects.toBeInstanceOf(AdminError);
	});
});

describe('fetchGithubOrgs', () => {
	it('returns id + login + role for each active membership, and says the read was complete', async () => {
		const { fn, calls } = fakeFetch({
			'/user/memberships/orgs': {
				body: [
					{ state: 'active', role: 'admin', organization: { id: 11, login: 'sensei-hq' } },
					{ state: 'active', role: 'member', organization: { id: 22, login: 'acme' } }
				]
			}
		});
		expect(await fetchGithubOrgs('tok', fn)).toEqual({
			orgs: [
				{ id: '11', login: 'sensei-hq', role: 'admin' },
				{ id: '22', login: 'acme', role: 'member' }
			],
			complete: true
		});
		expect(calls[0].url).toContain('/user/memberships/orgs');
	});

	it('excludes a PENDING invitation', async () => {
		// An invitation is not membership. Provisioning from one would hand
		// someone a tenant — a governance boundary — they were merely invited to.
		const { fn } = fakeFetch({
			'/user/memberships/orgs': {
				body: [
					{ state: 'pending', role: 'admin', organization: { id: 11, login: 'not-yet' } },
					{ state: 'active', role: 'member', organization: { id: 22, login: 'acme' } }
				]
			}
		});
		expect((await fetchGithubOrgs('tok', fn)).orgs).toEqual([{ id: '22', login: 'acme', role: 'member' }]);
	});

	it('skips an entry with no stable id or no login rather than guessing — and the read is then incomplete', async () => {
		// A connection is a claim of identity; a guessed one is worse than none (F7).
		// An org it could not read must not look like an org the person left (#309).
		const { fn } = fakeFetch({
			'/user/memberships/orgs': {
				body: [
					{ state: 'active', role: 'member', organization: { login: 'no-id' } },
					{ state: 'active', role: 'member', organization: { id: 33 } },
					{ state: 'active', role: 'member', organization: { id: 44, login: 'good' } }
				]
			}
		});
		expect(await fetchGithubOrgs('tok', fn)).toEqual({
			orgs: [{ id: '44', login: 'good', role: 'member' }],
			complete: false
		});
	});

	it('treats any role that is not admin as member', async () => {
		const { fn } = fakeFetch({
			'/user/memberships/orgs': {
				body: [{ state: 'active', role: 'billing_manager', organization: { id: 1, login: 'a' } }]
			}
		});
		expect((await fetchGithubOrgs('tok', fn)).orgs[0].role).toBe('member');
	});

	it('reads every page — someone in more than 100 orgs is in all of them (#309)', async () => {
		const page1 = Array.from({ length: 100 }, (_, i) => ({
			state: 'active',
			role: 'member',
			organization: { id: i + 1, login: `org-${i + 1}` }
		}));
		const { fn, calls } = fakeFetch({
			'&page=1': { body: page1 },
			'&page=2': { body: [{ state: 'active', role: 'admin', organization: { id: 101, login: 'org-101' } }] }
		});
		const got = await fetchGithubOrgs('tok', fn);
		expect(got.orgs).toHaveLength(101);
		expect(got.complete).toBe(true);
		expect(calls.map((c) => c.url)).toEqual([
			expect.stringContaining('&page=1'),
			expect.stringContaining('&page=2')
		]);
	});

	it('throws 502 on a non-2xx — never an empty list', async () => {
		// The difference matters enormously: [] means "you are in no orgs", which
		// a de-provisioning pass acts on (#309). An API outage must never be
		// spelled the same way as "the user left everything".
		const { fn } = fakeFetch({ '/user/memberships/orgs': { status: 500, body: {} } });
		await expect(fetchGithubOrgs('tok', fn)).rejects.toMatchObject({ status: 502 });
	});

	it('throws when the body is not a list — an unreadable answer is not "in no orgs"', async () => {
		const { fn } = fakeFetch({ '/user/memberships/orgs': { body: { message: 'weird' } } });
		await expect(fetchGithubOrgs('tok', fn)).rejects.toMatchObject({ status: 502 });
	});
});

describe('fetchGithubRepoFacts', () => {
	it('maps the forge`s `private` boolean to public/private', async () => {
		// The forge's answer, read from the forge — never inferred from the remote
		// URL or the owner's name (acceptance criterion 6 of the sharing doc).
		const open = fakeFetch({ '/repos/sensei-hq/dbd': { body: { private: false, default_branch: 'main' } } });
		expect((await fetchGithubRepoFacts('sensei-hq', 'dbd', 'tok', open.fn))?.visibility).toBe('public');
		expect(open.calls[0].url).toBe('https://api.github.com/repos/sensei-hq/dbd');
		// The token travels as a header, never in the URL.
		expect(open.calls[0].url).not.toContain('tok');
		expect((open.calls[0].init?.headers as Record<string, string>).Authorization).toBe(
			'Bearer tok'
		);

		const shut = fakeFetch({ '/repos/acme/secret': { body: { private: true } } });
		expect((await fetchGithubRepoFacts('acme', 'secret', 'tok', shut.fn))?.visibility).toBe('private');
	});

	it('reads the default branch from the same response, and says null when GitHub does not state one', async () => {
		// The branch the shared code graph follows by default (graph-sync.md §9).
		// Absent is not `main`: a guessed branch would file a graph nobody tracks.
		const stated = fakeFetch({ '/repos/acme/x': { body: { private: true, default_branch: 'develop' } } });
		expect(await fetchGithubRepoFacts('acme', 'x', 'tok', stated.fn)).toEqual({
			visibility: 'private',
			defaultBranch: 'develop'
		});
		for (const default_branch of [undefined, '', 42]) {
			const unstated = fakeFetch({ '/repos/acme/x': { body: { private: false, default_branch } } });
			expect(await fetchGithubRepoFacts('acme', 'x', 'tok', unstated.fn)).toEqual({
				visibility: 'public',
				defaultBranch: null
			});
		}
	});

	it('returns null on 404 — this token cannot see it, which is not a value', async () => {
		// No access, or renamed upstream. Writing `private` here would be a guess
		// that in an org tenant resolves to ORG-MANDATED and shares the repository
		// with no election by anyone (§8a BLOCKING).
		const { fn } = fakeFetch({ '/repos/acme/gone': { status: 404, body: {} } });
		expect(await fetchGithubRepoFacts('acme', 'gone', 'tok', fn)).toBeNull();
	});

	it('throws 502 on any other non-2xx — never a guessed visibility', async () => {
		for (const status of [401, 403, 500, 503]) {
			const { fn } = fakeFetch({ '/repos/acme/x': { status, body: {} } });
			const err = await fetchGithubRepoFacts('acme', 'x', 'tok', fn).catch((e) => e);
			expect(err).toBeInstanceOf(AdminError);
			expect(err.status).toBe(502);
		}
	});

	it('throws when the body carries no boolean `private`', async () => {
		// A shape we cannot read is a FAILED read. `!body.private` would silently
		// call every unreadable response "public", which is the free-to-host path.
		const missing = fakeFetch({ '/repos/acme/x': { body: { name: 'x' } } });
		await expect(fetchGithubRepoFacts('acme', 'x', 'tok', missing.fn)).rejects.toBeInstanceOf(
			AdminError
		);
		const notBool = fakeFetch({ '/repos/acme/x': { body: { private: 'true' } } });
		await expect(fetchGithubRepoFacts('acme', 'x', 'tok', notBool.fn)).rejects.toBeInstanceOf(
			AdminError
		);
	});

	it('percent-encodes the path segments it was given', async () => {
		// owner/repo reach this from a stored repo_key. Encoding them keeps a
		// segment from escaping into another API path.
		const { fn, calls } = fakeFetch({ '/repos/': { body: { private: false } } });
		await fetchGithubRepoFacts('a/../b', 'c d', 'tok', fn);
		expect(calls[0].url).toBe('https://api.github.com/repos/a%2F..%2Fb/c%20d');
	});
});

describe('fetchGithubFacts', () => {
	it('composes user + orgs and stamps the provider', async () => {
		const { fn } = fakeFetch({
			'/user/memberships/orgs': {
				body: [{ state: 'active', role: 'admin', organization: { id: 11, login: 'sensei-hq' } }]
			},
			'/user': { body: USER_OK }
		});
		const facts = await fetchGithubFacts('tok', fn);
		expect(facts.provider).toBe('github');
		expect(facts.user.id).toBe('4242');
		expect(facts.orgs).toEqual([{ id: '11', login: 'sensei-hq', role: 'admin' }]);
		expect(facts.orgsComplete).toBe(true);
	});

	it('throws rather than returning a partial when either read fails', async () => {
		// A half-read picture would look like "you are in no orgs" to the caller.
		const { fn } = fakeFetch({
			'/user/memberships/orgs': { status: 503, body: {} },
			'/user': { body: USER_OK }
		});
		await expect(fetchGithubFacts('tok', fn)).rejects.toBeInstanceOf(AdminError);
	});
});

// #307 A2c: a member's own permission on every repository they can see — what the
// per-person forge mirror records. A partial list would CLEAR roles on the
// repositories it missed, so the read is all or nothing.
describe('permissionOf', () => {
	it('takes the highest permission GitHub reports', () => {
		expect(permissionOf({ admin: true, maintain: true, push: true, triage: true, pull: true })).toBe('admin');
		expect(permissionOf({ admin: false, maintain: true, push: true, pull: true })).toBe('maintain');
		expect(permissionOf({ push: true, pull: true })).toBe('write');
		expect(permissionOf({ triage: true, pull: true })).toBe('triage');
		expect(permissionOf({ pull: true })).toBe('read');
		expect(permissionOf({})).toBeNull();
		expect(permissionOf(undefined)).toBeNull();
	});
});

describe('fetchGithubRepoPermissions', () => {
	const repo = (n: number, perms: Record<string, boolean> = { push: true, pull: true }) => ({
		full_name: `Acme/Repo-${n}`,
		permissions: perms
	});

	it('reads every page and keys each repository as the daemon does', async () => {
		const page1 = Array.from({ length: 100 }, (_, i) => repo(i));
		const { fn, calls } = fakeFetch({
			'&page=1': { body: page1 },
			'&page=2': { body: [repo(100, { admin: true, pull: true }), { full_name: 'Acme/no-perms' }] }
		});
		const got = await fetchGithubRepoPermissions('tok', fn);
		expect(got).toHaveLength(101);
		expect(got[0]).toEqual({ repo_key: 'github.com/acme/repo-0', permission: 'write' });
		expect(got[100]).toEqual({ repo_key: 'github.com/acme/repo-100', permission: 'admin' });
		expect(calls.map((c) => c.url).every((u) => u.includes('/user/repos'))).toBe(true);
	});

	it('a failed page fails the whole read — never a partial list', async () => {
		const page1 = Array.from({ length: 100 }, (_, i) => repo(i));
		const { fn } = fakeFetch({ '&page=1': { body: page1 }, '&page=2': { status: 502, body: null } });
		await expect(fetchGithubRepoPermissions('tok', fn)).rejects.toThrow(/502/);
	});

	it('more repositories than it will page through fails rather than truncates', async () => {
		const full = Array.from({ length: 100 }, (_, i) => repo(i));
		const { fn } = fakeFetch({ '&page=': { body: full } });
		await expect(fetchGithubRepoPermissions('tok', fn)).rejects.toThrow(/more than/);
	});
});
