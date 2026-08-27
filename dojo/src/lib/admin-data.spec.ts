import { describe, expect, it, vi } from 'vitest';
import {
	AdminApiError,
	addMember,
	createIdentity,
	deleteIdentity,
	deletePolicy,
	dojoApiUrl,
	getHealth,
	listAudit,
	listIdentities,
	listMembers,
	listPolicies,
	patchPolicy,
	setMemberRole,
	updateIdentity,
	upsertPolicy
} from '$lib/admin-data';

// A fake fetch that records the call and returns a canned JSON response. Mirrors
// the triage-data.spec harness (same shape) since both clients share dojo-api.
function fakeFetch(status: number, body: unknown) {
	const calls: { url: string; init?: RequestInit }[] = [];
	const fn = vi.fn(async (url: string | URL, init?: RequestInit) => {
		calls.push({ url: String(url), init });
		return {
			ok: status >= 200 && status < 300,
			status,
			json: async () => body
		} as Response;
	});
	return { fn: fn as unknown as typeof fetch, calls };
}

function headersOf(init?: RequestInit): Record<string, string> {
	return (init?.headers ?? {}) as Record<string, string>;
}

describe('admin-data reuses the shared dojo base url', () => {
	it('defaults to same-origin (empty base)', () => {
		expect(dojoApiUrl).toBe('');
	});
});

describe('listMembers', () => {
	it('GETs the tenant members path and unwraps members', async () => {
		const members = [{ id: 'm1' }, { id: 'm2' }];
		const { fn, calls } = fakeFetch(200, { members });
		const out = await listMembers('github/globex', { fetch: fn });
		expect(out).toEqual(members);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/members`);
		expect(calls[0].init?.method ?? 'GET').toBe('GET');
	});

	it('encodes each tenant segment but keeps the slash as a separator', async () => {
		const { fn, calls } = fakeFetch(200, { members: [] });
		await listMembers('other/acme corp', { fetch: fn });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/other/acme%20corp/members`);
	});

	it('sends a bearer header when an access token is given', async () => {
		const { fn, calls } = fakeFetch(200, { members: [] });
		await listMembers('t/x', { fetch: fn, accessToken: 'jwt-123' });
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt-123');
	});

	it('omits the Authorization header when the token is null or absent', async () => {
		const nullTok = fakeFetch(200, { members: [] });
		await listMembers('t/x', { fetch: nullTok.fn, accessToken: null });
		expect(headersOf(nullTok.calls[0].init).Authorization).toBeUndefined();

		const noTok = fakeFetch(200, { members: [] });
		await listMembers('t/x', { fetch: noTok.fn });
		expect(headersOf(noTok.calls[0].init).Authorization).toBeUndefined();
	});

	it('returns an empty array when the envelope has no members', async () => {
		const { fn } = fakeFetch(200, {});
		expect(await listMembers('t/x', { fetch: fn })).toEqual([]);
	});

	it('throws an AdminApiError carrying the API error message on a 403', async () => {
		const { fn } = fakeFetch(403, { error: 'admin role required' });
		await expect(listMembers('t/x', { fetch: fn })).rejects.toMatchObject({
			name: 'DojoApiError',
			status: 403,
			message: 'admin role required'
		});
		const again = fakeFetch(403, { error: 'admin role required' });
		await expect(listMembers('t/x', { fetch: again.fn })).rejects.toBeInstanceOf(AdminApiError);
	});
});

describe('addMember', () => {
	it('POSTs the membership body with the bearer + json content-type', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'm9', role: 'contributor' });
		const body = { user_id: 'u1', kind: 'human', authenticated_via: 'sso', git_provider_role: 'write' };
		const out = await addMember('github/globex', body, { fetch: fn, accessToken: 'jwt' });
		expect(out).toEqual({ id: 'm9', role: 'contributor' });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/members`);
		expect(calls[0].init?.method).toBe('POST');
		expect(headersOf(calls[0].init)['content-type']).toBe('application/json');
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual(body);
	});
});

describe('setMemberRole', () => {
	it('PATCHes the member role path with the role body (user id encoded)', async () => {
		const { fn, calls } = fakeFetch(200, { user_id: 'u/1', role: 'maintainer' });
		const out = await setMemberRole('github/globex', 'u/1', 'maintainer', { fetch: fn });
		expect(out).toEqual({ user_id: 'u/1', role: 'maintainer' });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/members/u%2F1/role`);
		expect(calls[0].init?.method).toBe('PATCH');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({ role: 'maintainer' });
	});

	it('surfaces the API 400 for an invalid role', async () => {
		const { fn } = fakeFetch(400, { error: 'role must be contributor|maintainer|lead|admin' });
		await expect(setMemberRole('t/x', 'u1', 'wizard', { fetch: fn })).rejects.toBeInstanceOf(
			AdminApiError
		);
	});
});

