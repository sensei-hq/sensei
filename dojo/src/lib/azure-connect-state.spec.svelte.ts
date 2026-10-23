import { describe, it, expect, vi } from 'vitest';
import { AzureConnectState, type AzureConnectApi } from './azure-connect-state.svelte';
import { DojoApiError } from './dojo-api';
import type { AzureConnectionView, AzureImportResult } from './azure-view';

// Connect Azure DevOps (#308 R5b, D35): the organization URL and a PAT, sent once
// to the server, which validates the token before storing it. The PAT field is
// emptied after every request — success or failure — and is never kept, shown
// or sent anywhere else. A dōjō's connections list each organization's token
// state; sync re-reads with the stored token; a failure says why.
const PAT = 'ztestpatAZDO0000';
const CONN: AzureConnectionView = { id: 'org-1', name: 'example-org', verifiedAt: null, token: 'ok', tokenExpiresAt: null };
const IMPORTED: AzureImportResult = {
	dojo: { key: 'organization/example-org', created: true },
	organization: { id: 'org-1', name: 'example-org' },
	projects: { total: 2, mapped: 2, elsewhere: 0, complete: true }
};

function api(over: Partial<AzureConnectApi> = {}) {
	return {
		status: vi.fn().mockResolvedValue({ connections: [CONN], scopes: [] }),
		connect: vi.fn().mockResolvedValue(IMPORTED),
		sync: vi.fn().mockResolvedValue({ organization: IMPORTED.organization, projects: IMPORTED.projects }),
		removeToken: vi.fn().mockResolvedValue(undefined),
		...over
	};
}

describe('AzureConnectState', () => {
	it('reads the dōjō\'s connections', async () => {
		const s = new AzureConnectState(api());
		await s.load();
		expect(s.connections).toEqual([CONN]);
		expect(s.loaded).toBe(true);
	});

	it('a failed read says why and shows no list', async () => {
		const s = new AzureConnectState(api({ status: vi.fn().mockRejectedValue(new DojoApiError(500, 'boom')) }));
		await s.load();
		expect(s.connections).toEqual([]);
		expect(s.loaded).toBe(false);
		expect(s.error).toBe('boom');
	});

	it('connects with the URL and the token, says what it came to, re-reads, and empties the token field', async () => {
		const a = api();
		const s = new AzureConnectState(a);
		s.organizationUrl = 'https://dev.azure.com/example-org';
		s.token = PAT;
		await s.connect();
		expect(a.connect).toHaveBeenCalledWith('https://dev.azure.com/example-org', PAT);
		expect(s.token).toBe('');
		expect(s.message).toMatch(/example-org is now the dōjō organization\/example-org/);
		expect(s.lastImport).toEqual(IMPORTED);
		expect(a.status).toHaveBeenCalled();
		expect(s.busy).toBeNull();
	});

	it('a refused token: the reason, the token field emptied all the same, no message', async () => {
		const a = api({ connect: vi.fn().mockRejectedValue(new DojoApiError(422, 'Azure DevOps did not accept that token')) });
		const s = new AzureConnectState(a);
		s.organizationUrl = 'https://dev.azure.com/example-org';
		s.token = PAT;
		await s.connect();
		expect(s.token).toBe('');
		expect(s.error).toBe('Azure DevOps did not accept that token');
		expect(s.message).toBeNull();
		expect(JSON.stringify(s)).not.toContain(PAT);
	});

	it('asks for both fields before sending anything', async () => {
		const a = api();
		const s = new AzureConnectState(a);
		s.token = PAT;
		await s.connect();
		expect(a.connect).not.toHaveBeenCalled();
		expect(s.error).toMatch(/organization URL/);
		s.organizationUrl = 'https://dev.azure.com/example-org';
		s.token = '  ';
		await s.connect();
		expect(a.connect).not.toHaveBeenCalled();
		expect(s.error).toMatch(/token/);
	});

	it('works with no connection list (the personal dōjō): connect only', async () => {
		const connect = vi.fn().mockResolvedValue(IMPORTED);
		const s = new AzureConnectState({ connect });
		await s.load();
		expect(s.loaded).toBe(false);
		s.organizationUrl = 'https://dev.azure.com/example-org';
		s.token = PAT;
		await s.connect();
		expect(connect).toHaveBeenCalledTimes(1);
		expect(s.message).toMatch(/organization\/example-org/);
	});

	it('syncs one organization and removes a token, re-reading after each', async () => {
		const a = api();
		const s = new AzureConnectState(a);
		await s.sync(CONN);
		expect(a.sync).toHaveBeenCalledWith('org-1');
		expect(s.message).toBe('example-org synced · 2 projects filed as workspaces');
		await s.removeToken(CONN);
		expect(a.removeToken).toHaveBeenCalledWith('org-1');
		expect(a.status).toHaveBeenCalledTimes(2);
	});

	it('a sync refused because the token is dead says so', async () => {
		const a = api({ sync: vi.fn().mockRejectedValue(new DojoApiError(409, 'Azure DevOps no longer accepts the stored token for example-org; paste a new one')) });
		const s = new AzureConnectState(a);
		await s.sync(CONN);
		expect(s.error).toMatch(/paste a new one/);
		expect(a.status).toHaveBeenCalledTimes(1);
	});
});
