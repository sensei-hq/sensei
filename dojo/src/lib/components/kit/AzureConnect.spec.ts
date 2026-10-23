import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AzureConnect from './AzureConnect.svelte';
import { AzureConnectState } from '$lib/azure-connect-state.svelte';
import { DojoApiError } from '$lib/dojo-api';
import type { AzureConnectionView, AzureGrantView, AzureImportResult } from '$lib/azure-view';

// Connect Azure DevOps through Microsoft Entra (#308 R5b, D42): a Connect button
// (no token field anywhere), the read scopes the connect asks for and the
// work-or-school-account note; after Microsoft, the organizations to import, a
// warning when the consent carries more than reading; on an organization dōjō,
// each connected organization's state with Sync and Remove.
const IMPORTED: AzureImportResult = {
	dojo: { key: 'organization/example-org', created: true },
	organization: { id: 'org-1', name: 'example-org' },
	projects: { total: 2, mapped: 2, elsewhere: 0, complete: true },
	members: { complete: true, listed: 3, ended: 0 }
};
const CONN: AzureConnectionView = { id: 'org-1', name: 'example-org', verifiedAt: null, token: 'ok', tokenExpiresAt: null };
const DEAD: AzureConnectionView = { id: 'org-2', name: 'other-org', verifiedAt: null, token: 'needs_new_token', tokenExpiresAt: null };
const GRANT: AzureGrantView = { organizations: [{ id: 'a1', name: 'example-org' }, { id: 'a2', name: 'second-org' }], complete: true, writeScopes: [], target: 'this' };

function setup(over: { grant?: AzureGrantView | null; configured?: boolean; withList?: boolean } = {}) {
	const api = {
		start: vi.fn().mockResolvedValue({ url: 'https://login.microsoftonline.com/x' }),
		navigate: vi.fn(),
		organizations: over.grant === null ? vi.fn().mockRejectedValue(new DojoApiError(428, 'Connect first')) : vi.fn().mockResolvedValue(over.grant ?? GRANT),
		importOrg: vi.fn().mockResolvedValue(IMPORTED),
		...(over.withList === false
			? {}
			: {
					status: vi.fn().mockResolvedValue({ connections: [CONN, DEAD], scopes: [], configured: over.configured ?? true }),
					sync: vi.fn().mockResolvedValue({ organization: IMPORTED.organization, projects: IMPORTED.projects, members: IMPORTED.members }),
					removeToken: vi.fn().mockResolvedValue(undefined)
				})
	};
	return { api, state: new AzureConnectState(api) };
}

describe('AzureConnect', () => {
	afterEach(cleanup);

	it('offers Connect — with the read scopes and the account note — and no token field anywhere', async () => {
		const { api, state } = setup({ grant: null });
		await state.load();
		const { getByRole, getByText, container } = render(AzureConnect, { props: { state } });
		for (const s of ['User profile (read)', 'Project and team (read)', 'Graph (read)', 'Member entitlement management (read)', 'Code (read)']) {
			expect(getByText(s, { exact: false })).toBeTruthy();
		}
		expect(getByText(/work or school account/)).toBeTruthy();
		expect(getByText(/Personal Microsoft accounts cannot/)).toBeTruthy();
		expect(container.querySelector('input[type=password]')).toBeNull();
		expect(container.querySelector('input')).toBeNull();
		await fireEvent.click(getByRole('button', { name: 'Connect Azure DevOps' }));
		expect(api.navigate).toHaveBeenCalledWith('https://login.microsoftonline.com/x');
	});

	it('says when this dōjō cannot connect, and offers no button', async () => {
		const { state } = setup({ grant: null, configured: false });
		await state.load();
		const { getByText, queryByRole } = render(AzureConnect, { props: { state } });
		expect(getByText('Azure DevOps is not configured on this dōjō')).toBeTruthy();
		expect(queryByRole('button', { name: 'Connect Azure DevOps' })).toBeNull();
	});

	it('after Microsoft: each organization to import, and where it goes', async () => {
		const { api, state } = setup();
		await state.load();
		const { getByRole, getByText, findByText } = render(AzureConnect, { props: { state } });
		expect(getByText(/will connect to this dōjō/i)).toBeTruthy();
		await fireEvent.click(getByRole('button', { name: 'Import second-org' }));
		expect(api.importOrg).toHaveBeenCalledWith('second-org');
		expect(await findByText(/example-org is now the dōjō organization\/example-org/)).toBeTruthy();
	});

	it('warns when the consent carries more than reading, and when the list is incomplete', async () => {
		const { state } = setup({ grant: { ...GRANT, writeScopes: ['vso.code_write'], complete: false, target: 'own' } });
		await state.load();
		const { getByText } = render(AzureConnect, { props: { state } });
		expect(getByText(/also grants vso.code_write/)).toBeTruthy();
		expect(getByText(/could not list every organization/)).toBeTruthy();
		expect(getByText(/each becomes its own dōjō/i)).toBeTruthy();
	});

	it('on an organization dōjō: each organization, its state, Sync and Remove', async () => {
		const { api, state } = setup({ grant: null });
		await state.load();
		const { getByText, getByRole } = render(AzureConnect, { props: { state } });
		expect(getByText(/reconnect Azure DevOps/)).toBeTruthy();
		// A connection Microsoft no longer accepts cannot sync; only reconnecting mends it.
		expect((getByRole('button', { name: 'Sync other-org' }) as HTMLButtonElement).disabled).toBe(true);
		const sync = getByRole('button', { name: 'Sync example-org' }) as HTMLButtonElement;
		expect(sync.disabled).toBe(false);
		await fireEvent.click(sync);
		expect(api.sync).toHaveBeenCalledWith('org-1');
		await fireEvent.click(getByRole('button', { name: 'Remove the connection to example-org' }));
		expect(api.removeToken).toHaveBeenCalledWith('org-1');
	});

	it('a refusal is said in an alert', async () => {
		const { state } = setup({ grant: null });
		state.error = 'Azure DevOps is not configured on this dōjō';
		const { getByRole } = render(AzureConnect, { props: { state } });
		expect(getByRole('alert').textContent).toContain('not configured');
	});

	it('in the personal dōjō: no connection list', async () => {
		const { state } = setup({ grant: null, withList: false });
		await state.load();
		const { queryByRole, getByRole } = render(AzureConnect, { props: { state } });
		expect(getByRole('button', { name: 'Connect Azure DevOps' })).toBeTruthy();
		expect(queryByRole('button', { name: /^Sync / })).toBeNull();
	});
});
