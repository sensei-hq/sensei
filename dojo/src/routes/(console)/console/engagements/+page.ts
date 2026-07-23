import type { PageLoad } from './$types';
import { DojoApiError, listEngagements, type Engagement } from '$lib/client-data';
import { guardTenantScope } from '$lib/org-guard';

// Engagements: the tenant's client engagements (list_engagements, LEAD floor). A
// membership-less user (`tenantKey === null`, DJ1) skips the fetch via the shared
// guard → "join or create a Dōjō" empty state. For a member it degrades to an
// empty list + a surfaced error so the screen renders under SSR/prerender and
// without a live dojo service / lead JWT; a 403 from a non-lead surfaces the same
// way (the error banner).
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let engagements: Engagement[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<Engagement[]>(tenantKey, [], (tk) =>
			listEngagements(tk, { fetch, accessToken })
		);
		engagements = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { engagements, error, noMembership };
};
