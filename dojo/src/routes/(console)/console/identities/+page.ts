import type { PageLoad } from './$types';
import { DojoApiError, listIdentities, type Identity } from '$lib/admin-data';

// Identities: the tenant's SSO / GitHub / device-code identity mappings
// (list_identities, ADMIN floor). Degrades to an empty list + a surfaced error so
// the screen renders under SSR/prerender and without a live service / admin JWT.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let identities: Identity[] = [];
	let error: string | null = null;
	try {
		identities = await listIdentities(tenantKey, { fetch, accessToken });
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { identities, error };
};
