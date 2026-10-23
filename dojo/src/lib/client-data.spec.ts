import { describe, expect, it, vi } from 'vitest';
import {
	ClientApiError,
	dojoApiUrl,
	listWorkspaces,
	listUserWorkspaces,
	getWorkspaceConstitution,
	listContributions,
	adoptContribution,
	createInvite,
	acceptInvite,
	provisionFromForge,
	startAzureConnectAsOwn,
	listAzureGrantOrganizations,
	importAzureOrganization,
	getMicrosoftLink
} from '$lib/client-data';

// A fake fetch that records the call and returns a canned response. Mirrors the
// admin-data.spec / triage-data.spec harness (same shape) since all three clients
// share dojo-api. `text` optionally backs a non-JSON (CSV) body.
function fakeFetch(status: number, body: unknown, text?: string) {
	const calls: { url: string; init?: RequestInit }[] = [];
	const fn = vi.fn(async (url: string | URL, init?: RequestInit) => {
		calls.push({ url: String(url), init });
		return {
			ok: status >= 200 && status < 300,
			status,
			json: async () => body,
			text: async () => text ?? ''
		} as Response;
	});
	return { fn: fn as unknown as typeof fetch, calls };
}

function headersOf(init?: RequestInit): Record<string, string> {
	return (init?.headers ?? {}) as Record<string, string>;
}

describe('client-data reuses the shared dojo base url', () => {
	it('defaults to same-origin (empty base)', () => {
		expect(dojoApiUrl).toBe('');
	});
});

describe('listWorkspaces / listUserWorkspaces — tenant vs user-wide reads', () => {
	it('listWorkspaces GETs the tenant-scoped workspaces path', async () => {
		const workspaces = [{ id: 'p1' }];
		const { fn, calls } = fakeFetch(200, { workspaces });
		expect(await listWorkspaces('github/globex', { fetch: fn })).toEqual(workspaces);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/workspaces`);
	});

	it('listUserWorkspaces GETs the user-wide /v1/you/workspaces path (no tenant)', async () => {
		const workspaces = [{ id: 'p1' }, { id: 'p2' }];
		const { fn, calls } = fakeFetch(200, { workspaces });
		const out = await listUserWorkspaces({ fetch: fn });
		expect(out).toEqual(workspaces);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/you/workspaces`);
		expect(calls[0].init?.method ?? 'GET').toBe('GET');
	});

	it('listUserWorkspaces unwraps to [] on a missing envelope key (honest-empty)', async () => {
		const { fn } = fakeFetch(200, {});
		expect(await listUserWorkspaces({ fetch: fn })).toEqual([]);
	});

	it('getWorkspaceConstitution GETs the per-workspace constitution path, by workspace id (#299)', async () => {
		// By id: a slug is unique only within one dōjō.
		const constitution = { rules: [], conflicts: [], locks: 0 };
		const { fn, calls } = fakeFetch(200, { constitution });
		expect(await getWorkspaceConstitution('proj-1', { fetch: fn })).toEqual(constitution);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/you/workspaces/proj-1/constitution`);
	});

	it('getWorkspaceConstitution returns null when none is federated (honest-empty)', async () => {
		const { fn } = fakeFetch(200, { constitution: null });
		expect(await getWorkspaceConstitution('proj-1', { fetch: fn })).toBeNull();
	});

	it('listContributions GETs /v1/you/contributions and unwraps { mine, downstream }', async () => {
		const { fn, calls } = fakeFetch(200, { mine: [{ title: 'a' }], downstream: [{ title: 'b' }] });
		const out = await listContributions({ fetch: fn });
		expect(out).toEqual({ mine: [{ title: 'a' }], downstream: [{ title: 'b' }] });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/you/contributions`);
	});

	it('listContributions unwraps missing keys to [] (honest-empty)', async () => {
		const { fn } = fakeFetch(200, {});
		expect(await listContributions({ fetch: fn })).toEqual({ mine: [], downstream: [] });
	});

	it('adoptContribution POSTs the artifactId to the adopt path', async () => {
		const { fn, calls } = fakeFetch(200, { ok: true });
		await adoptContribution('art-1', { fetch: fn });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/you/contributions/adopt`);
		expect(calls[0].init?.method).toBe('POST');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({ artifactId: 'art-1' });
	});

	it('createInvite POSTs to the tenant invites path (F3b issue)', async () => {
		const { fn, calls } = fakeFetch(201, { id: 'inv1', token: 'tok', email: 'a@b.co', role: 'contributor', expires_at: 'x' });
		const out = await createInvite('github/acme', { email: 'a@b.co', kind: 'client' }, { fetch: fn });
		expect(out).toMatchObject({ id: 'inv1', token: 'tok' });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/acme/invites`);
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({ email: 'a@b.co', kind: 'client' });
	});

	it('acceptInvite POSTs the token to /v1/you/invites/accept (F3b accept)', async () => {
		const { fn, calls } = fakeFetch(200, { tenant_id: 't1', role: 'contributor' });
		const out = await acceptInvite('tok', { fetch: fn });
		expect(out).toEqual({ tenant_id: 't1', role: 'contributor' });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/you/invites/accept`);
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({ token: 'tok' });
	});

	it('Azure DevOps: start from the personal dōjō, list the pending grant\'s organizations, import one — no body carries a token', async () => {
		const { fn, calls } = fakeFetch(200, { url: 'https://login.microsoftonline.com/x' });
		await startAzureConnectAsOwn({ fetch: fn, accessToken: 'jwt' });
		await listAzureGrantOrganizations({ fetch: fn });
		await importAzureOrganization('example-org', { fetch: fn });
		expect(calls.map((c) => [c.init?.method, c.url, c.init?.body])).toEqual([
			['POST', `${dojoApiUrl}/v1/you/integrations/azure_devops/connect`, JSON.stringify({})],
			['POST', `${dojoApiUrl}/v1/integrations/azure_devops/organizations`, JSON.stringify({})],
			['POST', `${dojoApiUrl}/v1/integrations/azure_devops/import`, JSON.stringify({ organization: 'example-org' })]
		]);
	});

	it('provisionFromForge POSTs to /v1/you/provision and returns the pass result', async () => {
		const result = {
			synced: true,
			personal: {
				id: 't1',
				key: 'personal/devuser',
				origin: 'personal',
				role: 'admin',
				created: true
			},
			tenants: [
				{ id: 't2', key: 'organization/acme', origin: 'organization', role: 'contributor', created: true }
			]
		};
		const { fn, calls } = fakeFetch(200, result);
		expect(await provisionFromForge({ fetch: fn })).toEqual(result);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/you/provision`);
		expect(calls[0].init?.method).toBe('POST');
	});
});

