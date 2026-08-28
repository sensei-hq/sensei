// POST /v1/you/metrics — receive metric rows pushed by the daemon.
//
// The third leg of the user-plane cycle: /v1/you/repositories answers WHICH
// TENANT, /v1/you/sync/plan answers WHAT MAY SYNC, and this one takes the rows.
//
// Entitlement is re-decided here per row rather than trusted from the plan — see
// `metrics-ingest.ts`. The plan spares the daemon shipping data that would be
// refused; it is not the boundary.
//
// Partial acceptance is the designed outcome, not a fallback: a batch with one
// bad row stores the rest and names what it refused. The alternative — 4xx on the
// whole batch — would let one unmappable repository block a machine's entire
// history indefinitely.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { ingestMetrics, type MetricInput } from '$lib/server/metrics-ingest';

/** Cap on one batch. The daemon pages; an unbounded body is a memory profile
 *  set by the client, which is not a decision a client gets to make. */
const MAX_ROWS = 1000;

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, db } = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => null)) as { metrics?: unknown } | null;
		const rows = body?.metrics;
		if (!Array.isArray(rows)) {
			return apiError(400, 'body must be { metrics: [...] }');
		}
		if (rows.length > MAX_ROWS) {
			return apiError(413, `at most ${MAX_ROWS} metrics per request, got ${rows.length}`);
		}
		return Response.json(await ingestMetrics(db, userId, rows as MetricInput[]));
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
