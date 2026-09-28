// POST /v1/you/provision — establish everything the caller's forge proves.
//
// The web sign-in caller of §II.7. kavach owns the OAuth callback
// (`routes.session` is served inside `kavach.handle`), so there is no callback
// file to hook — the client calls this once immediately after signing in
// (spec §VIII.3). An explicit endpoint on the plane that already exists, which
// is also testable without a browser and does not run on every navigation the
// way a root layout load would.
//
// Idempotent, so calling it again is free and the console may re-run it.
import type { RequestHandler } from './$types';
import { resolveCaller, apiError } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { provisionWithToken } from '$lib/server/provisioning';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId, email, db } = await resolveCaller(request, locals);

		// Two sources for the forge token, because they have different lifetimes.
		// The web session's `provider_token` exists only immediately after the
		// OAuth exchange; the daemon's is persisted to the OS keychain and is
		// therefore the more durable of the two (§IV.8), so it sends it here.
		//
		// The dōjō reads the org list ITSELF from this token. It never accepts a
		// list of orgs from a caller — that would be the service trusting a client
		// about its own entitlements, the one thing §II.5 says it must not do.
		//
		// Accepting a token in the body is safe against the obvious attack: a
		// caller presenting someone ELSE'S forge token cannot annex their orgs,
		// because `ensureIdentity` refuses to re-point a forge account that already
		// belongs to another principal (409). And anyone holding that token could
		// simply sign in as its owner anyway, so it grants no new capability.
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const fromBody =
			typeof body.provider_token === 'string' && body.provider_token.trim()
				? body.provider_token.trim()
				: null;
		const fromSession =
			(locals as { session?: { provider_token?: string | null } }).session?.provider_token ?? null;

		const result = await provisionWithToken(db, userId, fromBody ?? fromSession, { email });
		return Response.json(result);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
