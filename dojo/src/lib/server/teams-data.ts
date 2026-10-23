// A dōjō's teams (#307 A3, D7) — named groups of its members, for "by team"
// reports and for granting a workspace's read-only view to many people at once. A
// team grants no access; repository roles do. Admins manage them through the
// database's own functions (which decide who may), and a refusal keeps the status
// the database meant. Fails closed: a failed read is an error, never an empty list.
import { AdminError, type DojoClient } from './admin-data';
import { resolveDisplayNames } from './identity-resolve';
import { callFn, requireInTenant } from './tenant-rows';

export { AdminError };

export interface TeamMember {
	principal_id: string;
	display_name: string | null;
	email: string | null;
}

export interface Team {
	id: string;
	name: string;
	members: TeamMember[];
}

/** The dōjō's teams by name, each with its members named. The teams and their
 *  members are read as `db` (the caller); the names through `nameDb` — the service
 *  client when the caller may read the ids but not the identities (the access
 *  matrix's `resolveDisplayNames` exception: names for ids the caller already read). */
export async function listTeams(db: DojoClient, tenantId: string, nameDb: DojoClient = db): Promise<Team[]> {
	const teams = await db.from('teams').select('id, name').eq('tenant_id', tenantId);
	if (teams.error) throw new AdminError(500, teams.error.message);
	const rows = ((teams.data ?? []) as Array<{ id: string; name: string }>).sort((a, b) => a.name.localeCompare(b.name));
	if (rows.length === 0) return [];

	const members = await db
		.from('team_members')
		.select('team_id, principal_id')
		.in(
			'team_id',
			rows.map((t) => t.id)
		);
	if (members.error) throw new AdminError(500, members.error.message);
	const links = (members.data ?? []) as Array<{ team_id: string; principal_id: string }>;
	const names = await resolveDisplayNames(
		nameDb,
		tenantId,
		links.map((l) => l.principal_id)
	);

	return rows.map((t) => ({
		id: t.id,
		name: t.name,
		members: links
			.filter((l) => l.team_id === t.id)
			.map((l) => {
				const n = names.get(l.principal_id);
				return { principal_id: l.principal_id, display_name: n?.display_name ?? null, email: n?.email ?? null };
			})
	}));
}

const teamName = (name: string): string => {
	const trimmed = name.trim();
	if (!trimmed) throw new AdminError(400, 'a team needs a name');
	return trimmed;
};

/** A name the dōjō already uses is said in words, not as the constraint's name. */
async function named(name: string, write: () => Promise<unknown>): Promise<void> {
	try {
		await write();
	} catch (e) {
		if (e instanceof AdminError && e.status === 409) {
			throw new AdminError(409, `this dōjō already has a team named ${name}`);
		}
		throw e;
	}
}

const requireTeam = (db: DojoClient, tenantId: string, teamId: string) =>
	requireInTenant(db, 'teams', tenantId, teamId, 'team');

/** Create a team in the dōjō. */
export async function createTeam(db: DojoClient, tenantId: string, name: string): Promise<void> {
	const p_name = teamName(name);
	await named(p_name, () => callFn(db, 'create_team', { p_tenant: tenantId, p_name }));
}

export async function renameTeam(db: DojoClient, tenantId: string, teamId: string, name: string): Promise<void> {
	const p_name = teamName(name);
	await requireTeam(db, tenantId, teamId);
	await named(p_name, () => callFn(db, 'rename_team', { p_team: teamId, p_name }));
}

/** Delete a team. The workspace views it granted were granted to people, and stay. */
export async function deleteTeam(db: DojoClient, tenantId: string, teamId: string): Promise<void> {
	await requireTeam(db, tenantId, teamId);
	await callFn(db, 'delete_team', { p_team: teamId });
}

/** Put a member of the dōjō on a team. */
export async function addTeamMember(db: DojoClient, tenantId: string, teamId: string, principalId: string): Promise<void> {
	await requireTeam(db, tenantId, teamId);
	await callFn(db, 'add_team_member', { p_team: teamId, p_principal: principalId });
}

export async function removeTeamMember(
	db: DojoClient,
	tenantId: string,
	teamId: string,
	principalId: string
): Promise<void> {
	await requireTeam(db, tenantId, teamId);
	await callFn(db, 'remove_team_member', { p_team: teamId, p_principal: principalId });
}

/** Grant everyone now on the team the view of one of the dōjō's workspaces. */
export async function grantTeamWorkspaceView(
	db: DojoClient,
	tenantId: string,
	teamId: string,
	workspaceId: string
): Promise<void> {
	await requireTeam(db, tenantId, teamId);
	await requireInTenant(db, 'workspaces', tenantId, workspaceId, 'workspace');
	await callFn(db, 'grant_workspace_viewer_to_team', { p_workspace: workspaceId, p_team: teamId });
}
