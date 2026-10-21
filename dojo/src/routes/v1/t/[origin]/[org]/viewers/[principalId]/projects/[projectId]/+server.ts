// PUT / DELETE /v1/t/{origin}/{org}/viewers/{principalId}/projects/{projectId} —
// grant or revoke a person's read-only view of one of the dōjō's projects (#307
// A3, D8). ADMIN floor; the database function decides, with the caller's own JWT.
// A person not in the dōjō is a 409: invite them as a viewer first. 204 on success.
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute, noContent } from '$lib/server/tenant-route';
import { grantProjectView, revokeProjectView } from '$lib/server/viewers-data';

export const PUT = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await grantProjectView(db, caller.tenantId, params.projectId!, params.principalId!);
	return noContent();
});

export const DELETE = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await revokeProjectView(db, caller.tenantId, params.projectId!, params.principalId!);
	return noContent();
});
