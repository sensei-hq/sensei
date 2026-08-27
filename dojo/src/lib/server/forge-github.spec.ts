// The forge reads that provisioning is built on.
//
// The contract under test is fail-closed: these functions feed tenant creation,
// so anything they return becomes a governance boundary. A fabricated or
// defaulted org is far worse than an error.
import { describe, it, expect } from 'vitest';
import { AdminError } from './admin-data';
import { fetchGithubUser, fetchGithubOrgs, fetchGithubFacts } from './forge-github';

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

const USER_OK = { id: 4242, login: 'jerrythomas', name: 'Jerry Thomas', email: 'j@example.com' };

describe('fetchGithubUser', () => {
	it('returns the stable id as text, plus login/name/email', async () => {
		const { fn, calls } = fakeFetch({ '/user': { body: USER_OK } });
		expect(await fetchGithubUser('tok', fn)).toEqual({
			id: '4242',
			login: 'jerrythomas',
			name: 'Jerry Thomas',
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
	it('returns id + login + role for each active membership', async () => {
		const { fn, calls } = fakeFetch({
			'/user/memberships/orgs': {
				body: [
					{ state: 'active', role: 'admin', organization: { id: 11, login: 'sensei-hq' } },
					{ state: 'active', role: 'member', organization: { id: 22, login: 'acme' } }
				]
			}
		});
		expect(await fetchGithubOrgs('tok', fn)).toEqual([
			{ id: '11', login: 'sensei-hq', role: 'admin' },
			{ id: '22', login: 'acme', role: 'member' }
		]);
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
		expect(await fetchGithubOrgs('tok', fn)).toEqual([
			{ id: '22', login: 'acme', role: 'member' }
		]);
	});

	it('skips an entry with no stable id or no login rather than guessing', async () => {
		// A connection is a claim of identity; a guessed one is worse than none (F7).
		const { fn } = fakeFetch({
			'/user/memberships/orgs': {
				body: [
					{ state: 'active', role: 'member', organization: { login: 'no-id' } },
					{ state: 'active', role: 'member', organization: { id: 33 } },
					{ state: 'active', role: 'member', organization: { id: 44, login: 'good' } }
				]
			}
		});
		expect(await fetchGithubOrgs('tok', fn)).toEqual([{ id: '44', login: 'good', role: 'member' }]);
	});

	it('treats any role that is not admin as member', async () => {
		const { fn } = fakeFetch({
			'/user/memberships/orgs': {
				body: [{ state: 'active', role: 'billing_manager', organization: { id: 1, login: 'a' } }]
			}
		});
		expect((await fetchGithubOrgs('tok', fn))[0].role).toBe('member');
	});

	it('throws 502 on a non-2xx — never an empty list', async () => {
		// The difference matters enormously: [] means "you are in no orgs", which
		// a de-provisioning pass would later act on (§IV.6). An API outage must
		// never be spelled the same way as "the user left everything".
		const { fn } = fakeFetch({ '/user/memberships/orgs': { status: 500, body: {} } });
		await expect(fetchGithubOrgs('tok', fn)).rejects.toMatchObject({ status: 502 });
	});

	it('returns [] when the body is not an array', async () => {
		const { fn } = fakeFetch({ '/user/memberships/orgs': { body: { message: 'weird' } } });
		expect(await fetchGithubOrgs('tok', fn)).toEqual([]);
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
