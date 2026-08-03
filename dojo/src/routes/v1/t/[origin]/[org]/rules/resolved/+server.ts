// GET /v1/t/{origin}/{org}/rules/resolved?ns=organization:acme,stack:react
//
// The full-resolve leg for rule packs (Option A). The daemon can't query the
// Dōjō DB (Fork 1), so at get_rules time it asks the Worker for the rules of
// every pack ADOPTED at its repo's namespaces (passed as stable scope:slug
// pairs). The Worker unions them with the adoption's tier override applied
// (never-weaken) and returns them; the daemon folds them into its resolved
// ruleset → the SessionStart/PreCompact push. Member+ (any authenticated device
// token for this tenant), mirroring the sibling rules pull.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveApiKeyAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import {
	parseNamespacePairs,
	resolveNamespaces,
	resolveAdoptedPackRules,
	RulesError
} from '$lib/server/rules-data';

export const GET: RequestHandler = async ({ params, request, url }) => {
	try {
		await resolveApiKeyAccess(params.origin, params.org, request, ACCESS.member);
		const db = dojoDb();
		const namespaces = await resolveNamespaces(db, parseNamespacePairs(url.searchParams.get('ns')));
		const rules = await resolveAdoptedPackRules(db, namespaces);
		return Response.json({ rules });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof RulesError) return apiError(e.status, e.message);
		throw e;
	}
};
