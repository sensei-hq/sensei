import type { PageLoad } from './$types';
import { DojoApiError, listIdentities, type Identity } from '$lib/admin-data';
import { guardTenantScope } from '$lib/org-guard';

// Identities: the tenant's SSO / GitHub / device-code identity mappings
// (list_identities, ADMIN floor). A membership-less user (`tenantKey === null`,
// DJ1) skips the fetch via the shared guard → "join or create a Dōjō" empty state.
// For a member it degrades to an empty list + a surfaced error so the screen
// renders under SSR/prerender and without a live service / admin JWT.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let identities: Identity[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<Identity[]>(tenantKey, [], (tk) =>
			listIdentities(tk, { fetch, accessToken })
		);
		identities = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { identities, error, noMembership };
};
