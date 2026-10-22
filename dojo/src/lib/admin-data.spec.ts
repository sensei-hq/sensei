import { describe, expect, it, vi } from 'vitest';
import {
	AdminApiError,
	deletePolicy,
	dojoApiUrl,
	getHealth,
	listAudit,
	listIdentities,
	listMembers,
	listPolicies,
	patchPolicy,
	setMemberRole,
	upsertPolicy,
	listViewers,
	removeViewer,
	grantWorkspaceView,
	revokeWorkspaceView,
	listTeams,
	createTeam,
	renameTeam,
	deleteTeam,
	addTeamMember,
	removeTeamMember,
	grantTeamWorkspaceView,
	getTeamMetrics,
	listIntegrationUnits,
	claimIntegrationUnit,
	mapIntegrationUnit
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

describe("the dōjō's viewers (#307 D8)", () => {
	it('lists them', async () => {
		const row = { principal_id: 'v/1', display_name: null, email: 'v@x.co', joined_at: 't', workspaces: [] };
		const { fn, calls } = fakeFetch(200, { viewers: [row] });
		expect(await listViewers('personal/acme', { fetch: fn })).toEqual([row]);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/personal/acme/viewers`);
	});

	it('removes one, and grants and revokes a workspace (ids encoded)', async () => {
		const { fn, calls } = fakeFetch(204, null);
		await removeViewer('personal/acme', 'v/1', { fetch: fn });
		await grantWorkspaceView('personal/acme', 'v/1', 'p/1', { fetch: fn });
		await revokeWorkspaceView('personal/acme', 'v/1', 'p/1', { fetch: fn });
		expect(calls.map((c) => [c.init?.method, c.url])).toEqual([
			['DELETE', `${dojoApiUrl}/v1/t/personal/acme/viewers/v%2F1`],
			['PUT', `${dojoApiUrl}/v1/t/personal/acme/viewers/v%2F1/workspaces/p%2F1`],
			['DELETE', `${dojoApiUrl}/v1/t/personal/acme/viewers/v%2F1/workspaces/p%2F1`]
		]);
	});

	it('surfaces a refusal with its reason', async () => {
		const { fn } = fakeFetch(409, { error: 'invite them to this dōjō as a viewer first' });
		await expect(grantWorkspaceView('personal/acme', 'v1', 'p1', { fetch: fn })).rejects.toThrow(
			'invite them to this dōjō as a viewer first'
		);
	});
});

describe("the lead's team view (#307 R8, D38)", () => {
	it('reads GET …/team-metrics and returns the wire as sent', async () => {
		const wire = { since: '2026-10-16', days: 7, teams: [] };
		const { fn, calls } = fakeFetch(200, wire);
		expect(await getTeamMetrics('personal/acme', { fetch: fn })).toEqual(wire);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/personal/acme/team-metrics`);
	});

	it('a refusal is an error, never an empty view', async () => {
		const { fn } = fakeFetch(403, { error: 'lead role required' });
		await expect(getTeamMetrics('personal/acme', { fetch: fn })).rejects.toThrow();
	});
});

describe("the dōjō's teams (#307 D7)", () => {
	it('lists them', async () => {
		const { fn, calls } = fakeFetch(200, { teams: [{ id: 't1', name: 'P', members: [] }] });
		expect(await listTeams('personal/acme', { fetch: fn })).toEqual([{ id: 't1', name: 'P', members: [] }]);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/personal/acme/teams`);
	});

	it('writes to the team paths (ids encoded)', async () => {
		const { fn, calls } = fakeFetch(204, null);
		await createTeam('personal/acme', 'Platform', { fetch: fn });
		await renameTeam('personal/acme', 't/1', 'Core', { fetch: fn });
		await addTeamMember('personal/acme', 't/1', 'u/1', { fetch: fn });
		await removeTeamMember('personal/acme', 't/1', 'u/1', { fetch: fn });
		await grantTeamWorkspaceView('personal/acme', 't/1', 'p/1', { fetch: fn });
		await deleteTeam('personal/acme', 't/1', { fetch: fn });
		const base = `${dojoApiUrl}/v1/t/personal/acme/teams`;
		expect(calls.map((c) => [c.init?.method, c.url, c.init?.body ?? null])).toEqual([
			['POST', base, JSON.stringify({ name: 'Platform' })],
			['PATCH', `${base}/t%2F1`, JSON.stringify({ name: 'Core' })],
			['PUT', `${base}/t%2F1/members/u%2F1`, null],
			['DELETE', `${base}/t%2F1/members/u%2F1`, null],
			['PUT', `${base}/t%2F1/workspaces/p%2F1`, null],
			['DELETE', `${base}/t%2F1`, null]
		]);
	});
});

describe('the dōjō`s integrations (#308 R5a)', () => {
	it('POSTs to list, claim and map — the bodies carry no token (the GitHub connection is server-side)', async () => {
		const view = { provider: 'github', units: [], complete: true };
		const { fn, calls } = fakeFetch(200, view);
		expect(await listIntegrationUnits('organization/acme', 'github', { fetch: fn, accessToken: 'jwt' })).toEqual(view);
		await claimIntegrationUnit('organization/acme', 'github', '22', { fetch: fn });
		await mapIntegrationUnit('organization/acme', 'github', '22', { as: 'workspace', name: 'Acme' }, { fetch: fn });
		const base = `${dojoApiUrl}/v1/t/organization/acme/integrations/github`;
		expect(calls.map((c) => [c.init?.method, c.url, c.init?.body])).toEqual([
			['POST', base, JSON.stringify({})],
			['POST', `${base}/claim`, JSON.stringify({ unit_id: '22' })],
			['POST', `${base}/map`, JSON.stringify({ unit_id: '22', as: 'workspace', name: 'Acme' })]
		]);
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt');
	});

	it('throws the API`s refusal, with its status, rather than an empty list', async () => {
		const { fn } = fakeFetch(428, { error: 'Connect GitHub to read your organizations' });
		const err = await listIntegrationUnits('organization/acme', 'github', { fetch: fn }).catch((e) => e);
		expect(err).toBeInstanceOf(AdminApiError);
		expect(err.status).toBe(428);
	});
});
