import type { PageLoad } from './$types';
import { listTriage, TriageApiError, type TriageRow } from '$lib/triage-data';
import { guardTenantScope } from '$lib/org-guard';

// The triage queue: the tenant's open rows (list_triage). A membership-less user
// (`tenantKey === null`, DJ1) skips the fetch via the shared guard → "join or
// create a Dōjō" empty state. For a member it degrades to an empty queue + a
// surfaced error so the screen renders under SSR/prerender and without a live
// dojo service.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let queue: TriageRow[] = [];
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<TriageRow[]>(tenantKey, [], (tk) =>
			listTriage(tk, { fetch, accessToken })
		);
		queue = guarded.value;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof TriageApiError ? e.message : 'could not reach the dojo service';
	}
	return { queue, error, noMembership };
};
