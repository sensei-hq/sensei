// PUT /v1/t/{origin}/{org}/teams/{teamId}/projects/{projectId} — grant everyone now
// on the team the read-only view of one of the dōjō's projects (admins; #307 A3,
// D7/D8). The grants are to people: they outlive the team.
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute, noContent } from '$lib/server/tenant-route';
import { grantTeamProjectView } from '$lib/server/teams-data';

export const PUT = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await grantTeamProjectView(db, caller.tenantId, params.teamId!, params.projectId!);
	return noContent();
});
