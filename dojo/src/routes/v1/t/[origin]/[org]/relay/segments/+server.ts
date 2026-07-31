// GET/POST /v1/t/{origin}/{org}/relay/segments — the run's outline
// (Execution → Segment → Item). POST = the daemon publishing the outline
// (device-token plane, upsert by session+seq); GET = the phone reading it
// (Supabase-JWT plane). The per-segment review verdicts are phone-owned and NOT
// touched by the daemon publish. See docs/plan/relay-engine.md §6.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, resolveTenantAccess, membershipIdsForTenant, apiError, ACCESS } from '$lib/server/dojo-auth';

const COLS =
	'id, session_id, parent_id, seq, title, summary, detail, agent, model, spec_ref, state, is_gate, gate_severity, response_verdict, response_note, submitted_at';

const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);

// Daemon → publish the run's outline. Upserts each segment by (session_id, seq);
// only daemon-owned fields are written, so a re-publish never clobbers the phone's
// per-segment review verdict/note (those columns are absent from the payload).
export const POST: RequestHandler = async ({ params, request }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const runId = str(body.run_id);
		const segments = Array.isArray(body.segments) ? (body.segments as Record<string, unknown>[]) : null;
		if (!runId || !segments) return apiError(400, 'run_id and segments are required');
		const db = dojoDb();
		const { data: sess, error: sErr } = await db
			.from('relay_sessions')
			.select('id')
			.eq('membership_id', caller.membershipId)
			.eq('run_id', runId)
			.maybeSingle();
		if (sErr) return apiError(500, sErr.message);
		if (!sess) return apiError(404, 'no relay session for run_id (POST relay/session first)');
		// parent_id is NOT taken from the payload — the daemon can't know a phase's
		// server id (assigned here on upsert), so it carries the child's parent phase
		// as `parent_seq`. We omit parent_id from the upsert (so a re-publish never
		// clobbers the resolved nesting) and resolve it below.
		const rows = segments.map((s) => ({
			session_id: sess.id,
			seq: typeof s.seq === 'number' ? s.seq : 0,
			title: str(s.title) ?? '',
			summary: str(s.summary),
			detail: str(s.detail),
			agent: str(s.agent),
			model: str(s.model),
			spec_ref: str(s.spec_ref),
			state: str(s.state) ?? 'pending',
			is_gate: s.is_gate === true,
			gate_severity: str(s.gate_severity),
			updated_at: new Date().toISOString()
		}));
		const { error } = await db.from('relay_segments').upsert(rows, { onConflict: 'session_id,seq' });
		if (error) return apiError(500, error.message);

		// Nesting: now every (session_id, seq) row has an id, resolve parent_id from the
		// daemon's parent_seq (phase seq → its id). Idempotent across re-publishes.
		const parentSeqBySeq = new Map<number, number>();
		for (const s of segments) {
			const seq = typeof s.seq === 'number' ? s.seq : 0;
			if (typeof s.parent_seq === 'number') parentSeqBySeq.set(seq, s.parent_seq);
		}
		if (parentSeqBySeq.size) {
			const { data: existing, error: exErr } = await db
				.from('relay_segments')
				.select('id, seq')
				.eq('session_id', sess.id);
			// Fail CLOSED: a read failure here would silently drop all parent nesting
			// yet still report success — surface it instead.
			if (exErr) return apiError(500, exErr.message);
			const idBySeq = new Map((existing ?? []).map((e) => [e.seq as number, e.id as string]));
			for (const [seq, parentSeq] of parentSeqBySeq) {
				const parentId = idBySeq.get(parentSeq);
				if (parentId) {
					await db
						.from('relay_segments')
						.update({ parent_id: parentId })
						.eq('session_id', sess.id)
						.eq('seq', seq);
				}
			}
		}
		return Response.json({ upserted: rows.length });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};

// Phone → read a run's outline, ordered by seq.
export const GET: RequestHandler = async ({ params, request, locals, url }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		const runId = url.searchParams.get('run_id');
		if (!runId) return apiError(400, 'run_id query param is required');
		const db = dojoDb();
		const membershipIds = await membershipIdsForTenant(db, tenantId);
		const { data: sess, error: sErr } = await db
			.from('relay_sessions')
			.select('id')
			.in('membership_id', membershipIds)
			.eq('run_id', runId)
			.maybeSingle();
		// Fail CLOSED on a lookup error; only a genuine miss (no session published
		// yet for this run) is honest-empty.
		if (sErr) return apiError(500, sErr.message);
		if (!sess) return Response.json({ segments: [] });
		const { data, error } = await db
			.from('relay_segments')
			.select(COLS)
			.eq('session_id', sess.id)
			.order('seq', { ascending: true });
		if (error) return apiError(500, error.message);
		return Response.json({ segments: data ?? [] });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
