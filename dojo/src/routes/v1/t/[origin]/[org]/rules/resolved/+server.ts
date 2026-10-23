// GET /v1/t/{origin}/{org}/rules/resolved?ns=organization:acme,repository:github.com/acme/web
//
// The full-resolve leg for rule packs (Option A). The daemon can't query the
// Dōjō DB (Fork 1), so at get_rules time it asks the Worker for the rules of
// every pack ADOPTED at its folder's namespaces (passed as stable scope:slug
// pairs). A member gets those adopted at the namespace their tenant owns, with
// each adoption's never-weaken tier override (`dojo.tenant_pack_rules`, decision
// 2026-10-12) — folded by the daemon into its resolved ruleset → the
// SessionStart/PreCompact push.
//
// As the person (#236 step 4).
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantCaller, apiError, ACCESS } from '$lib/server/dojo-auth';
import { parseNamespacePairs, resolveTenantPackRules, RulesError } from '$lib/server/rules-data';

export const GET: RequestHandler = async ({ params, request, url }) => {
	try {
		const caller = await resolveTenantCaller(params.origin, params.org, request, ACCESS.member);
		const pairs = parseNamespacePairs(url.searchParams.get('ns'));
		const rules = await resolveTenantPackRules(callerDb(caller), caller.tenantId, pairs);
		return Response.json({ rules });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof RulesError) return apiError(e.status, e.message);
		throw e;
	}
};
