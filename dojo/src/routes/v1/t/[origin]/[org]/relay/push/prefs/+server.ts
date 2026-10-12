// POST /v1/t/{origin}/{org}/relay/push/prefs — the human's per-event push opt-ins
// (relay P4.3). Supabase-JWT plane (resolveTenantAccess). Upserts the caller's
// dojo.notification_prefs row (user_id is the PK) — merging the posted `events`
// onto whatever is already stored so enabling "stalls" doesn't wipe an existing
// "approvals" opt-in. quiet_hours / muted_tenants are left untouched here (a
// later screen owns those). Written as the signed-in person: RLS keeps the row
// their own (#236). The send that reads these runs as the system (P4.4).
import type { RequestHandler } from './$types';
import { asOwner } from '$lib/server/relay-owner';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

/** The push event opt-in keys this route accepts (mirrors notification_prefs.events). */
const EVENT_KEYS = ['approvals', 'decisions', 'stalls', 'crashed'] as const;

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.member);
		const { db } = asOwner(caller);
		const { userId } = caller;
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const incoming = (body.events ?? {}) as Record<string, unknown>;

		// Whitelist + coerce to booleans — never trust arbitrary keys into the jsonb.
		const events: Record<string, boolean> = {};
		for (const k of EVENT_KEYS) {
			if (k in incoming) events[k] = Boolean(incoming[k]);
		}
		if (Object.keys(events).length === 0) return apiError(400, 'events is required');

		const nowIso = new Date().toISOString();

		// Merge onto the existing prefs (PK = user_id) so we don't clobber other opt-ins.
		const { data: existing, error: findErr } = await db
			.from('notification_prefs')
			.select('events')
			.eq('user_id', userId)
			.maybeSingle();
		if (findErr) return apiError(500, findErr.message);

		const merged = { ...((existing?.events as Record<string, unknown>) ?? {}), ...events };
		const { data, error } = await db
			.from('notification_prefs')
			.upsert(
				{ user_id: userId, events: merged, updated_at: nowIso },
				{ onConflict: 'user_id' }
			)
			.select('user_id')
			.single();
		if (error) return apiError(500, error.message);
		return Response.json({ user_id: data.user_id, events: merged });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
