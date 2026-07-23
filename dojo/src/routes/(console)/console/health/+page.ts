import type { PageLoad } from './$types';
import { DojoApiError, getHealth, type HealthRollup } from '$lib/admin-data';
import { guardTenantScope } from '$lib/org-guard';

// Health: the connections / queue-depth / publish-rate / error-rate rollups
// (admin_health, ADMIN floor). A membership-less user (`tenantKey === null`, DJ1)
// skips the fetch via the shared guard → "join or create a Dōjō" empty state. For
// a member it degrades to a null rollup + a surfaced error so the screen renders
// under SSR/prerender and without a live service / admin JWT.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let health: HealthRollup | null = null;
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<HealthRollup | null>(tenantKey, null, (tk) =>
			getHealth(tk, { fetch, accessToken })
		);
		health = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { health, error, noMembership };
};
