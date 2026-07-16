// POST /v1/t/{origin}/{org}/relay/reply — the phone/console answers a gate.
// Supabase-JWT plane (a signed-in human). Sets reply + status='answered'; the
// dojo.relay_inbox_bump_seq trigger advances seq on this UPDATE, so the daemon's
// `?since=<cursor>` poll re-surfaces the now-answered row and consumes the reply.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const inboxId = typeof body.inbox_id === 'string' ? body.inbox_id : '';
		if (!inboxId) return apiError(400, 'inbox_id is required');
		if (body.reply === undefined) return apiError(400, 'reply is required');
		const { data, error } = await dojoDb()
			.from('relay_inbox')
			.update({
				reply: body.reply,
				status: 'answered',
				answered_at: new Date().toISOString()
			})
			.eq('id', inboxId)
			.eq('tenant_id', tenantId)
			.eq('status', 'pending') // only a pending gate can be answered (idempotent)
			.select('id, seq')
			.maybeSingle();
		if (error) return apiError(500, error.message);
		if (!data) return apiError(404, 'no pending inbox row for id');
		return Response.json({ id: data.id, seq: data.seq });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
