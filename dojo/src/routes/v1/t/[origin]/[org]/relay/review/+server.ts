// POST /v1/t/{origin}/{org}/relay/review — the phone submits a batch of per-segment
// review verdicts (the PR-review "Send"). Supabase-JWT plane. Sets
// response_verdict / response_note / submitted_at on each reviewed segment, scoped
// to the run's session (segments carry no tenant_id — the session does, so we
// resolve run_id → session under the caller's tenant first). See relay-engine §6.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, membershipIdsForTenant, apiError, ACCESS } from '$lib/server/dojo-auth';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const runId = typeof body.run_id === 'string' ? body.run_id : '';
		const reviews = Array.isArray(body.reviews) ? (body.reviews as Record<string, unknown>[]) : null;
		if (!runId || !reviews) return apiError(400, 'run_id and reviews are required');
		const db = dojoDb();
		const membershipIds = await membershipIdsForTenant(db, tenantId);
		const { data: sess, error: sErr } = await db
			.from('relay_sessions')
			.select('id')
			.in('membership_id', membershipIds)
			.eq('run_id', runId)
			.maybeSingle();
		if (sErr) return apiError(500, sErr.message);
		if (!sess) return apiError(404, 'no relay session for run_id');
		const now = new Date().toISOString();
		let submitted = 0;
		for (const r of reviews) {
			const seq = typeof r.seq === 'number' ? r.seq : null;
			const verdict = typeof r.verdict === 'string' ? r.verdict : null;
			if (seq === null || !verdict) continue; // skip malformed entries
			const { error } = await db
				.from('relay_segments')
				.update({
					response_verdict: verdict,
					response_note: typeof r.note === 'string' ? r.note : null,
					submitted_at: now,
					updated_at: now
				})
				.eq('session_id', sess.id)
				.eq('seq', seq);
			if (error) return apiError(500, error.message);
			submitted++;
		}
		return Response.json({ submitted });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
