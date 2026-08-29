// Real org-picker data: the signed-in Supabase user + the Dōjōs they belong to
// (`dojo.memberships` → `tenants`). Replaces the mock user/orgs the page shipped
// with. The mapping lives in the shared server helper so this page and the
// console chrome ((console)/+layout.server.ts) can't drift.
import type { PageServerLoad } from './$types';
import {
	listUserOrgs,
	principalIdForSession,
	userProfile,
	sessionUser
} from '$lib/server/dojo-orgs';
import type { DojoOrg } from '$lib/dojo-data';

export const load: PageServerLoad = async ({ locals }) => {
	const su = sessionUser(locals);
	const user = userProfile(su);
	// The PRINCIPAL id, not `su.id` (the Supabase auth id) — see
	// `principalIdForSession`. Querying memberships with the auth id matched
	// nothing, so the picker showed no orgs to members who had them.
	const principalId = await principalIdForSession(locals);
	const orgs: DojoOrg[] = principalId ? await listUserOrgs(principalId) : [];
	return { user, orgs };
};
