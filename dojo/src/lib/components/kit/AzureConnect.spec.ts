import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import AzureConnect from './AzureConnect.svelte';
import { AzureConnectState } from '$lib/azure-connect-state.svelte';
import { DojoApiError } from '$lib/dojo-api';
import type { AzureConnectionView, AzureImportResult } from '$lib/azure-view';

// Connect Azure DevOps (#308 R5b, D35): the organization URL, a PAT in a password
// field that is empty again after the request, the read-only scopes to give it,
// and — on an organization dōjō — each connected organization's token state with
// Sync and Remove token.
const PAT = 'ztestpatAZDO0000';
const IMPORTED: AzureImportResult = {
	dojo: { key: 'organization/example-org', created: true },
	organization: { id: 'org-1', name: 'example-org' },
	projects: { total: 2, mapped: 2, elsewhere: 0, complete: true },
	members: { complete: true, listed: 3, ended: 0 }
};
const CONN: AzureConnectionView = { id: 'org-1', name: 'example-org', verifiedAt: null, token: 'ok', tokenExpiresAt: null };
const DEAD: AzureConnectionView = { id: 'org-2', name: 'other-org', verifiedAt: null, token: 'needs_new_token', tokenExpiresAt: null };

function setup(withList = true) {
	const api = {
		connect: vi.fn().mockResolvedValue(IMPORTED),
		...(withList
			? {
					status: vi.fn().mockResolvedValue({ connections: [CONN, DEAD], scopes: [] }),
					sync: vi.fn().mockResolvedValue({ organization: IMPORTED.organization, projects: IMPORTED.projects, members: IMPORTED.members }),
					removeToken: vi.fn().mockResolvedValue(undefined)
				}
			: {})
	};
	return { api, state: new AzureConnectState(api) };
}

describe('AzureConnect', () => {
	afterEach(cleanup);

	it('asks for the organization URL and a token in a password field, naming the read-only scopes', () => {
		const { state } = setup();
		const { getByLabelText, getByText } = render(AzureConnect, { props: { state } });
		expect((getByLabelText('Organization URL') as HTMLInputElement).type).toBe('text');
		const pat = getByLabelText('Personal access token') as HTMLInputElement;
		expect(pat.type).toBe('password');
		expect(pat.getAttribute('autocomplete')).toBe('off');
		for (const s of ['Project and Team (Read)', 'Graph (Read)', 'Member Entitlement Management (Read)', 'Code (Read)']) {
			expect(getByText(s, { exact: false })).toBeTruthy();
		}
		expect(getByText(/read-only personal access token/i)).toBeTruthy();
	});

	it('connects once, then the token field is empty and the result is said', async () => {
		const { api, state } = setup();
		const { getByLabelText, getByRole, findByText } = render(AzureConnect, { props: { state } });
		await fireEvent.input(getByLabelText('Organization URL'), { target: { value: 'https://dev.azure.com/example-org' } });
		const pat = getByLabelText('Personal access token') as HTMLInputElement;
		await fireEvent.input(pat, { target: { value: PAT } });
		await fireEvent.click(getByRole('button', { name: 'Connect Azure DevOps' }));
		expect(api.connect).toHaveBeenCalledWith('https://dev.azure.com/example-org', PAT);
		expect(await findByText(/example-org is now the dōjō organization\/example-org/)).toBeTruthy();
		expect(pat.value).toBe('');
	});

	it('a refused token: the reason in an alert, and the field emptied', async () => {
		const { api, state } = setup();
		api.connect.mockRejectedValue(new DojoApiError(422, 'Azure DevOps did not accept that token'));
		const { getByLabelText, getByRole, findByRole } = render(AzureConnect, { props: { state } });
		await fireEvent.input(getByLabelText('Organization URL'), { target: { value: 'https://dev.azure.com/example-org' } });
		const pat = getByLabelText('Personal access token') as HTMLInputElement;
		await fireEvent.input(pat, { target: { value: PAT } });
		await fireEvent.click(getByRole('button', { name: 'Connect Azure DevOps' }));
		expect((await findByRole('alert')).textContent).toMatch(/did not accept that token/);
		expect(pat.value).toBe('');
	});

	it('on an organization dōjō: each organization, its token state, Sync and Remove token', async () => {
		const { api, state } = setup();
		await state.load();
		const { getByText, getByRole } = render(AzureConnect, { props: { state } });
		expect(getByText('example-org')).toBeTruthy();
		expect(getByText(/no longer accepts the stored token/)).toBeTruthy();
		// A token Azure no longer takes cannot sync; only a new one mends it.
		expect((getByRole('button', { name: 'Sync other-org' }) as HTMLButtonElement).disabled).toBe(true);
		const sync = getByRole('button', { name: 'Sync example-org' }) as HTMLButtonElement;
		expect(sync.disabled).toBe(false);
		await fireEvent.click(sync);
		expect(api.sync).toHaveBeenCalledWith('org-1');
		await fireEvent.click(getByRole('button', { name: 'Remove the token for example-org' }));
		expect(api.removeToken).toHaveBeenCalledWith('org-1');
	});

	it('in the personal dōjō: the form only, no connection list', () => {
		const { state } = setup(false);
		const { queryByRole } = render(AzureConnect, { props: { state } });
		expect(queryByRole('button', { name: /^Sync / })).toBeNull();
	});
});
