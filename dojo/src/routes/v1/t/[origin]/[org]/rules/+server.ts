// POST/GET /v1/t/{origin}/{org}/rules — the tenant's shared-rules federation
// (Plane A). The in-Worker port of dojo-mind's `publish_rule` / `pull_rules`
// (api.rs), reusing the device-token auth plane (`resolveApiKeyAccess`): the
// daemon's `KnowledgeSource.url` points at this tenant path and presents its
// Keychain device token as `Authorization: Bearer …`. The wire shape matches
// `dojo_protocol` (PublishResponse / PullResponse) so senseid's federation
// client retargets with only a URL/auth swap. See
// docs/plan/2026-07-23-retire-dojo-mind.md (D1 option a).
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import {
	parsePublishedRule,
	publishRule,
	pullRules,
	recordRulesAudit,
	RulesError
} from '$lib/server/rules-data';

// Daemon → publish (contribute) a rule under this tenant. Contributor+ (mirrors
// dojo-mind's `Role::Publisher` floor). Attribution is server-controlled.
export const POST: RequestHandler = async ({ params, request }) => {
	try {
		const caller = await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.contributor);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const rule = parsePublishedRule(body);
		if (!rule) return apiError(400, 'a required rule field is missing');

		const db = dojoDb();
		// Attribution comes from the authenticated caller, not the body — a
		// publisher can't spoof who published a rule.
		const resp = await publishRule(db, rule, caller.userId);
		// Audit-write failures don't fail the (already committed) publish — but
		// they are logged, never silently dropped (mirrors dojo-mind).
		await recordRulesAudit(db, caller.tenantId, 'publish', resp.id, caller.userId, {
			version: resp.version,
			seq: resp.seq
		});
		return Response.json(resp);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof RulesError) return apiError(e.status, e.message);
		throw e;
	}
};

// Daemon → pull rule deltas since a cursor. Member+ (any authenticated device
// token for this tenant may pull, mirroring dojo-mind's `pull_rules`).
export const GET: RequestHandler = async ({ params, request, url }) => {
	try {
		await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.member);
		const sinceRaw = url.searchParams.get('since');
		const parsed = sinceRaw == null ? 0 : Number.parseInt(sinceRaw, 10);
		const since = Number.isFinite(parsed) ? parsed : 0;
		const page = await pullRules(dojoDb(), since);
		return Response.json(page);
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof RulesError) return apiError(e.status, e.message);
		throw e;
	}
};