describe('getMicrosoftLink (#308 R5c)', () => {
	it('GETs /v1/you/microsoft with the bearer and returns the status', async () => {
		const { fn, calls } = fakeFetch(200, { state: 'not_linked', reason: 'why' });
		expect(await getMicrosoftLink({ fetch: fn, accessToken: 'jwt' })).toEqual({ state: 'not_linked', reason: 'why' });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/you/microsoft`);
	});
});

describe('the shared request plumbing', () => {
	it('a GET carries the bearer only when a token is given', async () => {
		const tok = fakeFetch(200, { workspaces: [] });
		await listWorkspaces('github/globex', { fetch: tok.fn, accessToken: 'jwt' });
		expect(headersOf(tok.calls[0].init).Authorization).toBe('Bearer jwt');
		const noTok = fakeFetch(200, { workspaces: [] });
		await listWorkspaces('github/globex', { fetch: noTok.fn });
		expect(headersOf(noTok.calls[0].init).Authorization).toBeUndefined();
	});

	it('a write sends JSON with the bearer', async () => {
		const { fn, calls } = fakeFetch(201, { id: 'inv1' });
		await createInvite('github/acme', { email: 'a@b.co', kind: 'member' }, { fetch: fn, accessToken: 'jwt' });
		expect(calls[0].init?.method).toBe('POST');
		expect(headersOf(calls[0].init)['content-type']).toBe('application/json');
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt');
	});

	it('throws a ClientApiError carrying the API error message on a 403', async () => {
		const { fn } = fakeFetch(403, { error: 'admin role required' });
		await expect(listWorkspaces('t/x', { fetch: fn })).rejects.toMatchObject({
			name: 'DojoApiError',
			status: 403,
			message: 'admin role required'
		});
		const again = fakeFetch(403, { error: 'admin role required' });
		await expect(listWorkspaces('t/x', { fetch: again.fn })).rejects.toBeInstanceOf(ClientApiError);
	});
});
