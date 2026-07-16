// POST/GET /v1/t/{origin}/{org}/relay/inbox — the daemon raises gate/decision/
// chat/nudge/stall rows and polls them back by a monotonic seq cursor. Both the
// daemon plane (API-key). The seq trigger (dojo.relay_inbox_bump_seq) re-surfaces
// a row whenever it's answered. See docs/plan/relay-engine.md §4/§6.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

const COLS =
	'id, seq, session_id, segment_id, kind, direction, status, payload, reply, created_at, answered_at';

const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);

// Daemon → raise an inbox row for a run (gate/decision/chat/nudge/stall).
export const POST: RequestHandler = async ({ params, request }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const runId = str(body.run_id);
		const kind = str(body.kind);
		if (!runId || !kind) return apiError(400, 'run_id and kind are required');
		const db = dojoDb();
		// The wire item is keyed by run_id; map it to the cloud session row.
		const { data: sess, error: sErr } = await db
			.from('relay_sessions')
			.select('id')
			.eq('tenant_id', caller.tenantId)
			.eq('run_id', runId)
			.maybeSingle();
		if (sErr) return apiError(500, sErr.message);
		if (!sess) return apiError(404, 'no relay session for run_id (POST relay/session first)');
		const { data, error } = await db
			.from('relay_inbox')
			.insert({
				session_id: sess.id,
				segment_id: str(body.segment_id),
				tenant_id: caller.tenantId,
				membership_id: caller.membershipId,
				user_id: caller.userId,
				kind,
				direction: str(body.direction) ?? 'agent_to_human',
				payload: body.payload ?? {}
			})
			.select('id, seq')
			.single();
		if (error) return apiError(500, error.message);
		return Response.json({ id: data.id, seq: data.seq });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};

// Daemon → poll rows with seq > since (answered replies + new items), oldest first.
export const GET: RequestHandler = async ({ params, request, url }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const since = Number(url.searchParams.get('since') ?? '0') || 0;
		const { data, error } = await dojoDb()
			.from('relay_inbox')
			.select(COLS)
			.eq('tenant_id', caller.tenantId)
			.gt('seq', since)
			.order('seq', { ascending: true });
		if (error) return apiError(500, error.message);
		const items = data ?? [];
		const cursor = items.reduce((m, r) => Math.max(m, Number(r.seq)), since);
		return Response.json({ items, cursor });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
