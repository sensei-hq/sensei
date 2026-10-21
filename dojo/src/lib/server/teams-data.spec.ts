// A dōjō's teams (#307 A3, D7): named groups of its members, for "by team"
// reports and for granting a project's view to many people at once. A team
// grants no access. The admin lists, creates, renames and deletes teams, puts
// members on them, and grants a project to a whole team. Every write goes
// through the database's functions; a refusal keeps its status.
import { describe, it, expect } from 'vitest';
import { fakeDojoDb, type FakeTable } from './fake-dojo-db';
import {
	listTeams,
	createTeam,
	renameTeam,
	deleteTeam,
	addTeamMember,
	removeTeamMember,
	grantTeamProjectView
} from './teams-data';
import { AdminError, type DojoClient } from './admin-data';

const T = 'tenant-t';

function db(over: Record<string, FakeTable> = {}) {
	const f = fakeDojoDb({
		teams: {
			rows: [
				{ id: 'tm2', tenant_id: T, name: 'Search', created_at: '2' },
				{ id: 'tm1', tenant_id: T, name: 'Payments', created_at: '1' },
				{ id: 'tmx', tenant_id: 'other', name: 'Not ours', created_at: '3' }
			]
		},
		team_members: {
			rows: [
				{ team_id: 'tm1', principal_id: 'ada' },
				{ team_id: 'tm1', principal_id: 'dev' },
				{ team_id: 'tmx', principal_id: 'ada' }
			]
		},
		memberships: {
			rows: [
				{ tenant_id: T, user_id: 'ada' },
				{ tenant_id: T, user_id: 'dev' }
			]
		},
		identities: {
			rows: [{ principal_id: 'ada', display_name: 'Ada', email: 'ada@x.co', last_login_at: null }]
		},
		projects: { rows: [{ id: 'p1', tenant_id: T }, { id: 'px', tenant_id: 'other' }] },
		'rpc:create_team': { rows: [{ id: 'new-team' }] },
		'rpc:rename_team': { rows: [] },
		'rpc:delete_team': { rows: [] },
		'rpc:add_team_member': { rows: [] },
		'rpc:remove_team_member': { rows: [] },
		'rpc:grant_project_viewer_to_team': { rows: [] },
		...over
	});
	return f as unknown as DojoClient & typeof f;
}

describe('listTeams', () => {
	it("lists the dōjō's teams by name, each with its members named", async () => {
		expect(await listTeams(db(), T)).toEqual([
			{
				id: 'tm1',
				name: 'Payments',
				members: [
					{ principal_id: 'ada', display_name: 'Ada', email: 'ada@x.co' },
					{ principal_id: 'dev', display_name: null, email: null }
				]
			},
			{ id: 'tm2', name: 'Search', members: [] }
		]);
	});

	it('a failed read is an error, never an empty list', async () => {
		await expect(listTeams(db({ teams: { rows: [], error: { message: 'boom' } } }), T)).rejects.toThrow(AdminError);
		await expect(listTeams(db({ team_members: { rows: [], error: { message: 'boom' } } }), T)).rejects.toThrow(AdminError);
	});
});

describe('team writes', () => {
	it('create names the dōjō and the trimmed name; an empty name is a 400', async () => {
		const d = db();
		await createTeam(d, T, '  Platform ');
		expect(d.rpcCalls).toEqual([{ fn: 'create_team', args: { p_tenant: T, p_name: 'Platform' } }]);
		await expect(createTeam(d, T, '   ')).rejects.toMatchObject({ status: 400 });
	});

	it("rename, delete and membership act on this dōjō's teams only", async () => {
		const d = db();
		await renameTeam(d, T, 'tm1', 'Payments & Billing');
		await addTeamMember(d, T, 'tm1', 'dev');
		await removeTeamMember(d, T, 'tm1', 'dev');
		await grantTeamProjectView(d, T, 'tm1', 'p1');
		await deleteTeam(d, T, 'tm1');
		expect(d.rpcCalls.map((c) => [c.fn, c.args])).toEqual([
			['rename_team', { p_team: 'tm1', p_name: 'Payments & Billing' }],
			['add_team_member', { p_team: 'tm1', p_principal: 'dev' }],
			['remove_team_member', { p_team: 'tm1', p_principal: 'dev' }],
			['grant_project_viewer_to_team', { p_project: 'p1', p_team: 'tm1' }],
			['delete_team', { p_team: 'tm1' }]
		]);
		await expect(deleteTeam(d, T, 'tmx')).rejects.toMatchObject({ status: 404 });
		await expect(grantTeamProjectView(d, T, 'tm1', 'px')).rejects.toMatchObject({ status: 404 });
	});

	it('keeps the status the database meant', async () => {
		const refused = (fn: string, code: string) => db({ [`rpc:${fn}`]: { rows: [], error: { code, message: 'no' } } });
		await expect(createTeam(refused('create_team', '23505'), T, 'Payments')).rejects.toMatchObject({
			status: 409,
			message: 'this dōjō already has a team named Payments'
		});
		await expect(renameTeam(refused('rename_team', '23505'), T, 'tm1', 'Search')).rejects.toMatchObject({
			status: 409,
			message: 'this dōjō already has a team named Search'
		});
		await expect(addTeamMember(refused('add_team_member', '23503'), T, 'tm1', 'out')).rejects.toMatchObject({ status: 409 });
		await expect(renameTeam(refused('rename_team', '42501'), T, 'tm1', 'x')).rejects.toMatchObject({ status: 403 });
	});
});
