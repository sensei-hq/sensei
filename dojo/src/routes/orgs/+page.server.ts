// Real org-picker data: the signed-in Supabase user + the Dōjōs they belong to
// (`dojo.memberships` → `tenants`). Replaces the mock user/orgs the page shipped
// with. The mapping lives in the shared server helper so this page and the
// console chrome ((console)/+layout.server.ts) can't drift.
import type { PageServerLoad } from './$types';
import { listUserOrgs, userProfile, sessionUser } from '$lib/server/dojo-orgs';
import type { DojoOrg } from '$lib/dojo-data';

export const load: PageServerLoad = async ({ locals }) => {
	const su = sessionUser(locals);
	const user = userProfile(su);
	const orgs: DojoOrg[] = su?.id ? await listUserOrgs(su.id) : [];
	return { user, orgs };
};
