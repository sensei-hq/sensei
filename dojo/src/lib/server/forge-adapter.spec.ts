// The one interface every forge sits behind (#308 R5a): what a person can see
// (`listUnits`), whether they administer one (`proveAdmin`), and who belongs to
// one (`listMembers`). Each read that ends something carries `complete`, and a
// read that cannot be trusted THROWS — never an empty list, never a `false` that
// is really an outage (#309's rule: a partial read never ends a membership).
import { describe, it, expect } from 'vitest';
import { AdminError } from './admin-data';
import { forgeAdapter, type ForgeAdapter } from './forge-adapter';
import { fetchGithubOrgs, ForgeReadError } from './forge-github';
import { fakeFetch } from './fake-forge-fetch';

const ACME = { id: '22', login: 'acme' };

describe('forgeAdapter', () => {
	it('gives GitHub an adapter and names the provider it speaks for', () => {
		expect(forgeAdapter('github').provider).toBe('github');
	});

	it('refuses a forge that has no adapter yet, rather than reading it as GitHub', () => {
		// Azure DevOps' adapter is bound to one named organization
		// (`azureAdapter(org)`, #308 R5b), so asked for by provider alone it is a
		// 501 like GitLab and Bitbucket — never a GitHub read with someone else's token.
		for (const p of ['azure_devops', 'gitlab', 'bitbucket'] as const) {
			expect(() => forgeAdapter(p)).toThrow(AdminError);
			try {
				forgeAdapter(p);
			} catch (e) {
				expect((e as AdminError).status).toBe(501);
			}
		}
	});
});

describe('GitHub adapter · listUnits', () => {
	it('is the org-membership read provisioning already uses — same units, same completeness', async () => {
		const body = [
			{ state: 'active', role: 'admin', organization: { id: 11, login: 'sensei-hq' } },
			{ state: 'active', role: 'member', organization: { login: 'no-id' } },
			{ state: 'active', role: 'member', organization: { id: 22, login: 'acme' } }
		];
		const a = fakeFetch({ '/user/memberships/orgs': { body } });
		const b = fakeFetch({ '/user/memberships/orgs': { body } });
		const adapter: ForgeAdapter = forgeAdapter('github', a.fn);
		const read = await fetchGithubOrgs('tok', b.fn);
		expect(await adapter.listUnits('tok')).toEqual({ units: read.orgs, complete: read.complete });
		expect(read.complete).toBe(false);
	});

	it('throws on a failed read — an outage is never "in no orgs"', async () => {
		const { fn } = fakeFetch({ '/user/memberships/orgs': { status: 500, body: {} } });
		await expect(forgeAdapter('github', fn).listUnits('tok')).rejects.toMatchObject({ status: 502 });
	});
});

describe('GitHub adapter · proveAdmin', () => {
	const membership = (role: string, state = 'active', id: unknown = 22) => ({
		'/user/memberships/orgs/acme': { body: { state, role, organization: { id, login: 'acme' } } }
	});

	it('is true only for an ACTIVE admin membership, read with the person`s own token', async () => {
		const { fn, calls } = fakeFetch(membership('admin'));
		expect(await forgeAdapter('github', fn).proveAdmin('tok', ACME)).toBe(true);
		expect(calls[0].url).toBe('https://api.github.com/user/memberships/orgs/acme');
		expect((calls[0].init?.headers as Record<string, string>).Authorization).toBe('Bearer tok');
	});

	it('is false for a member, and for a pending admin invitation', async () => {
		expect(await forgeAdapter('github', fakeFetch(membership('member')).fn).proveAdmin('tok', ACME)).toBe(false);
		expect(
			await forgeAdapter('github', fakeFetch(membership('admin', 'pending')).fn).proveAdmin('tok', ACME)
		).toBe(false);
	});

	it('is false when the name now belongs to a DIFFERENT org — the stable id decides', async () => {
		// A slug can be renamed and re-registered upstream; administering the new
		// holder of the name proves nothing about the org we know by its id.
		const { fn } = fakeFetch(membership('admin', 'active', 99));
		expect(await forgeAdapter('github', fn).proveAdmin('tok', ACME)).toBe(false);
	});

	it('is false on 404 — GitHub`s answer for "not a member of that org"', async () => {
		const { fn } = fakeFetch({});
		expect(await forgeAdapter('github', fn).proveAdmin('tok', ACME)).toBe(false);
	});

	it('THROWS on any other failure — a refused read is not "not an admin"', async () => {
		for (const status of [401, 403, 500]) {
			const { fn } = fakeFetch({ '/user/memberships/orgs/acme': { status, body: {} } });
			const err = await forgeAdapter('github', fn)
				.proveAdmin('tok', ACME)
				.catch((e) => e);
			expect(err).toBeInstanceOf(ForgeReadError);
			expect(err.forgeStatus).toBe(status);
		}
	});

	it('encodes the org name into the path', async () => {
		const { fn, calls } = fakeFetch({});
		await forgeAdapter('github', fn).proveAdmin('tok', { id: '1', login: 'a/b c' });
		expect(calls[0].url).toBe('https://api.github.com/user/memberships/orgs/a%2Fb%20c');
	});
});

describe('GitHub adapter · listMembers', () => {
	it('reads every page of the org`s members and says the read was complete', async () => {
		const page1 = Array.from({ length: 100 }, (_, i) => ({ id: i + 1, login: `m-${i + 1}` }));
		const { fn, calls } = fakeFetch({
			'/orgs/acme/members?per_page=100&page=1': { body: page1 },
			'/orgs/acme/members?per_page=100&page=2': { body: [{ id: 101, login: 'm-101' }] }
		});
		const got = await forgeAdapter('github', fn).listMembers('tok', ACME);
		expect(got.complete).toBe(true);
		expect(got.members).toHaveLength(101);
		expect(got.members[100]).toEqual({ id: '101', login: 'm-101' });
		expect(calls).toHaveLength(2);
	});

	it('skips a member it cannot name, and the read is then incomplete', async () => {
		const { fn } = fakeFetch({
			'/orgs/acme/members': { body: [{ login: 'no-id' }, { id: 5 }, { id: 6, login: 'ok' }] }
		});
		expect(await forgeAdapter('github', fn).listMembers('tok', ACME)).toEqual({
			members: [{ id: '6', login: 'ok' }],
			complete: false
		});
	});

	it('throws on a failed page or an answer that is not a list — never a shorter list', async () => {
		const failed = fakeFetch({ '/orgs/acme/members': { status: 503, body: {} } });
		await expect(forgeAdapter('github', failed.fn).listMembers('tok', ACME)).rejects.toBeInstanceOf(
			ForgeReadError
		);
		const odd = fakeFetch({ '/orgs/acme/members': { body: { message: 'weird' } } });
		await expect(forgeAdapter('github', odd.fn).listMembers('tok', ACME)).rejects.toMatchObject({
			status: 502
		});
	});

	it('throws rather than return a truncated list when the org is larger than it will read', async () => {
		const full = Array.from({ length: 100 }, (_, i) => ({ id: i + 1, login: `m-${i + 1}` }));
		const { fn } = fakeFetch({ '/orgs/acme/members': { body: full } });
		await expect(forgeAdapter('github', fn).listMembers('tok', ACME)).rejects.toMatchObject({ status: 502 });
	});
});
