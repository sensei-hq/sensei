import type { PageLoad } from './$types';
import { DojoApiError, listMembers, type Membership } from '$lib/admin-data';
import { guardTenantScope } from '$lib/org-guard';

// Members & roles: the tenant's memberships (list_members, ADMIN floor). A
// membership-less user (`tenantKey === null`, DJ1) skips the fetch via the shared
// guard and the screen shows the "join or create a Dōjō" empty state instead of a
// fabricated tenant's rows. For a member it degrades to an empty list + a surfaced
// error so the screen renders under SSR/prerender and without a live dojo service /
// admin JWT; a 403 from a non-admin surfaces the same way (the error banner).
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let members: Membership[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<Membership[]>(tenantKey, [], (tk) =>
			listMembers(tk, { fetch, accessToken })
		);
		members = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { members, error, noMembership };
};
