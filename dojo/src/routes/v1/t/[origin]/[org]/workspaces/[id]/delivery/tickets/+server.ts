// GET /v1/t/{origin}/{org}/workspaces/{id}/delivery/tickets?board=<id>[&sprint=<external id>|&backlog=1]
// — one board's tickets from its kept copy: one sprint's, the backlog's, or all,
// with their titles (spec tracker-delivery-metrics D4, D5; #352). The same order
// of checks as the boards route; a sprint and the backlog together is the
// caller's input to fix.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveCaller, resolveTenantId, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { getDeliveryTickets } from '$lib/server/delivery-data';
import { isUuid } from '$lib/server/ids';

export const GET: RequestHandler = async ({ params, request, locals, url }) => {
	try {
		const caller = await resolveCaller(request, locals);
		if (!isUuid(params.id)) throw new AdminError(400, 'not a workspace id');
		const board = url.searchParams.get('board');
		if (!isUuid(board)) throw new AdminError(400, 'board must be a board id');
		const sprint = url.searchParams.get('sprint') ?? undefined;
		const backlog = url.searchParams.get('backlog') === '1';
		if (sprint !== undefined && backlog) throw new AdminError(400, 'a sprint or the backlog, not both');
		const tenantId = await resolveTenantId(dojoDb(), params.origin, params.org);
		const tickets = await getDeliveryTickets(callerDb(caller), tenantId, params.id, board, { sprint, backlog });
		return Response.json({ tickets });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
