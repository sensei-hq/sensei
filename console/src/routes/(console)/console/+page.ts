import type { PageLoad } from './$types';
import { listTriage, TriageApiError, type TriageRow } from '$lib/triage-data';

// Overview loads the tenant's open triage queue (the one live signal on this
// screen — the metric cards / activity / published-adoption panels are the
// design's static demo content until their endpoints land). Failures degrade to
// an empty queue + a surfaced error so the dashboard still renders under
// SSR/prerender and without a running dojo service (Jerry's live-verify step).
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
