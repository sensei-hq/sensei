import type { PageLoad } from './$types';
import { DojoApiError, getHealth, type HealthRollup } from '$lib/admin-data';

// Health: the connections / queue-depth / publish-rate / error-rate rollups
// (admin_health, ADMIN floor). Degrades to a null rollup + a surfaced error so
// the screen renders under SSR/prerender and without a live service / admin JWT.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let health: HealthRollup | null = null;
	let error: string | null = null;
	try {
		health = await getHealth(tenantKey, { fetch, accessToken });
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { health, error };
};
