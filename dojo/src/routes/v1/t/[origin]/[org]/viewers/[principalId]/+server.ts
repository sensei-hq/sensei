// DELETE /v1/t/{origin}/{org}/viewers/{principalId} — remove a viewer from the
// dōjō, and their project views here (#307 A3, D8). ADMIN floor; the database
// function decides, with the caller's own JWT. 204 on success.
import { ACCESS } from '$lib/server/dojo-auth';
import { tenantRoute, noContent } from '$lib/server/tenant-route';
import { removeDojoViewer } from '$lib/server/viewers-data';

export const DELETE = tenantRoute(ACCESS.admin, async (db, caller, { params }) => {
	await removeDojoViewer(db, caller.tenantId, params.principalId!);
	return noContent();
});
