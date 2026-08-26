// POST/GET /v1/t/{origin}/{org}/relay/inbox — the daemon raises gate/decision/
// chat/nudge/stall rows and polls them back by a monotonic seq cursor. Both the
// daemon plane (API-key). The seq trigger (dojo.relay_inbox_bump_seq) re-surfaces
// a row whenever it's answered. See docs/plan/relay-engine.md §4/§6.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { sendRelayPushFromEnv } from '$lib/server/relay-push-env';
import type { RelayInboxKind } from '$lib/server/relay-push-send';

const COLS =
	'id, seq, session_id, segment_id, kind, direction, status, payload, reply, created_at, answered_at';

const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);

// The inbox kinds that are a "needs-you" gate the human should be pushed for.
// chat / nudge are free-form or human-initiated — never a push. A crash is a run
// STATUS (relay/session), not an inbox kind, so it's pushed from that route.
const PUSH_KINDS = new Set<RelayInboxKind>(['approval', 'decision', 'stall']);

// Daemon → raise an inbox row for a run (gate/decision/chat/nudge/stall).
export const POST: RequestHandler = async ({ params, request, platform }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const runId = str(body.run_id);
		const kind = str(body.kind);
		if (!runId || !kind) return apiError(400, 'run_id and kind are required');
		const db = dojoDb();
		// The wire item is keyed by run_id; map it to the cloud session row. `title`
		// is fetched now so the zero-knowledge push can label "…on <run title>".
		const { data: sess, error: sErr } = await db
			.from('relay_sessions')
			.select('id, title')
			.eq('membership_id', caller.membershipId)
			.eq('run_id', runId)
			.maybeSingle();
		if (sErr) return apiError(500, sErr.message);
		if (!sess) return apiError(404, 'no relay session for run_id (POST relay/session first)');
		const direction = str(body.direction) ?? 'agent_to_human';
		const { data, error } = await db
			.from('relay_inbox')
			.insert({
				session_id: sess.id,
				segment_id: str(body.segment_id),
				membership_id: caller.membershipId,
				kind,
				direction,
				payload: body.payload ?? {}
			})
			.select('id, seq')
			.single();
		if (error) return apiError(500, error.message);

		// P4.4 — on a raised "needs-you" gate, push the human's backgrounded devices.
		// Fire-and-forget via waitUntil: the send runs after this response and NEVER
		// blocks or fails the daemon's raise (fail-open — the send swallows its own
		// errors). Only agent_to_human approval/decision/stall push (not chat/nudge,
		// not a human_to_agent nudge). Prefs gating + dedup + expired-disable live in
		// the send module. Absent `platform` (local dev / vitest) → the guard skips.
		if (direction === 'agent_to_human' && PUSH_KINDS.has(kind as RelayInboxKind)) {
			const push = sendRelayPushFromEnv({
				userId: caller.userId,
				tenantId: caller.tenantId,
				runId,
				runTitle: (sess.title as string | null) ?? null,
				signal: { type: 'inbox', kind: kind as RelayInboxKind }
			});
			if (platform?.context?.waitUntil) {
				platform.context.waitUntil(push);
			} else {
				// No waitUntil (non-Worker runtime): still fire-and-forget, but keep the
				// rejection from becoming an unhandled promise.
				push.catch(() => {});
			}
		}

		return Response.json({ id: data.id, seq: data.seq });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};

// Daemon → either ONE row by id, or a cursor scan.
//
// `?id=` is the shape a waiting gate actually needs: the daemon raised that gate,
// holds its id, and is asking "has it been answered yet". A cursor is the wrong
// abstraction for request/response — and an unsafe one, because `seq` comes from
// nextval, which is assigned BEFORE commit. Two concurrent writers can commit out
// of order, so a poller that advances its cursor past the higher seq never sees
// the lower one. By id there is no watermark to skip past.
//
// `?since=` is kept for the cursor scan (the nudge pickup, which reads from 0 and
// filters by run) and for daemons older than this route.
export const GET: RequestHandler = async ({ params, request, url }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const id = url.searchParams.get('id');
		const since = Number(url.searchParams.get('since') ?? '0') || 0;
		const db = dojoDb();
		// membership_id is filtered in BOTH shapes: an id alone must never be
		// enough to read another tenant's gate.
		const base = db.from('relay_inbox').select(COLS).eq('membership_id', caller.membershipId);
		const { data, error } = id
			? await base.eq('id', id)
			: await base.gt('seq', since).order('seq', { ascending: true });
		if (error) return apiError(500, error.message);
		const rows = data ?? [];
		// The wire contract (dojo_protocol::relay::RelayInboxItem, decoded by the
		// daemon's poll_inbox) is keyed by run_id — NOT the DB's session_id. Resolve
		// it so the daemon can decode the response (run_id is a required field); the
		// raw session_id never crosses the wire. Mirrors the phone-plane gates route.
		const sessionIds = [...new Set(rows.map((r) => r.session_id as string))];
		const runById = new Map<string, string>();
		if (sessionIds.length) {
			const { data: sess, error: sErr } = await db
				.from('relay_sessions')
				.select('id, run_id')
				.in('id', sessionIds);
			if (sErr) return apiError(500, sErr.message);
			for (const s of sess ?? []) runById.set(s.id as string, s.run_id as string);
		}
		const items = rows.map(({ session_id, ...rest }) => ({
			...rest,
			run_id: runById.get(session_id as string) ?? (session_id as string)
		}));
		const cursor = rows.reduce((m, r) => Math.max(m, Number(r.seq)), since);
		return Response.json({ items, cursor });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
