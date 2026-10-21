// PUT /v1/t/{origin}/{org}/teams/{teamId}/workspaces/{workspaceId} — grant everyone now
// on the team the read-only view of one of the dōjō's workspaces (admins; #307 A3,
// D7/D8). The grants are to people: they outlive the team.
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute, noContent } from '$lib/server/tenant-route';
import { grantTeamWorkspaceView } from '$lib/server/teams-data';

export const PUT = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await grantTeamWorkspaceView(db, caller.tenantId, params.teamId!, params.workspaceId!);
	return noContent();
});
