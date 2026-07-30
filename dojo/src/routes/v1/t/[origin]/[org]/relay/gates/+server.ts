// GET /v1/t/{origin}/{org}/relay/gates — the phone's "needs you" view: PENDING
// gate/decision rows awaiting the human, Supabase-JWT plane (resolveTenantAccess),
// scoped to the caller's tenant. Distinct from relay/inbox GET, which is the DAEMON
// plane (API-key + ?since cursor). relay_inbox is keyed by session_id (no run_id), so
// we resolve run_id/title from the session for deep-linking. Reply via relay/reply.
// See docs/plan/relay-engine.md §6.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, membershipIdsForTenant, apiError, ACCESS } from '$lib/server/dojo-auth';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.member
		);
		const db = dojoDb();
		const membershipIds = await membershipIdsForTenant(db, tenantId);
		// Pending items the agent is waiting on the human for, oldest first.
		const { data: rows, error } = await db
			.from('relay_inbox')
			.select('id, seq, session_id, segment_id, kind, payload, created_at')
			.in('membership_id', membershipIds)
			.eq('status', 'pending')
			.eq('direction', 'agent_to_human')
			.order('seq', { ascending: true });
		if (error) return apiError(500, error.message);
		const inbox = rows ?? [];

		// Resolve run_id/title per row from its session (relay_inbox has no run_id).
		const sessionIds = [...new Set(inbox.map((r) => r.session_id as string))];
		const sessById = new Map<string, { run_id: string; title: string | null }>();
		if (sessionIds.length) {
			const { data: sess, error: sErr } = await db
				.from('relay_sessions')
				.select('id, run_id, title')
				.in('id', sessionIds);
			if (sErr) return apiError(500, sErr.message);
			for (const s of sess ?? []) sessById.set(s.id as string, { run_id: s.run_id, title: s.title });
		}

		const gates = inbox.map((r) => {
			const s = sessById.get(r.session_id as string);
			return {
				id: r.id,
				seq: r.seq,
				run_id: s?.run_id ?? null,
				run_title: s?.title ?? null,
				segment_id: r.segment_id,
				kind: r.kind,
				payload: r.payload ?? {},
				created_at: r.created_at
			};
		});
		return Response.json({ gates });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
