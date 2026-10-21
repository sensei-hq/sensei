// PUT / DELETE /v1/t/{origin}/{org}/viewers/{principalId}/projects/{projectId} —
// grant or revoke a person's read-only view of one of the dōjō's projects (#307
// A3, D8). ADMIN floor; the database function decides, with the caller's own JWT.
// A person not in the dōjō is a 409: invite them as a viewer first. 204 on success.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { grantProjectView, revokeProjectView, AdminError } from '$lib/server/viewers-data';

type Write = typeof grantProjectView;

const handle =
	(write: Write): RequestHandler =>
	async ({ params, request, locals }) => {
		try {
			const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.admin);
			await write(callerDb(caller), caller.tenantId, params.projectId!, params.principalId!);
			return new Response(null, { status: 204 });
		} catch (e) {
			if (e instanceof Response) return e;
			if (e instanceof AdminError) return apiError(e.status, e.message);
			throw e;
		}
	};

export const PUT = handle(grantProjectView);
export const DELETE = handle(revokeProjectView);
