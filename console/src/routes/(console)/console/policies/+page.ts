import type { PageLoad } from './$types';
import { DojoApiError, listPolicies, type Policy } from '$lib/admin-data';

// Policies: the tenant's per-scope policy grid (list_policies, ADMIN floor).
// Degrades to an empty grid + a surfaced error so the screen renders under
// SSR/prerender and without a live service / admin JWT (Jerry's live-verify step).
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let policies: Policy[] = [];
	let error: string | null = null;
	try {
		policies = await listPolicies(tenantKey, { fetch, accessToken });
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { policies, error };
};
