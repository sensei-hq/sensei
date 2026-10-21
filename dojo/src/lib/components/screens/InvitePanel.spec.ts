import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import InvitePanel from './InvitePanel.svelte';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import { InviteState } from '$lib/invite-state.svelte';

// The admin invite panel (#307 A3, D8): member or viewer, the role only for a
// member, and the accept link to share once the dōjō issues it.

const issued = { id: 'i1', token: 'tk', email: 'boss@example.com', role: 'contributor', expires_at: '' };

function opened(create = vi.fn().mockResolvedValue(issued)) {
	const s = new InviteState(create, 'https://dojo.test');
	s.start();
	return { s, create };
}

describe('InvitePanel', () => {
	afterEach(cleanup);

	it('a member invite offers the role; a viewer invite does not', async () => {
		const { s } = opened();
		const { getByRole, queryByRole } = render(InvitePanel, { props: { invite: s } });
		expect(getByRole('radiogroup', { name: 'Role' })).toBeTruthy();
		await fireEvent.click(getByRole('radio', { name: 'Viewer' }));
		await tick();
		expect(s.draft.access).toBe('viewer');
		expect(queryByRole('radiogroup', { name: 'Role' })).toBeNull();
	});

	it('sends a viewer invite and shows the link to share', async () => {
		const { s, create } = opened();
		const { getByRole, getByLabelText, findByText } = render(InvitePanel, { props: { invite: s } });
		await fireEvent.input(getByLabelText('Email'), { target: { value: 'boss@example.com' } });
		await fireEvent.click(getByRole('radio', { name: 'Viewer' }));
		await fireEvent.click(getByRole('button', { name: 'Send invite' }));
		await tick();
		expect(create).toHaveBeenCalledWith({ email: 'boss@example.com', kind: 'employer', access: 'viewer' });
		expect(await findByText('https://dojo.test/you/invites/accept?token=tk')).toBeTruthy();
	});

	it('says why an invite was not issued', async () => {
		const { s } = opened();
		const { getByRole, findByText } = render(InvitePanel, { props: { invite: s } });
		await fireEvent.click(getByRole('button', { name: 'Send invite' }));
		expect(await findByText('Enter the email address to invite.')).toBeTruthy();
	});
});

describe('ScrRoleSurfaces — the Invite button', () => {
	afterEach(cleanup);

	it('opens the invite panel on the Members tab', async () => {
		const s = new InviteState(vi.fn(), 'https://dojo.test');
		const { getByRole, queryByLabelText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme', tab: 'members', invite: s }
		});
		expect(queryByLabelText('Email')).toBeNull();
		await fireEvent.click(getByRole('button', { name: 'Invite' }));
		await tick();
		expect(queryByLabelText('Email')).toBeTruthy();
	});
});

describe('ScrRoleSurfaces — the Viewers tab', () => {
	afterEach(cleanup);

	it("lists the dōjō's viewers, and its Invite button starts a viewer invite", async () => {
		const s = new InviteState(vi.fn(), 'https://dojo.test');
		const { getByRole, getByText } = render(ScrRoleSurfaces, {
			props: {
				orgName: 'Acme',
				tab: 'members',
				invite: s,
				viewers: [{ principalId: 'v1', name: 'The Boss', email: null, projects: [] }],
				viewerProjects: []
			}
		});
		await fireEvent.click(getByRole('tab', { name: 'Viewers' }));
		await tick();
		expect(getByText('The Boss')).toBeTruthy();
		await fireEvent.click(getByRole('button', { name: 'Invite' }));
		await tick();
		expect(s.open).toBe(true);
		expect(s.draft.access).toBe('viewer');
	});
});
