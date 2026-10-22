import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import TeamsList from './TeamsList.svelte';
import ScrRoleSurfaces from './ScrRoleSurfaces.svelte';
import { TeamDraft } from '$lib/teams-state.svelte';
import type { TeamView } from '$lib/teams-view';

// A dōjō's teams (#307 A3, D7): each team, its members (each removable), a picker
// of the dōjō's members not yet on it, a picker granting a workspace to everyone on
// it, rename and delete. A team grants no access; the copy says so.

const teams: TeamView[] = [
	{ id: 't1', name: 'Payments', members: [{ principalId: 'u1', name: 'Ada', email: null }] },
	{ id: 't2', name: 'Search', members: [] }
];
const members = [
	{ name: 'Ada', userId: 'u1', git: '', role: 'admin', scopes: '', active: '' },
	{ name: 'Dev', userId: 'u2', git: '', role: 'contributor', scopes: '', active: '' }
];
const workspaces = [{ id: 'p1', name: 'Ledger' }];

describe('TeamsList', () => {
	afterEach(cleanup);

	it('lists each team with its members', () => {
		const { getByText } = render(TeamsList, { props: { teams, members, workspaces, draft: new TeamDraft() } });
		expect(getByText('Payments')).toBeTruthy();
		expect(getByText('Ada')).toBeTruthy();
		expect(getByText('No one on this team yet')).toBeTruthy();
	});

	it('adds, removes, grants, renames and deletes', async () => {
		const on = {
			onAddMember: vi.fn(),
			onRemoveMember: vi.fn(),
			onGrant: vi.fn(),
			onDelete: vi.fn(),
			onSave: vi.fn().mockResolvedValue(true)
		};
		const draft = new TeamDraft();
		const { getByLabelText, getByRole } = render(TeamsList, { props: { teams, members, workspaces, draft, ...on } });
		const add = getByLabelText('Add a member to Payments') as HTMLSelectElement;
		expect([...add.options].map((o) => o.value)).toEqual(['', 'u2']);
		await fireEvent.change(add, { target: { value: 'u2' } });
		expect(on.onAddMember).toHaveBeenCalledWith(teams[0], 'u2');
		await fireEvent.click(getByRole('button', { name: 'Take Ada off Payments' }));
		expect(on.onRemoveMember).toHaveBeenCalledWith(teams[0], 'u1');
		await fireEvent.change(getByLabelText('Grant a workspace to everyone on Payments'), { target: { value: 'p1' } });
		expect(on.onGrant).toHaveBeenCalledWith(teams[0], 'p1');
		await fireEvent.click(getByRole('button', { name: 'Rename Payments' }));
		await tick();
		expect(draft.open).toBe(true);
		expect(draft.name).toBe('Payments');
		await fireEvent.click(getByRole('button', { name: 'Delete Search' }));
		expect(on.onDelete).toHaveBeenCalledWith(teams[1]);
	});

	it('the open draft shows a name field and saves through onSave', async () => {
		const onSave = vi.fn().mockResolvedValue(true);
		const draft = new TeamDraft();
		draft.start();
		const { getByLabelText, getByRole } = render(TeamsList, { props: { teams: [], members, workspaces, draft, onSave } });
		await fireEvent.input(getByLabelText('Team name'), { target: { value: 'Platform' } });
		await fireEvent.click(getByRole('button', { name: 'Save team' }));
		expect(onSave).toHaveBeenCalledWith(null, 'Platform');
	});

	it('says when there are no teams', () => {
		const { getByText } = render(TeamsList, { props: { teams: [], members, workspaces, draft: new TeamDraft() } });
		expect(getByText(/No teams yet/)).toBeTruthy();
	});
});

describe('ScrRoleSurfaces — the Teams tab', () => {
	afterEach(cleanup);

	it("lists the dōjō's teams, and its New team button opens a blank draft", async () => {
		const teamDraft = new TeamDraft();
		const { getByRole, getByText } = render(ScrRoleSurfaces, {
			props: { orgName: 'Acme', tab: 'members', members, teams, teamDraft, viewerWorkspaces: workspaces }
		});
		await fireEvent.click(getByRole('tab', { name: 'Teams' }));
		await tick();
		expect(getByText('Payments')).toBeTruthy();
		await fireEvent.click(getByRole('button', { name: 'New team' }));
		await tick();
		expect(teamDraft.open).toBe(true);
		expect(teamDraft.teamId).toBeNull();
	});
});
