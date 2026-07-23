import type { PageLoad } from './$types';
import { DojoApiError, listPolicies, type Policy } from '$lib/admin-data';
import { guardTenantScope } from '$lib/org-guard';

// Policies: the tenant's per-scope policy grid (list_policies, ADMIN floor). A
// membership-less user (`tenantKey === null`, DJ1) skips the fetch via the shared
// guard → "join or create a Dōjō" empty state. For a member it degrades to an
// empty grid + a surfaced error so the screen renders under SSR/prerender and
// without a live service / admin JWT.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let policies: Policy[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<Policy[]>(tenantKey, [], (tk) =>
			listPolicies(tk, { fetch, accessToken })
		);
		policies = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { policies, error, noMembership };
};
