// POST /v1/t/{origin}/{org}/relay/reply — the phone/console answers a gate.
// Supabase-JWT plane (a signed-in human). Sets reply + status='answered'; the
// dojo.relay_inbox_bump_seq trigger advances seq on this UPDATE, so the daemon's
// `?since=<cursor>` poll re-surfaces the now-answered row and consumes the reply.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, membershipIdsForTenant, apiError, ACCESS } from '$lib/server/dojo-auth';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const inboxId = typeof body.inbox_id === 'string' ? body.inbox_id : '';
		if (!inboxId) return apiError(400, 'inbox_id is required');
		if (body.reply === undefined) return apiError(400, 'reply is required');
		const db = dojoDb();
		const membershipIds = await membershipIdsForTenant(db, tenantId);
		const { data, error } = await db
			.from('relay_inbox')
			.update({
				reply: body.reply,
				status: 'answered',
				answered_at: new Date().toISOString()
			})
			.eq('id', inboxId)
			.in('membership_id', membershipIds)
			// A reply answers an agent-raised gate only — never a human_to_agent
			// nudge/chat row (those aren't awaiting a human answer). Scoped by the
			// tenant's memberships, so this can't touch another tenant's rows.
			.eq('direction', 'agent_to_human')
			.select('id, seq')
			.maybeSingle();
		// Idempotent: re-answering an already-answered gate is a no-harm overwrite
		// (last-write-wins) — safe under network retries and phone double-taps. The
		// seq trigger re-advances on each write so the daemon poll always re-surfaces it.
		if (error) return apiError(500, error.message);
		if (!data) return apiError(404, 'no inbox row for id');
		return Response.json({ id: data.id, seq: data.seq });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
