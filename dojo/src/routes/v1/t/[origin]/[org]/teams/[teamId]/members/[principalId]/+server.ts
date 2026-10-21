// PUT / DELETE /v1/t/{origin}/{org}/teams/{teamId}/members/{principalId} — put a
// member of the dōjō on a team, or take them off (admins; #307 A3, D7). Someone
// not in the dōjō is a 409.
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute, noContent } from '$lib/server/tenant-route';
import { addTeamMember, removeTeamMember } from '$lib/server/teams-data';

export const PUT = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await addTeamMember(db, caller.tenantId, params.teamId!, params.principalId!);
	return noContent();
});

export const DELETE = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await removeTeamMember(db, caller.tenantId, params.teamId!, params.principalId!);
	return noContent();
});
