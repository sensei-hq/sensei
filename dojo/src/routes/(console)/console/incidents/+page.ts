import type { PageLoad } from './$types';
import { DojoApiError, listIncidents, type Incident } from '$lib/client-data';

// Incidents: the tenant's confidentiality incidents (list_incidents, LEAD floor),
// worst-severity first, plus the open-count done-gate (`resolved_at is null`).
// Degrades to an empty list + a surfaced error so the screen renders under
// SSR/prerender and without a live dojo service / lead JWT (Jerry's
// live-verify step); a 403 from a non-lead surfaces the same way.
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let incidents: Incident[] = [];
	let openCount = 0;
	let error: string | null = null;
	try {
		const list = await listIncidents(tenantKey, { fetch, accessToken });
		incidents = list.incidents;
		openCount = list.open_count;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return { incidents, openCount, error };
};
