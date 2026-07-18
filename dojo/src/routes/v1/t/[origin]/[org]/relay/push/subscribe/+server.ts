// POST /v1/t/{origin}/{org}/relay/push/subscribe — the phone/console registers a
// Web Push subscription (relay P4.3, client half). Supabase-JWT plane
// (resolveTenantAccess), like the other relay routes. Upserts a
// dojo.push_subscriptions row keyed by (user_id, web_push.endpoint) so a device
// re-subscribing (permission re-grant, key rotation) refreshes in place rather
// than piling up rows. Service-role client (dojoDb) — bypasses RLS; the route
// enforces auth. The actual send-on-gate is P4.4.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { userId, membershipId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.member
		);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;

		// Defensive input validation — a subscription without an endpoint or the
		// p256dh/auth keys is unusable for a VAPID send, so reject it up front.
		const endpoint = typeof body.endpoint === 'string' ? body.endpoint.trim() : '';
		if (!endpoint) return apiError(400, 'endpoint is required');
		// The Worker later fetch()es this endpoint to send the push (P4.4), so it's
		// an SSRF surface. Every real Web Push endpoint is https — require it, which
		// blocks http/internal-host targets. (Self-scoped even without this: a user
		// can only push against their own subscriptions. A push-service host
		// allowlist is a tracked follow-up — see decisions.md — deferred because an
		// incomplete list would reject legitimate browser endpoints.)
		let endpointUrl: URL;
		try {
			endpointUrl = new URL(endpoint);
		} catch {
			return apiError(400, 'endpoint must be a valid URL');
		}
		if (endpointUrl.protocol !== 'https:') return apiError(400, 'endpoint must be https');
		const keys = (body.keys ?? {}) as Record<string, unknown>;
		const p256dh = typeof keys.p256dh === 'string' ? keys.p256dh : '';
		const auth = typeof keys.auth === 'string' ? keys.auth : '';
		if (!p256dh || !auth) return apiError(400, 'keys.p256dh and keys.auth are required');
		// Only web is handled here; native tokens use a separate registration path (P5+).
		const platform = body.platform === 'web' ? 'web' : 'web';

		const db = dojoDb();
		const nowIso = new Date().toISOString();
		const webPush = { endpoint, p256dh, auth };

		// Keyed by (user_id, endpoint): endpoint lives inside the web_push jsonb, so
		// match it with a jsonb `->>` filter rather than a column upsert.
		const { data: existing, error: findErr } = await db
			.from('push_subscriptions')
			.select('id')
			.eq('user_id', userId)
			.eq('web_push->>endpoint', endpoint)
			.maybeSingle();
		if (findErr) return apiError(500, findErr.message);

		if (existing) {
			const { data, error } = await db
				.from('push_subscriptions')
				.update({
					web_push: webPush,
					platform,
					membership_id: membershipId ?? null,
					enabled: true,
					last_seen: nowIso,
					updated_at: nowIso
				})
				.eq('id', existing.id)
				.select('id')
				.single();
			if (error) return apiError(500, error.message);
			return Response.json({ id: data.id });
		}

		const { data, error } = await db
			.from('push_subscriptions')
			.insert({
				user_id: userId,
				membership_id: membershipId ?? null,
				platform,
				web_push: webPush,
				enabled: true,
				last_seen: nowIso
			})
			.select('id')
			.single();
		if (error) return apiError(500, error.message);
		return Response.json({ id: data.id });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
