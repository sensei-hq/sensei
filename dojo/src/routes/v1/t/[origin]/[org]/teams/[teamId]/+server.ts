// PATCH  /v1/t/{origin}/{org}/teams/{teamId} {name} — rename a team (admins).
// DELETE /v1/t/{origin}/{org}/teams/{teamId} — delete it; the workspace views it
//        granted were granted to people, and stay (#307 A3, D7).
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute, nameFrom, noContent } from '$lib/server/tenant-route';
import { renameTeam, deleteTeam } from '$lib/server/teams-data';

export const PATCH = tenantRoute(ACCESS.admin, async (db, caller, { params, request }) => {
	await renameTeam(db, caller.tenantId, params.teamId!, await nameFrom(request));
	return noContent();
});

export const DELETE = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await deleteTeam(db, caller.tenantId, params.teamId!);
	return noContent();
});
