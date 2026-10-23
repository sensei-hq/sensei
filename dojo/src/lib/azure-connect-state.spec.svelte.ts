import { describe, it, expect, vi } from 'vitest';
import { AzureConnectState, type AzureConnectApi } from './azure-connect-state.svelte';
import { DojoApiError } from './dojo-api';
import type { AzureConnectionView, AzureGrantView, AzureImportResult } from './azure-view';

// Connect Azure DevOps through Microsoft Entra (#308 R5b, D42): Connect sends the
// person to Microsoft; back on the screen, the pending connect's organizations
// are listed (with the write-scope warning) and each can be imported. A dōjō's
// connections list each organization's state; sync re-reads with the stored
// grant. No token ever passes through here. A failure says why.
const CONN: AzureConnectionView = { id: 'org-1', name: 'example-org', verifiedAt: null, token: 'ok', tokenExpiresAt: null };
const GRANT: AzureGrantView = { organizations: [{ id: 'a1', name: 'example-org' }], complete: true, writeScopes: [], target: 'this' };
const IMPORTED: AzureImportResult = {
	dojo: { key: 'organization/example-org', created: true },
	organization: { id: 'org-1', name: 'example-org' },
	projects: { total: 2, mapped: 2, elsewhere: 0, complete: true },
	members: { complete: true, listed: 3, ended: 0 }
};
const NO_GRANT = new DojoApiError(428, 'Connect Azure DevOps first — the connection has expired or was never made');

function api(over: Partial<AzureConnectApi> = {}) {
	return {
		start: vi.fn().mockResolvedValue({ url: 'https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize?x' }),
		navigate: vi.fn(),
		organizations: vi.fn().mockResolvedValue(GRANT),
		importOrg: vi.fn().mockResolvedValue(IMPORTED),
		status: vi.fn().mockResolvedValue({ connections: [CONN], scopes: [], configured: true }),
		sync: vi.fn().mockResolvedValue({ organization: IMPORTED.organization, projects: IMPORTED.projects, members: IMPORTED.members }),
		removeToken: vi.fn().mockResolvedValue(undefined),
		afterImport: vi.fn().mockResolvedValue(undefined),
		...over
	};
}

describe('AzureConnectState', () => {
	it("reads the dōjō's connections, whether it can connect, and any pending connect", async () => {
		const s = new AzureConnectState(api());
		await s.load();
		expect(s.connections).toEqual([CONN]);
		expect(s.configured).toBe(true);
		expect(s.grant).toEqual(GRANT);
		expect(s.error).toBeNull();
	});

	it('no pending connect (428) is not an error: nothing to import', async () => {
		const s = new AzureConnectState(api({ organizations: vi.fn().mockRejectedValue(NO_GRANT) }));
		await s.load();
		expect(s.grant).toBeNull();
		expect(s.error).toBeNull();
	});

	it('a failed read says why and shows no list', async () => {
		const s = new AzureConnectState(api({ status: vi.fn().mockRejectedValue(new DojoApiError(500, 'boom')) }));
		await s.load();
		expect(s.connections).toEqual([]);
		expect(s.error).toBe('boom');
	});

	it('Connect sends the person to Microsoft', async () => {
		const a = api();
		const s = new AzureConnectState(a);
		await s.connect();
		expect(a.navigate).toHaveBeenCalledWith('https://login.microsoftonline.com/organizations/oauth2/v2.0/authorize?x');
	});

	it('a connect the server refuses (not configured, too many) says why and navigates nowhere', async () => {
		const a = api({ start: vi.fn().mockRejectedValue(new DojoApiError(503, 'Azure DevOps is not configured on this dōjō')) });
		const s = new AzureConnectState(a);
		await s.connect();
		expect(s.error).toBe('Azure DevOps is not configured on this dōjō');
		expect(a.navigate).not.toHaveBeenCalled();
		expect(s.busy).toBeNull();
	});

	it('imports one organization, says where it went, and re-reads', async () => {
		const a = api();
		const s = new AzureConnectState(a);
		await s.importOrg('example-org');
		expect(a.importOrg).toHaveBeenCalledWith('example-org');
		expect(s.message).toMatch(/example-org is now the dōjō organization\/example-org/);
		expect(s.lastImport).toEqual(IMPORTED);
		expect(a.status).toHaveBeenCalled();
		expect(a.afterImport).toHaveBeenCalled();
	});

	it('an import refused says why', async () => {
		const s = new AzureConnectState(api({ importOrg: vi.fn().mockRejectedValue(new DojoApiError(403, 'only a Project Collection Administrator of example-org can import it')) }));
		await s.importOrg('example-org');
		expect(s.error).toMatch(/Project Collection Administrator/);
		expect(s.message).toBeNull();
	});

	it("takes the callback's notice from the page's query", () => {
		const s = new AzureConnectState(api());
		s.notice(new URLSearchParams('azure_error=denied'));
		expect(s.error).toBe('The Microsoft sign-in was cancelled or consent was declined');
		s.notice(new URLSearchParams('azure=connected'));
		expect(s.message).toBe('Connected to Azure DevOps — pick the organizations to import');
		expect(s.error).toBeNull();
	});

	it('works with no connection list (the personal dōjō): connect and import only', async () => {
		const a = api();
		const s = new AzureConnectState({ start: a.start, navigate: a.navigate, organizations: a.organizations, importOrg: a.importOrg });
		await s.load();
		expect(s.configured).toBeNull();
		expect(s.grant).toEqual(GRANT);
		await s.importOrg('example-org');
		expect(s.message).toMatch(/organization\/example-org/);
	});

	it('syncs one organization and removes a grant, re-reading after each — and after a refused sync', async () => {
		const a = api();
		const s = new AzureConnectState(a);
		await s.sync(CONN);
		expect(a.sync).toHaveBeenCalledWith('org-1');
		expect(s.message).toBe('example-org synced · 2 projects filed as workspaces · 3 members read');
		await s.removeToken(CONN);
		expect(a.removeToken).toHaveBeenCalledWith('org-1');
		expect(a.status).toHaveBeenCalledTimes(2);
		(a.sync as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new DojoApiError(409, 'Microsoft no longer accepts this Azure DevOps connection — reconnect Azure DevOps'));
		await s.sync(CONN);
		expect(s.error).toMatch(/reconnect/);
		expect(a.status).toHaveBeenCalledTimes(3);
	});
});