describe('identities', () => {
	it('GETs + unwraps identities', async () => {
		const identities = [{ id: 'i1' }];
		const { fn, calls } = fakeFetch(200, { identities });
		expect(await listIdentities('t/x', { fetch: fn })).toEqual(identities);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/identities`);
	});

	it('POSTs a new identity body', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'i2' });
		const body = { principal_id: 'p1', provider: 'sso', subject: 'okta|abc', email: 'a@b.co' };
		await createIdentity('t/x', body, { fetch: fn });
		expect(calls[0].init?.method).toBe('POST');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual(body);
	});

	it('PATCHes an identity by id (id encoded)', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'i 3' });
		await updateIdentity('t/x', 'i 3', { display_name: 'Kei' }, { fetch: fn });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/identities/i%203`);
		expect(calls[0].init?.method).toBe('PATCH');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({ display_name: 'Kei' });
	});

	it('DELETEs an identity by id with no body', async () => {
		const { fn, calls } = fakeFetch(200, { deleted: true });
		expect(await deleteIdentity('t/x', 'i1', { fetch: fn })).toEqual({ deleted: true });
		expect(calls[0].init?.method).toBe('DELETE');
		expect(calls[0].init?.body).toBeUndefined();
	});
});

describe('policies', () => {
	it('GETs + unwraps policies', async () => {
		const policies = [{ id: 'p1', scope_key: 'Company' }];
		const { fn, calls } = fakeFetch(200, { policies });
		expect(await listPolicies('t/x', { fetch: fn })).toEqual(policies);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/policies`);
	});

	it('POSTs (upserts) a policy body', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'p2', scope_key: 'Team · Payments' });
		const body = { scope_key: 'Team · Payments', attribution_default: 'anonymous', retention_days: 365 };
		await upsertPolicy('t/x', body, { fetch: fn });
		expect(calls[0].init?.method).toBe('POST');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual(body);
	});

	it('PATCHes a policy by id', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'p1' });
		await patchPolicy('t/x', 'p1', { retention_days: 90 }, { fetch: fn });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/policies/p1`);
		expect(calls[0].init?.method).toBe('PATCH');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({ retention_days: 90 });
	});

	it('DELETEs a policy by id', async () => {
		const { fn, calls } = fakeFetch(200, { deleted: true });
		expect(await deletePolicy('t/x', 'p1', { fetch: fn })).toEqual({ deleted: true });
		expect(calls[0].init?.method).toBe('DELETE');
	});
});

describe('getHealth', () => {
	it('GETs the health path and returns the rollup verbatim', async () => {
		const rollup = { connections: 3, queue_depth: 5, publish_rate_1h: 12, error_rate_1h: 0 };
		const { fn, calls } = fakeFetch(200, rollup);
		expect(await getHealth('github/globex', { fetch: fn })).toEqual(rollup);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/health`);
	});
});

describe('listAudit', () => {
	it('GETs the audit path and unwraps events', async () => {
		const events = [{ id: 2 }, { id: 1 }];
		const { fn, calls } = fakeFetch(200, { events });
		expect(await listAudit('t/x', { fetch: fn })).toEqual(events);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/audit`);
	});

	it('appends a limit query when given', async () => {
		const { fn, calls } = fakeFetch(200, { events: [] });
		await listAudit('t/x', { fetch: fn, limit: 50 });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/audit?limit=50`);
	});

	it('returns an empty array when the envelope has no events', async () => {
		const { fn } = fakeFetch(200, {});
		expect(await listAudit('t/x', { fetch: fn })).toEqual([]);
	});
});
