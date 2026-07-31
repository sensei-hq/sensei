// GET/POST /v1/t/{origin}/{org}/relay/session — a run's filtered status snapshot.
// POST = the daemon publishing status (API-key plane, upsert by run_id); GET = the
// phone/console reading it (Supabase-JWT plane). See docs/plan/relay-engine.md §6.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, resolveTenantAccess, membershipIdsForTenant, apiError, ACCESS } from '$lib/server/dojo-auth';
import { sendRelayPushFromEnv } from '$lib/server/relay-push-env';
import { resolveProjectNamespaceId, openOrRefreshSeat } from '$lib/server/billing-data';
import { upsertProjectFromRun } from '$lib/server/projects-data';

const COLS =
	'id, run_id, title, goal, status, progress_done, progress_total, current_phase, current_feature, last_event_at, paused_until, pause_reason, heartbeat_at, started_at, completed_at';

const str = (v: unknown): string | null => (typeof v === 'string' ? v : null);
const int = (v: unknown): number => (typeof v === 'number' && Number.isFinite(v) ? v : 0);

// Daemon → publish the run's filtered status (upsert on tenant_id+run_id).
export const POST: RequestHandler = async ({ params, request, platform }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const runId = str(body.run_id);
		if (!runId) return apiError(400, 'run_id is required');
		const db = dojoDb();
		const status = str(body.status) ?? 'running';
		const title = str(body.title) ?? '';

		// Read the prior status so a crash push fires only on the TRANSITION into
		// crashed — not on every subsequent heartbeat upsert while already crashed.
		const { data: prior, error: pErr } = await db
			.from('relay_sessions')
			.select('status')
			.eq('membership_id', caller.membershipId)
			.eq('run_id', runId)
			.maybeSingle();
		if (pErr) return apiError(500, pErr.message);

		const { data, error } = await db
			.from('relay_sessions')
			.upsert(
				{
					membership_id: caller.membershipId,
					run_id: runId,
					title,
					goal: str(body.goal),
					status,
					progress_done: int(body.progress_done),
					progress_total: int(body.progress_total),
					current_phase: str(body.current_phase),
					current_feature: str(body.current_feature),
					last_event_at: str(body.last_event_at),
					paused_until: str(body.paused_until),
					pause_reason: str(body.pause_reason),
					// Liveness ping (activity.runs.heartbeat_at) — the phone badges
					// staleness from this instant's age (no update in ~5 min = stale).
					heartbeat_at: str(body.heartbeat_at),
					updated_at: new Date().toISOString()
				},
				{ onConflict: 'membership_id,run_id' }
			)
			.select('id')
			.single();
		if (error) return apiError(500, error.message);

		// P4.4 — push on a crash. A crash is a run STATUS (not an inbox kind), so it's
		// pushed here, gated by the `crashed` opt-in. Only on the transition into
		// crashed (prior !== crashed) to avoid re-pushing on heartbeat upserts.
		// Fire-and-forget via waitUntil; fail-open (the send swallows its own errors).
		if (status === 'crashed' && (prior?.status as string | undefined) !== 'crashed') {
			const push = sendRelayPushFromEnv({
				userId: caller.userId,
				tenantId: caller.tenantId,
				runId,
				runTitle: title,
				signal: { type: 'crashed' }
			});
			if (platform?.context?.waitUntil) {
				platform.context.waitUntil(push);
			} else {
				push.catch(() => {});
			}
		}

		// P4 attribution — when the daemon includes the run's project namespace
		// slug, open/refresh this user's seat: proof they are ACTIVELY using sensei
		// on that project (only such users are billed). Best-effort + fail-open —
		// billing must never break relay federation, and is throttle-free at relay
		// heartbeat volumes (an idempotent read+touch).
		const projectSlug = str(body.project_slug);
		if (projectSlug) {
			const seatWork = (async () => {
				const nsId = await resolveProjectNamespaceId(db, projectSlug);
				if (nsId) {
					await openOrRefreshSeat(db, {
						tenantId: caller.tenantId,
						userId: caller.userId,
						namespaceId: nsId,
						nowIso: new Date().toISOString()
					});
				}
			})().catch(() => {});
			if (platform?.context?.waitUntil) platform.context.waitUntil(seatWork);
		}

		// The run's project → dojo.projects (the display row for the projects
		// screens). The owner is the AUTHENTICATED caller (never the payload); a
		// personal project is stored tenant-less. Best-effort + fail-open, like the
		// seat — a projects-upsert failure must never break relay federation.
		const project = body.project as Record<string, unknown> | null | undefined;
		if (project && str(project.slug) && str(project.name)) {
			const projWork = upsertProjectFromRun(
				db,
				{ userId: caller.userId, tenantId: caller.tenantId },
				{
					slug: str(project.slug) as string,
					name: str(project.name) as string,
					classification: str(project.classification) ?? 'personal',
					phase: str(project.phase) ?? 'watch'
				}
			).catch(() => {});
			if (platform?.context?.waitUntil) platform.context.waitUntil(projWork);
		}

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
		const db = dojoDb();
		const membershipIds = await membershipIdsForTenant(db, tenantId);
		let q = db
			.from('relay_sessions')
			.select(COLS)
			.in('membership_id', membershipIds)
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
