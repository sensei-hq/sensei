import type { PageLoad } from './$types';
import { DojoApiError, listIncidents, type Incident } from '$lib/client-data';
import { guardTenantScope } from '$lib/org-guard';

// Incidents: the tenant's confidentiality incidents (list_incidents, LEAD floor),
// worst-severity first, plus the open-count done-gate (`resolved_at is null`). A
// membership-less user (`tenantKey === null`, DJ1) skips the fetch via the shared
// guard → "join or create a Dōjō" empty state. For a member it degrades to an
// empty list + a surfaced error so the screen renders under SSR/prerender and
// without a live dojo service / lead JWT; a 403 from a non-lead surfaces the same
// way.
type IncidentList = { incidents: Incident[]; open_count: number };
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let incidents: Incident[] = [];
	let openCount = 0;
	let error: string | null = null;
	let noMembership = false;
	try {
		const guarded = await guardTenantScope<IncidentList>(
			tenantKey,
			{ incidents: [], open_count: 0 },
			(tk) => listIncidents(tk, { fetch, accessToken })
		);
		incidents = guarded.value.incidents;
		openCount = guarded.value.open_count;
		noMembership = guarded.noMembership;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { incidents, openCount, error, noMembership };
};
