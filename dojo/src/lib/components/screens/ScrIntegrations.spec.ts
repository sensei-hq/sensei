import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import ScrIntegrations from './ScrIntegrations.svelte';
import { IntegrationsState } from '$lib/integrations-state.svelte';
import type { IntegrationUnit } from '$lib/integrations-view';

// Admin → Integrations (#308 R5a): paste a GitHub token, see the orgs it can see,
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
		map: vi.fn().mockResolvedValue({})
	};
	const state = new IntegrationsState(api);
	return { api, state };
}

describe('ScrIntegrations', () => {
	afterEach(cleanup);

	it('asks for a token and reads the orgs it sees', async () => {
		const { api, state } = setup();
		const { getByLabelText, getByRole, findByText } = render(ScrIntegrations, { props: { orgName: 'Here', state } });
		await fireEvent.input(getByLabelText('GitHub token'), { target: { value: 'gh-tok' } });
		await fireEvent.click(getByRole('button', { name: 'Read my GitHub organizations' }));
		expect(api.list).toHaveBeenCalledWith('gh-tok');
		expect(await findByText('acme')).toBeTruthy();
		expect(await findByText('organization/acme · unclaimed — an admin of acme on GitHub can claim it')).toBeTruthy();
		expect(await findByText('organization/beta · claimed by Ada')).toBeTruthy();
	});

	it('offers only the acts the person may take, and takes them', async () => {
		const { api, state } = setup();
		state.token = 'gh';
		await state.load();
		const { getByRole, queryByRole, getByLabelText } = render(ScrIntegrations, { props: { orgName: 'Here', state } });

		await fireEvent.click(getByRole('button', { name: 'Claim the acme dōjō' }));
		expect(api.claim).toHaveBeenCalledWith('gh', '22');
		await fireEvent.click(getByRole('button', { name: 'Make acme its own dōjō' }));
		expect(api.map).toHaveBeenLastCalledWith('gh', '22', { as: 'dojo' });
		await fireEvent.input(getByLabelText('Workspace name for acme'), { target: { value: 'Acme Web' } });
		await fireEvent.click(getByRole('button', { name: 'File acme as a workspace of Here' }));
		expect(api.map).toHaveBeenLastCalledWith('gh', '22', { as: 'workspace', name: 'Acme Web' });

		// beta: a member of a claimed org — nothing to do here.
		expect(queryByRole('button', { name: 'Claim the beta dōjō' })).toBeNull();
		expect(queryByRole('button', { name: 'Make beta its own dōjō' })).toBeNull();
		expect(queryByRole('button', { name: 'File beta as a workspace of Here' })).toBeNull();
	});

	it('says when the read was incomplete, and when the token sees no orgs', async () => {
		const partial = setup([ADMIN_ORG], false);
		partial.state.token = 'gh';
		await partial.state.load();
		const a = render(ScrIntegrations, { props: { orgName: 'Here', state: partial.state } });
		expect(a.getByText(/could not read every organization/)).toBeTruthy();
		cleanup();

		const none = setup([]);
		none.state.token = 'gh';
		await none.state.load();
		const b = render(ScrIntegrations, { props: { orgName: 'Here', state: none.state } });
		expect(b.getByText(/sees no GitHub organizations/)).toBeTruthy();
	});

	it('shows why a read or an act failed', async () => {
		const { state } = setup();
		state.error = 'GitHub read failed (401)';
		const { getByRole } = render(ScrIntegrations, { props: { orgName: 'Here', state } });
		expect(getByRole('alert').textContent).toContain('GitHub read failed (401)');
	});
});
