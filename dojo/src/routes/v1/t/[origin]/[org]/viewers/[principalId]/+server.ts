// DELETE /v1/t/{origin}/{org}/viewers/{principalId} — remove a viewer from the
// dōjō, and their project views here (#307 A3, D8). ADMIN floor; the database
// function decides, with the caller's own JWT. 204 on success.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { removeDojoViewer, AdminError } from '$lib/server/viewers-data';

export const DELETE: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.admin);
		await removeDojoViewer(callerDb(caller), caller.tenantId, params.principalId);
		return new Response(null, { status: 204 });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
