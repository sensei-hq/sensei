import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrIntegrations from './ScrIntegrations.svelte';
import { IntegrationsState } from '$lib/integrations-state.svelte';
import type { IntegrationUnit } from '$lib/integrations-view';
import { DojoApiError } from '$lib/dojo-api';
import { AzureConnectState } from '$lib/azure-connect-state.svelte';

// Admin → Integrations (#308 R5a): connect GitHub (a re-authorization), see the orgs it sees,
// and for each — what its dōjō is, where its repositories file, and only the acts
// this person may take (claim, its own dōjō, a workspace here).
const ADMIN_ORG: IntegrationUnit = {
	provider: 'github',
	id: '22',
	login: 'acme',
	role: 'admin',
	dojo: { key: 'organization/acme', name: 'acme', claimed: false, claimedBy: null },
	mapping: null,
	canClaim: true,
	canMapAsDojo: true,
	canMapAsWorkspace: true
};
const MEMBER_ORG: IntegrationUnit = {
	provider: 'github',
	id: '33',
	login: 'beta',
	role: 'member',
	dojo: { key: 'organization/beta', name: 'beta', claimed: true, claimedBy: 'Ada' },
	mapping: null,
	canClaim: false,
	canMapAsDojo: false,
	canMapAsWorkspace: false
};

function setup(units: IntegrationUnit[] = [ADMIN_ORG, MEMBER_ORG], complete = true) {
	const api = {
		list: vi.fn().mockResolvedValue({ provider: 'github', units, complete }),
		claim: vi.fn().mockResolvedValue({}),
		map: vi.fn().mockResolvedValue({}),
		connect: vi.fn().mockResolvedValue(undefined)
	};
	const state = new IntegrationsState(api);
	return { api, state };
}

describe('ScrIntegrations', () => {
	afterEach(cleanup);

	it('without a GitHub connection: Connect GitHub, and no token field anywhere', async () => {
		const { api, state } = setup();
		api.list.mockRejectedValue(new DojoApiError(428, 'Connect GitHub to read your organizations'));
		await state.load();
		const { getByRole, queryByLabelText, container } = render(ScrIntegrations, { props: { orgName: 'Here', state } });
		await fireEvent.click(getByRole('button', { name: 'Connect GitHub' }));
		expect(api.connect).toHaveBeenCalledTimes(1);
		expect(queryByLabelText(/token/i)).toBeNull();
		expect(container.querySelector('input[type=password]')).toBeNull();
	});

	it('connected: the orgs it sees, each with its dōjō and where it files', async () => {
		const { state } = setup();
		await state.load();
		const { getByText } = render(ScrIntegrations, { props: { orgName: 'Here', state } });
		expect(getByText('acme')).toBeTruthy();
		expect(getByText('organization/acme · unclaimed — an admin of acme on GitHub can claim it')).toBeTruthy();
		expect(getByText('organization/beta · claimed by Ada')).toBeTruthy();
	});

	it('offers only the acts the person may take, and takes them', async () => {
		const { api, state } = setup();
		await state.load();
		const { getByRole, queryByRole, getByLabelText } = render(ScrIntegrations, { props: { orgName: 'Here', state } });

		await fireEvent.click(getByRole('button', { name: 'Claim the acme dōjō' }));
		expect(api.claim).toHaveBeenCalledWith('22');
		await fireEvent.click(getByRole('button', { name: 'Make acme its own dōjō' }));
		expect(api.map).toHaveBeenLastCalledWith('22', { as: 'dojo' });
		await fireEvent.input(getByLabelText('Workspace name for acme'), { target: { value: 'Acme Web' } });
		await fireEvent.click(getByRole('button', { name: 'File acme as a workspace of Here' }));
		expect(api.map).toHaveBeenLastCalledWith('22', { as: 'workspace', name: 'Acme Web' });

		// beta: a member of a claimed org — nothing to do here.
		expect(queryByRole('button', { name: 'Claim the beta dōjō' })).toBeNull();
		expect(queryByRole('button', { name: 'Make beta its own dōjō' })).toBeNull();
		expect(queryByRole('button', { name: 'File beta as a workspace of Here' })).toBeNull();
	});

	it('says when the read was incomplete, and when the connected account sees none', async () => {
		const partial = setup([ADMIN_ORG], false);
		await partial.state.load();
		const a = render(ScrIntegrations, { props: { orgName: 'Here', state: partial.state } });
		expect(a.getByText(/could not read every organization/)).toBeTruthy();
		cleanup();

		const none = setup([]);
		await none.state.load();
		const b = render(ScrIntegrations, { props: { orgName: 'Here', state: none.state } });
		expect(b.getByText(/GitHub account sees no organizations/)).toBeTruthy();
	});

	it('shows why a read or an act failed', async () => {
		const { state } = setup();
		state.error = 'GitHub read failed (401)';
		const { getByRole } = render(ScrIntegrations, { props: { orgName: 'Here', state } });
		expect(getByRole('alert').textContent).toContain('GitHub read failed (401)');
	});

	it('offers Connect Azure DevOps beside GitHub when the page gives it (#308 R5b)', async () => {
		const { state } = setup();
		const azure = new AzureConnectState({ start: vi.fn(), navigate: vi.fn(), organizations: vi.fn(), importOrg: vi.fn() });
		const { getByRole, queryByLabelText } = render(ScrIntegrations, { props: { orgName: 'Here', state, azure } });
		expect(getByRole('button', { name: 'Connect Azure DevOps' })).toBeTruthy();
		// No token field: Microsoft hands the grant to the server.
		expect(queryByLabelText(/token/i)).toBeNull();
	});
});
