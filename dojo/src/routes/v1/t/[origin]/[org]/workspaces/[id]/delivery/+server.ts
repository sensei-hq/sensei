// GET /v1/t/{origin}/{org}/workspaces/{id}/delivery — the workspace's tracker
// boards and every sprint of each, from the boards' kept copies (spec
// docs/spec/daemon/tracker-delivery-metrics.md D4, D5; #352).
//
// Tenant-addressed, read as the caller: the caller first (an unknown org must not
// answer differently from a known one), then the id, then the tenant (service
// client), then the reads with the caller's own client — 403 not a reader of the
// workspace, 404 not this dōjō's.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveCaller, resolveTenantId, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { getWorkspaceDelivery } from '$lib/server/delivery-data';
import { isUuid } from '$lib/server/ids';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		if (!isUuid(params.id)) throw new AdminError(400, 'not a workspace id');
		const tenantId = await resolveTenantId(dojoDb(), params.origin, params.org);
		return Response.json(await getWorkspaceDelivery(callerDb(caller), tenantId, params.id));
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
