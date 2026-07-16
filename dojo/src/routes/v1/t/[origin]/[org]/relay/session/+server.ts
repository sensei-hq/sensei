// GET/POST /v1/t/{origin}/{org}/relay/session — a run's filtered status snapshot.
// POST = the daemon publishing status (API-key plane, upsert by run_id); GET = the
// phone/console reading it (Supabase-JWT plane). See docs/plan/relay-engine.md §6.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

const COLS =
	'id, run_id, title, goal, status, progress_done, progress_total, current_phase, current_feature, last_event_at, paused_until, pause_reason, started_at, completed_at';

const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);
const int = (v: unknown): number => (typeof v === 'number' && Number.isFinite(v) ? v : 0);

// Daemon → publish the run's filtered status (upsert on tenant_id+run_id).
export const POST: RequestHandler = async ({ params, request }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const runId = str(body.run_id);
		if (!runId) return apiError(400, 'run_id is required');
		const { data, error } = await dojoDb()
			.from('relay_sessions')
			.upsert(
				{
					tenant_id: caller.tenantId,
					membership_id: caller.membershipId,
					user_id: caller.userId,
					run_id: runId,
					title: str(body.title) ?? '',
					goal: str(body.goal),
					status: str(body.status) ?? 'running',
					progress_done: int(body.progress_done),
					progress_total: int(body.progress_total),
					current_phase: str(body.current_phase),
					current_feature: str(body.current_feature),
					last_event_at: str(body.last_event_at),
					paused_until: str(body.paused_until),
					pause_reason: str(body.pause_reason),
					updated_at: new Date().toISOString()
				},
				{ onConflict: 'tenant_id,run_id' }
			)
			.select('id')
			.single();
		if (error) return apiError(500, error.message);
		return Response.json({ id: data.id });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};

// Phone/console → read a run's status (optionally filtered by run_id).
export const GET: RequestHandler = async ({ params, request, locals, url }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		let q = dojoDb()
			.from('relay_sessions')
			.select(COLS)
			.eq('tenant_id', tenantId)
			.order('started_at', { ascending: false });
		const runId = url.searchParams.get('run_id');
		if (runId) q = q.eq('run_id', runId);
		const { data, error } = await q;
		if (error) return apiError(500, error.message);
		return Response.json({ sessions: data ?? [] });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
