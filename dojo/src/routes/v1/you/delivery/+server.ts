// POST /v1/you/delivery — one tracker board's facts, pushed by a contributor's
// daemon (spec docs/spec/daemon/tracker-delivery-metrics.md D3, D4; #352).
//
// The caller first; then the byte cap BEFORE the parse (an unbounded body is a
// memory profile the client would set); then the exact shape; then the database,
// as the caller, which re-decides consent rather than trusting the daemon's, finds
// the workspace and keeps one copy per board. Its refusals keep their status:
// 403 not the caller's to sync, 404 workspace_not_filed, 409 workspace_ambiguous.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { readCapped } from '$lib/server/body-cap';
import { DELIVERY_MAX_BYTES, checkDeliveryBody, fileDelivery } from '$lib/server/delivery-ingest';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const raw = await readCapped(request, DELIVERY_MAX_BYTES);
		if (raw === null) return apiError(413, `body over ${DELIVERY_MAX_BYTES} bytes`);
		let parsed: unknown;
		try {
			parsed = JSON.parse(raw);
		} catch {
			return apiError(400, 'body must be JSON');
		}
		const checked = checkDeliveryBody(parsed);
		if (!checked.ok) return apiError(checked.status, checked.message);
		return Response.json(await fileDelivery(callerDb(caller), checked.body));
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
