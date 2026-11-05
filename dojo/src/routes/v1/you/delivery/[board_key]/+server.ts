// DELETE /v1/you/delivery/{board_key}?tenant_id=&source= — a contributor's daemon
// withdraws what its install reported for a board (spec
// docs/spec/daemon/tracker-delivery-metrics.md D3 "Withdrawal", B13; #355).
//
// The caller first; then the key, tenant and install, checked before the database;
// then the database, as the caller, which removes only the caller's own rows.
// 204 when something was withdrawn, 404 when nothing of the caller's was there —
// the daemon treats both as done.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { isUuid } from '$lib/server/ids';
import { withdrawDelivery } from '$lib/server/delivery-ingest';

const BOARD_KEY_RE = /^[0-9a-f]{64}$/;

export const DELETE: RequestHandler = async ({ request, url, params, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const tenantId = url.searchParams.get('tenant_id');
		const source = url.searchParams.get('source');
		if (!BOARD_KEY_RE.test(params.board_key)) return apiError(400, 'board_key must be 64 lowercase hex characters');
		if (!isUuid(tenantId) || !isUuid(source)) return apiError(400, 'tenant_id and source must be uuids');
		const removed = await withdrawDelivery(callerDb(caller), tenantId, params.board_key, source);
		// The 404 says so in its own words, so a daemon never takes another 404 (a dōjō
		// without this route, a proxy) for "nothing there".
		return removed ? new Response(null, { status: 204 }) : Response.json({ withdrawn: false }, { status: 404 });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
