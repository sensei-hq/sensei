// POST /v1/t/{origin}/{org}/relay/nudge — the human sends an unsolicited nudge/chat
// TO the agent (direction human_to_agent): a NEW relay_inbox row for a run. Distinct
// from relay/reply, which answers an EXISTING agent-raised gate. Supabase-JWT plane
// (resolveTenantAccess). The daemon picks it up via poll_inbox and injects it into the
// held/live run through the hook control channel (see relay-engine §5 "control
// channel" + §6 "nudge — two modes").
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		if (!caller.membershipId) return apiError(403, 'membership required');
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const runId = typeof body.run_id === 'string' ? body.run_id : '';
		const text = typeof body.text === 'string' ? body.text.trim() : '';
		// Only chat/nudge are human-initiated (relay_inbox_kind); default to nudge.
		const kind = body.kind === 'chat' ? 'chat' : 'nudge';
		if (!runId || !text) return apiError(400, 'run_id and text are required');
		const db = dojoDb();
		// The nudge is keyed by run_id; map it to the cloud session row (tenant-scoped).
		const { data: sess, error: sErr } = await db
			.from('relay_sessions')
			.select('id')
			.eq('membership_id', caller.membershipId)
			.eq('run_id', runId)
			.maybeSingle();
		if (sErr) return apiError(500, sErr.message);
		if (!sess) return apiError(404, 'no relay session for run_id');
		const { data, error } = await db
			.from('relay_inbox')
			.insert({
				session_id: sess.id,
				membership_id: caller.membershipId,
				kind,
				direction: 'human_to_agent',
				payload: { text }
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
