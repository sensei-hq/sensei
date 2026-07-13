import type { PageLoad } from './$types';
import { listTriage, TriageApiError, type TriageRow } from '$lib/triage-data';

// The triage queue: the tenant's open rows (list_triage). Degrades to an empty
// queue + a surfaced error so the screen renders under SSR/prerender and without
// a live dojo service (Jerry's live-verify step).
export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey } = await parent();
	let queue: TriageRow[] = [];
	let error: string | null = null;
	try {
		queue = await listTriage(tenantKey, { fetch });
	} catch (e) {
		error = e instanceof TriageApiError ? e.message : 'could not reach the dojo service';
	}
	return { queue, error };
};
