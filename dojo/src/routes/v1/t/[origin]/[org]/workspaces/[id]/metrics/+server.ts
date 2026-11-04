// GET /v1/t/{origin}/{org}/workspaces/{id}/metrics[?days=N] — the workspace's
// metric series, day by day over the last N days (default 30, at most 90).
//
// Tenant-addressed but NOT tenant-floored: a workspace viewer reads this
// without a membership, so no member floor can admit them. The tenant comes
// from the path (404 when unknown), the caller from the JWT, and the database
// decides the rest — can_read_workspace gates (403), the rollup answers per
// row, and the read runs with the caller's own client throughout.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveCaller, resolveTenantId, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { getWorkspaceMetrics } from '$lib/server/workspaces-data';
import { windowDays } from '$lib/server/window-days';

/** The window when `?days=` is absent — a month of daily points. */
const DEFAULT_DAYS = 30;

/** A workspace id is a uuid; anything else is the caller's input to fix (400),
 *  never a 500 from can_read_workspace's 22P02. */
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export const GET: RequestHandler = async ({ params, request, locals, url }) => {
	try {
		// Order: the caller first — resolving the tenant first would answer
		// 404 for an unknown org and 401 for a real one, so anyone could probe
		// which dōjō slugs exist (the house pattern in resolveTenantAccess
		// authenticates first). Then everything the caller sent (a malformed
		// id, a bad window), then the tenant (system work, service client),
		// then the read as the caller.
		const caller = await resolveCaller(request, locals);
		if (!UUID_RE.test(params.id)) throw new AdminError(400, 'not a workspace id');
		const days = windowDays(url, DEFAULT_DAYS);
		const tenantId = await resolveTenantId(dojoDb(), params.origin, params.org);
		return Response.json(
			await getWorkspaceMetrics(callerDb(caller), tenantId, params.id, { days, today: new Date() })
		);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
